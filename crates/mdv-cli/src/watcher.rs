use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};

#[derive(Debug)]
pub enum WatchMessage {
    ExternalUpdate(String),
    Error(String),
}

pub fn start(path: &Path) -> notify::Result<(RecommendedWatcher, Receiver<WatchMessage>)> {
    start_with_factory(path, |mut handler| {
        recommended_watcher(move |result: notify::Result<Event>| handler.handle_event(result))
    })
}

fn start_with_factory<F>(
    path: &Path,
    make_watcher: F,
) -> notify::Result<(RecommendedWatcher, Receiver<WatchMessage>)>
where
    F: FnOnce(Box<dyn notify::EventHandler>) -> notify::Result<RecommendedWatcher>,
{
    let watched_path = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let handler: Box<dyn notify::EventHandler> = Box::new(move |result: notify::Result<Event>| {
        handle_notify_result(result, &watched_path, &tx)
    });

    let mut watcher = make_watcher(handler)?;

    watcher.watch(path, RecursiveMode::NonRecursive)?;
    Ok((watcher, rx))
}

fn handle_notify_result(
    result: notify::Result<Event>,
    watched_path: &Path,
    tx: &Sender<WatchMessage>,
) {
    match result {
        Ok(event) => {
            if !is_relevant(&event.kind) {
                return;
            }

            if !event
                .paths
                .iter()
                .any(|event_path| same_file(event_path, watched_path))
            {
                return;
            }

            match fs::read_to_string(watched_path) {
                Ok(content) => {
                    let _ = tx.send(WatchMessage::ExternalUpdate(content));
                }
                Err(err) => {
                    let _ = tx.send(WatchMessage::Error(err.to_string()));
                }
            }
        }
        Err(err) => {
            let _ = tx.send(WatchMessage::Error(err.to_string()));
        }
    }
}

fn is_relevant(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    )
}

fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }

    let ca = canonical(a);
    let cb = canonical(b);
    ca == cb && ca.is_some()
}

fn canonical(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use notify::Event;
    use notify::EventKind;
    use notify::event::{CreateKind, ModifyKind};

    use super::{WatchMessage, handle_notify_result, is_relevant, same_file};

    #[test]
    fn relevant_event_filter_works() {
        assert!(is_relevant(&EventKind::Modify(ModifyKind::Any)));
        assert!(is_relevant(&EventKind::Create(CreateKind::Any)));
        assert!(!is_relevant(&EventKind::Access(
            notify::event::AccessKind::Any
        )));
    }

    #[test]
    fn same_file_handles_equal_and_canonical_paths() {
        let dir = std::env::temp_dir();
        let path = dir.join("mdv-watch-same-file-test.md");
        let canonical = std::fs::canonicalize(&dir).expect("canonical temp dir");
        let alternate = canonical.join("mdv-watch-same-file-test.md");

        std::fs::write(&path, "x").expect("seed");
        assert!(same_file(&path, &path));
        assert!(same_file(&path, &alternate));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn handle_notify_result_sends_update() {
        let dir = std::env::temp_dir();
        let path = dir.join("mdv-watch-update-test.md");
        std::fs::write(&path, "new").expect("seed");
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };
        let (tx, rx) = mpsc::channel();

        handle_notify_result(Ok(event), &path, &tx);

        assert!(matches!(
            rx.recv().expect("msg"),
            WatchMessage::ExternalUpdate(text) if text == "new"
        ));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn handle_notify_result_sends_error_on_notify_error() {
        let path = std::env::temp_dir().join("mdv-watch-error-test.md");
        let (tx, rx) = mpsc::channel();
        handle_notify_result(Err(notify::Error::generic("watch failed")), &path, &tx);

        assert!(matches!(
            rx.recv().expect("msg"),
            WatchMessage::Error(err) if err.contains("watch failed")
        ));
    }

    #[test]
    fn handle_notify_result_ignores_irrelevant_or_other_path() {
        let dir = std::env::temp_dir();
        let path = dir.join("mdv-watch-ignore-test.md");
        std::fs::write(&path, "x").expect("seed");
        let other = dir.join("mdv-watch-ignore-test-other.md");
        std::fs::write(&other, "y").expect("seed other");

        let irrelevant = Event {
            kind: EventKind::Access(notify::event::AccessKind::Any),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };
        let mismatched = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![other.clone()],
            attrs: Default::default(),
        };

        let (tx, rx) = mpsc::channel();
        handle_notify_result(Ok(irrelevant), &path, &tx);
        handle_notify_result(Ok(mismatched), &path, &tx);
        assert!(rx.try_recv().is_err());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&other);
    }

    #[test]
    fn handle_notify_result_sends_read_error() {
        let dir = std::env::temp_dir();
        let path = dir.join("mdv-watch-read-error-test.md");
        std::fs::write(&path, "x").expect("seed");
        std::fs::remove_file(&path).expect("remove");
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };
        let (tx, rx) = mpsc::channel();
        handle_notify_result(Ok(event), &path, &tx);
        let msg = rx.recv().expect("msg");
        let debug = format!("{msg:?}");
        assert!(debug.starts_with("Error(\""), "debug: {debug}");
    }

    #[test]
    fn start_initializes_watcher() {
        let dir = std::env::temp_dir();
        let path = dir.join("mdv-watch-start-test.md");
        std::fs::write(&path, "x").expect("seed");
        let started = super::start(&path);
        assert!(started.is_ok());
        let (_watcher, _rx) = started.expect("watcher");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn start_returns_error_for_missing_path() {
        let path = std::env::temp_dir().join("mdv-watch-missing-start-test.md");
        let started = super::start(&path);
        assert!(started.is_err());
    }

    #[test]
    fn start_propagates_watcher_constructor_error() {
        let path = std::env::temp_dir().join("mdv-watch-factory-error-test.md");
        std::fs::write(&path, "x").expect("seed");
        let started = super::start_with_factory(&path, |_handler| {
            Err::<notify::RecommendedWatcher, _>(notify::Error::generic("factory failed"))
        });
        assert!(matches!(started, Err(err) if err.to_string().contains("factory failed")));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn start_callback_emits_update_after_write() {
        let dir = std::env::temp_dir();
        let path = dir.join("mdv-watch-callback-test.md");
        std::fs::write(&path, "b").expect("seed");
        let event_path = path.clone();
        let (_watcher, rx) = super::start_with_factory(&path, move |mut handler| {
            let event = Event {
                kind: EventKind::Modify(ModifyKind::Any),
                paths: vec![event_path.clone()],
                attrs: Default::default(),
            };
            handler.handle_event(Ok(event));
            notify::recommended_watcher(move |_result: notify::Result<Event>| {})
        })
        .expect("start");

        assert!(matches!(
            rx.recv_timeout(std::time::Duration::from_millis(250)).expect("msg"),
            WatchMessage::ExternalUpdate(text) if text == "b"
        ));

        let _ = std::fs::remove_file(&path);
    }
}

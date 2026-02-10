use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};

#[derive(Debug)]
pub enum WatchMessage {
    ExternalUpdate(String),
    Error(String),
}

pub fn start(path: &Path) -> notify::Result<(RecommendedWatcher, Receiver<WatchMessage>)> {
    let watched_path = path.to_path_buf();
    let (tx, rx) = mpsc::channel();

    let mut watcher = recommended_watcher(move |result: notify::Result<Event>| match result {
        Ok(event) => {
            if !is_relevant(&event.kind) {
                return;
            }

            if !event
                .paths
                .iter()
                .any(|event_path| same_file(event_path, &watched_path))
            {
                return;
            }

            match fs::read_to_string(&watched_path) {
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
    })?;

    watcher.watch(path, RecursiveMode::NonRecursive)?;
    Ok((watcher, rx))
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
    use notify::EventKind;
    use notify::event::{CreateKind, ModifyKind};

    use super::{is_relevant, same_file};

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
}

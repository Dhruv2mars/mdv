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
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{WatchMessage, start};

    #[test]
    fn watcher_emits_external_update() {
        let dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = dir.join(format!("mdv-watch-{stamp}.md"));

        std::fs::write(&path, "old").expect("seed file");
        let (_watcher, rx) = start(&path).expect("start watcher");
        std::fs::write(&path, "new").expect("write update");

        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            if std::time::Instant::now() > deadline {
                panic!("watcher timeout");
            }

            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(WatchMessage::ExternalUpdate(content)) => {
                    if content == "new" {
                        break;
                    }
                }
                Ok(WatchMessage::Error(err)) => panic!("watch error: {err}"),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(err) => panic!("channel error: {err}"),
            }
        }

        let _ = std::fs::remove_file(path);
    }
}

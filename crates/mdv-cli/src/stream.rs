use std::io::{self, BufRead};
use std::sync::mpsc::{self, Receiver, Sender};

const DEFAULT_STREAM_MAX_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub enum StreamMessage {
    Update { text: String, truncated: bool },
    End,
    Error(String),
}

#[cfg_attr(test, allow(dead_code))]
pub fn start() -> Receiver<StreamMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
        read_loop_with_limit(&mut reader, &tx, stream_max_bytes_from_env());
    });

    rx
}

#[cfg(test)]
fn read_loop(reader: &mut dyn BufRead, tx: &Sender<StreamMessage>) {
    read_loop_with_limit(reader, tx, stream_max_bytes_from_env());
}

fn read_loop_with_limit(reader: &mut dyn BufRead, tx: &Sender<StreamMessage>, max_bytes: usize) {
    let mut acc = String::new();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let _ = tx.send(StreamMessage::End);
                break;
            }
            Ok(_) => {
                acc.push_str(&line);
                let truncated = trim_head_to_max_bytes(&mut acc, max_bytes);
                let _ = tx.send(StreamMessage::Update {
                    text: acc.clone(),
                    truncated,
                });
            }
            Err(err) => {
                let _ = tx.send(StreamMessage::Error(err.to_string()));
                break;
            }
        }
    }
}

fn stream_max_bytes_from_env() -> usize {
    std::env::var("MDV_STREAM_MAX_BYTES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_STREAM_MAX_BYTES)
}

fn trim_head_to_max_bytes(text: &mut String, max_bytes: usize) -> bool {
    if text.len() <= max_bytes {
        return false;
    }
    if max_bytes == 0 {
        text.clear();
        return true;
    }

    let mut cut = text.len().saturating_sub(max_bytes);
    while cut < text.len() && !text.is_char_boundary(cut) {
        cut += 1;
    }
    text.drain(..cut);
    true
}

#[cfg(test)]
mod tests {
    use std::io::{self, BufReader, Cursor, Read};
    use std::sync::Mutex;
    use std::sync::mpsc;

    use super::{
        DEFAULT_STREAM_MAX_BYTES, StreamMessage, read_loop, read_loop_with_limit,
        stream_max_bytes_from_env,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct ErrorReader;
    impl Read for ErrorReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("boom"))
        }
    }

    #[test]
    fn stream_message_debug() {
        let msg = StreamMessage::Update {
            text: "abc".into(),
            truncated: false,
        };
        let s = format!("{msg:?}");
        assert!(s.contains("Update"));
    }

    #[test]
    fn read_loop_sends_updates_then_end() {
        let data = Cursor::new("a\nb\n");
        let mut reader = BufReader::new(data);
        let (tx, rx) = mpsc::channel();

        read_loop(&mut reader, &tx);

        let messages: Vec<_> = rx.try_iter().collect();
        assert_eq!(messages.len(), 3);
        assert_eq!(
            format!("{:?}", messages[0]),
            "Update { text: \"a\\n\", truncated: false }"
        );
        assert_eq!(
            format!("{:?}", messages[1]),
            "Update { text: \"a\\nb\\n\", truncated: false }"
        );
        assert_eq!(format!("{:?}", messages[2]), "End");
    }

    #[test]
    fn read_loop_sends_error_on_reader_failure() {
        let mut reader = BufReader::new(ErrorReader);
        let (tx, rx) = mpsc::channel();
        read_loop(&mut reader, &tx);

        let msg = rx.try_iter().next().expect("msg");
        assert!(matches!(msg, StreamMessage::Error(err) if err.contains("boom")));
    }

    #[test]
    fn read_loop_ignores_send_error_for_update_and_end_messages() {
        let data = Cursor::new("a\n");
        let mut reader = BufReader::new(data);
        let (tx, rx) = mpsc::channel::<StreamMessage>();
        drop(rx);
        read_loop(&mut reader, &tx);
    }

    #[test]
    fn read_loop_ignores_send_error_for_error_message() {
        let mut reader = BufReader::new(ErrorReader);
        let (tx, rx) = mpsc::channel::<StreamMessage>();
        drop(rx);
        read_loop(&mut reader, &tx);
    }

    #[test]
    fn read_loop_truncates_accumulator_to_max_bytes() {
        let data = Cursor::new("abcd\nxyz\n");
        let mut reader = BufReader::new(data);
        let (tx, rx) = mpsc::channel();

        read_loop_with_limit(&mut reader, &tx, 5);

        let messages: Vec<_> = rx.try_iter().collect();
        assert_eq!(messages.len(), 3);
        match &messages[1] {
            StreamMessage::Update { text, truncated } => {
                assert_eq!(text, "\nxyz\n");
                assert!(*truncated);
            }
            _ => panic!("expected update"),
        }
    }

    #[test]
    fn stream_max_bytes_from_env_parses_or_defaults() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        unsafe { std::env::remove_var("MDV_STREAM_MAX_BYTES") };
        assert_eq!(stream_max_bytes_from_env(), DEFAULT_STREAM_MAX_BYTES);

        unsafe { std::env::set_var("MDV_STREAM_MAX_BYTES", "1024") };
        assert_eq!(stream_max_bytes_from_env(), 1024);

        unsafe { std::env::set_var("MDV_STREAM_MAX_BYTES", "bad") };
        assert_eq!(stream_max_bytes_from_env(), DEFAULT_STREAM_MAX_BYTES);

        unsafe { std::env::set_var("MDV_STREAM_MAX_BYTES", "0") };
        assert_eq!(stream_max_bytes_from_env(), DEFAULT_STREAM_MAX_BYTES);

        unsafe { std::env::remove_var("MDV_STREAM_MAX_BYTES") };
    }
}

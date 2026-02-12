use std::io::{self, BufRead};
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug)]
pub enum StreamMessage {
    Update(String),
    End,
    Error(String),
}

#[cfg_attr(test, allow(dead_code))]
pub fn start() -> Receiver<StreamMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
        read_loop(&mut reader, &tx);
    });

    rx
}

fn read_loop(reader: &mut dyn BufRead, tx: &Sender<StreamMessage>) {
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
                let _ = tx.send(StreamMessage::Update(acc.clone()));
            }
            Err(err) => {
                let _ = tx.send(StreamMessage::Error(err.to_string()));
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, BufReader, Cursor, Read};
    use std::sync::mpsc;

    use super::{StreamMessage, read_loop};

    struct ErrorReader;
    impl Read for ErrorReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("boom"))
        }
    }

    #[test]
    fn stream_message_debug() {
        let msg = StreamMessage::Update("abc".into());
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
        assert_eq!(format!("{:?}", messages[0]), "Update(\"a\\n\")");
        assert_eq!(format!("{:?}", messages[1]), "Update(\"a\\nb\\n\")");
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
}

use std::io::{self, BufRead};
use std::sync::mpsc::{self, Receiver};

#[derive(Debug)]
pub enum StreamMessage {
    Update(String),
    End,
    Error(String),
}

pub fn start() -> Receiver<StreamMessage> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin.lock());
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
    });

    rx
}

#[cfg(test)]
mod tests {
    use super::StreamMessage;

    #[test]
    fn stream_message_debug() {
        let msg = StreamMessage::Update("abc".into());
        let s = format!("{msg:?}");
        assert!(s.contains("Update"));
    }
}

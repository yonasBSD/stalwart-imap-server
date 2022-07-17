use crate::core::{Command, StatusResponse};

use super::{search::Filter, ImapResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arguments {
    pub tag: String,
    pub filter: Filter,
    pub algorithm: Algorithm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Algorithm {
    OrderedSubject,
    References,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub is_uid: bool,
    pub threads: Vec<Vec<u32>>,
}

impl ImapResponse for Response {
    fn serialize(&self, tag: String) -> Vec<u8> {
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(b"* THREAD ");
        for thread in &self.threads {
            buf.push(b'(');
            for (pos, id) in thread.iter().enumerate() {
                if pos > 0 {
                    buf.push(b' ');
                }
                buf.extend_from_slice(id.to_string().as_bytes());
            }
            buf.push(b')');
        }
        buf.extend_from_slice(b"\r\n");
        StatusResponse::completed(Command::Thread(self.is_uid), tag).serialize(&mut buf);
        buf
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::ImapResponse;

    #[test]
    fn serialize_thread() {
        assert_eq!(
            String::from_utf8(
                super::Response {
                    is_uid: true,
                    threads: vec![vec![2, 10, 11], vec![49], vec![1, 3]],
                }
                .serialize("ABC".to_string())
            )
            .unwrap(),
            concat!(
                "* THREAD (2 10 11)(49)(1 3)\r\n",
                "ABC OK UID THREAD completed\r\n"
            )
        );
    }
}

//! POP3 wire format (RFC 1939): single-line replies `+OK …` /
//! `-ERR …`, and multi-line replies terminated by a lone `.` line
//! with dot-stuffed `..` escaping. `parse_line` reads a status line;
//! `parse_multiline` extracts the body of a multi-line reply and
//! un-stuffs doubled leading dots.
//!
//! ```
//! use izanagi_kit::pop3::{parse_line, Status};
//! assert_eq!(parse_line(b"+OK maildrop ready").unwrap(),
//!     (Status::Ok, "maildrop ready".to_string()));
//! let (st, _t) = parse_line(b"-ERR bad command").unwrap();
//! assert_eq!(st, Status::Err);
//! let body = izanagi_kit::pop3::parse_multiline(b"line1\r\n..dot\r\n.\r\n").unwrap();
//! assert_eq!(body, vec!["line1", ".dot"]);
//! ```

use std::string::String;
use std::vec::Vec;

/// POP3 status indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// `+OK` — success.
    Ok,
    /// `-ERR` — failure.
    Err,
}

/// Parse a `+OK` / `-ERR` status line; `None` for anything else.
/// Returns the status and the free text after it.
pub fn parse_line(line: &[u8]) -> Option<(Status, String)> {
    let s = std::str::from_utf8(line).ok()?;
    for (tag, st) in [("+OK", Status::Ok), ("-ERR", Status::Err)] {
        if let Some(rest) = s.strip_prefix(tag) {
            if rest.is_empty() || rest.starts_with(' ') {
                return Some((st, rest.trim_start().to_string()));
            }
            return None;
        }
    }
    None
}

/// Extract the lines of a POP3 multi-line reply body (before the
/// terminating `.`), undoing `..` dot-stuffing. `None` when the
/// terminator is missing.
pub fn parse_multiline(d: &[u8]) -> Option<Vec<String>> {
    let mut lines = Vec::new();
    for line in d.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line == b"." {
            return Some(lines);
        }
        let unstuffed = if line.starts_with(b"..") {
            &line[1..] // remove exactly one stuffed dot
        } else {
            line
        };
        lines.push(String::from_utf8_lossy(unstuffed).into_owned());
    }
    None
}

/// A parsed POP3 session transcript: status replies in order.
#[derive(Clone, Debug)]
pub struct Pop3 {
    /// Status replies `(+OK/-ERR, text)` in order.
    pub replies: Vec<(Status, String)>,
}

/// Parse a transcript of status lines (one per line); `None` when
/// empty or any non-empty line isn't a valid status.
pub fn parse(d: &[u8]) -> Option<Pop3> {
    if d.is_empty() {
        return None;
    }
    let mut replies = Vec::new();
    for line in d.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        replies.push(parse_line(line)?);
    }
    if replies.is_empty() {
        None
    } else {
        Some(Pop3 { replies })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status() {
        assert_eq!(parse_line(b"+OK").unwrap().0, Status::Ok);
        assert_eq!(parse_line(b"-ERR no such").unwrap().1, "no such");
        assert!(parse_line(b"OK nope").is_none());
        assert!(parse_line(b"+OKAY").is_none()); // tag must end the word
        assert!(parse_line(b"").is_none());
    }

    #[test]
    fn multiline() {
        let b = parse_multiline(b"1 100\r\n2 200\r\n.\r\n").unwrap();
        assert_eq!(b, vec!["1 100", "2 200"]);
        let b = parse_multiline(b"..hidden\r\n.\r\n").unwrap();
        assert_eq!(b, vec![".hidden"]);
        assert!(parse_multiline(b"no terminator\r\n").is_none());
        assert!(parse_multiline(b"").is_none());
    }

    #[test]
    fn transcript() {
        let p = parse(b"+OK hi\r\n-ERR bad\r\n+OK bye\r\n").unwrap();
        assert_eq!(p.replies.len(), 3);
        assert_eq!(p.replies[1].0, Status::Err);
        assert!(parse(b"+OK\r\nbogus\r\n").is_none());
    }
}

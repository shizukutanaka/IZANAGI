//! mbox — the `From ` separated mail spool format behind `mime`'s
//! messages. A new message starts at a line beginning with the five
//! bytes `From ` at column 0; a literal `From ` inside a body is
//! escaped as `>From ` on the wire (`>`s are not unescaped here —
//! bytes are returned verbatim).
//!
//! Header parsing itself is `mime`'s job; this module only splits
//! the spool into per-message byte ranges.
//!
//! ```
//! use izanagi_kit::mbox;
//! let spool = b"From a@x Mon Jan  1 00:00 2024\nSubject: hi\n\nbody\n\
//!               From b@y Mon Jan  1 00:01 2024\nX: 1\n\nB\n";
//! let m = mbox::parse(spool).unwrap();
//! assert_eq!(m.messages.len(), 2);
//! assert_eq!(m.messages[0].sender_line(), "a@x Mon Jan  1 00:00 2024");
//! assert_eq!(mbox::body(spool, &m.messages[0]).unwrap(), b"body\n");
//! ```

use std::string::String;
use std::vec::Vec;

/// One message's byte range inside an mbox.
#[derive(Clone, Debug)]
pub struct Msg {
    /// Offset of the `From ` separator line.
    pub offset: usize,
    /// Offset of the first byte after the separator's newline
    /// (start of the header block).
    pub header_at: usize,
    /// End of the header block (start of the blank line, or of the
    /// next `From ` separator when the message lacks one).
    pub header_end: usize,
    /// Offset of the body (first byte after the blank line), equal
    /// to `end` when the message has no blank line.
    pub body_at: usize,
    /// End offset (start of the next `From ` line or EOF).
    pub end: usize,
    /// The separator line's text after `From ` (sender + date).
    pub sender: String,
}

impl Msg {
    /// The `From ` line content after the marker.
    pub fn sender_line(&self) -> &str {
        &self.sender
    }
}

/// A parsed mbox spool.
#[derive(Clone, Debug)]
pub struct Mbox {
    /// Message ranges in file order.
    pub messages: Vec<Msg>,
}

/// Splits a spool into message ranges. `None` when the file does
/// not start with `From ` (empty input is a valid empty spool).
pub fn parse(d: &[u8]) -> Option<Mbox> {
    if d.is_empty() {
        return Some(Mbox {
            messages: Vec::new(),
        });
    }
    if !d.starts_with(b"From ") {
        return None;
    }
    let mut messages = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        // separator line
        let nl = d[i..].iter().position(|&b| b == b'\n').map(|p| i + p);
        let line_end = nl.unwrap_or(d.len());
        let sender = String::from_utf8_lossy(&d[i + 5..line_end])
            .trim_end()
            .to_string();
        let mut at = nl.map(|p| p + 1).unwrap_or(d.len());
        let header_at = at;
        // headers run until a blank line or the next `From `
        let mut body_at = at;
        let mut header_end = at;
        let mut saw_blank = false;
        while at < d.len() {
            let next_nl = d[at..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|p| at + p)
                .unwrap_or(d.len());
            if next_nl == at {
                // blank line ends headers
                header_end = at;
                body_at = at + 1;
                saw_blank = true;
                break;
            }
            if d[at..].starts_with(b"From ") {
                break; // message ended without blank line
            }
            at = next_nl + 1;
        }
        if !saw_blank {
            header_end = at;
            body_at = at;
        }
        // body runs until the next `From ` line
        let mut end = body_at;
        while end < d.len() {
            let next_nl = d[end..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|p| end + p)
                .unwrap_or(d.len());
            if d[end..].starts_with(b"From ") {
                break;
            }
            end = next_nl + 1;
        }
        messages.push(Msg {
            offset: i,
            header_at,
            header_end,
            body_at,
            end,
            sender,
        });
        i = end;
        if i >= d.len() {
            break;
        }
        if !d[i..].starts_with(b"From ") {
            // not a separator — corrupt tail; degrade
            return None;
        }
    }
    Some(Mbox { messages })
}

/// The header block of `m` (bytes between the separator and the
/// blank line / next message).
pub fn headers<'a>(d: &'a [u8], m: &Msg) -> Option<&'a [u8]> {
    d.get(m.header_at..m.header_end)
}

/// The body bytes of `m`.
pub fn body<'a>(d: &'a [u8], m: &Msg) -> Option<&'a [u8]> {
    d.get(m.body_at..m.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPOOL: &[u8] = b"From a@x Mon Jan  1 00:00 2024\nSubject: hi\nFrom: a@x\n\nbody\nline2\n\
                          From b@y Tue Jan  2 00:01 2024\nX: 1\n\nB\n";

    #[test]
    fn splits_two_messages() {
        let m = parse(SPOOL).unwrap();
        assert_eq!(m.messages.len(), 2);
        assert_eq!(m.messages[0].sender_line(), "a@x Mon Jan  1 00:00 2024");
        assert_eq!(m.messages[1].sender_line(), "b@y Tue Jan  2 00:01 2024");
        assert_eq!(body(SPOOL, &m.messages[0]).unwrap(), b"body\nline2\n");
        assert_eq!(body(SPOOL, &m.messages[1]).unwrap(), b"B\n");
        let h = headers(SPOOL, &m.messages[0]).unwrap();
        assert!(h.starts_with(b"Subject: hi"));
    }

    #[test]
    fn empty_spool_is_valid() {
        assert_eq!(parse(b"").unwrap().messages.len(), 0);
    }

    #[test]
    fn from_gt_line_stays_in_body() {
        let s = b"From a@x d\nH: 1\n\n>From not-a-sep\n";
        let m = parse(s).unwrap();
        assert_eq!(m.messages.len(), 1);
        assert_eq!(body(s, &m.messages[0]).unwrap(), b">From not-a-sep\n");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(b"garbage").is_none());
        assert!(parse(b" From a@x").is_none()); // leading space
    }
}

//! NNTP wire format (RFC 3977): commands `VERB [args]` and responses
//! `NNN text` where NNN's first digit is the class (1 informative,
//! 2 ok, 3 continue, 4/5 error); 1xx responses are multi-line and end
//! with a lone `.` line. `parse_line` classifies a line; `parse`
//! consumes a transcript; `parse_multiline` extracts article bodies
//! with dot-unstuffing.
//!
//! ```
//! use izanagi_kit::nntp::{parse_line, Msg};
//! let m = parse_line(b"200 news.example ready").unwrap();
//! assert_eq!(m, Msg::Response { code: 200, text: "news.example ready".into() });
//! let c = parse_line(b"GROUP alt.test").unwrap();
//! assert_eq!(c, Msg::Command { verb: "GROUP".into(), arg: "alt.test".into() });
//! ```

use std::string::String;
use std::vec::Vec;

/// One NNTP protocol line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// Client command: verb + raw argument tail.
    Command {
        /// Command verb (uppercased).
        verb: String,
        /// Argument text after the verb.
        arg: String,
    },
    /// Server response: 3-digit code + text.
    Response {
        /// Status code (first digit selects the class).
        code: u16,
        /// Text after `NNN `.
        text: String,
    },
}

/// Parse one NNTP line (no trailing CR/LF); `None` on empty/malformed
/// input.
pub fn parse_line(line: &[u8]) -> Option<Msg> {
    let s = std::str::from_utf8(line)
        .ok()?
        .trim_end_matches(['\r', '\n']);
    if s.is_empty() {
        return None;
    }
    if s.len() >= 3 && s[..3].bytes().all(|b| b.is_ascii_digit()) {
        let code: u16 = s[..3].parse().ok()?;
        if !(100..600).contains(&code) {
            return None;
        }
        let rest = s.get(3..)?;
        let text = if rest.is_empty() {
            ""
        } else {
            rest.strip_prefix(' ')?
        };
        return Some(Msg::Response {
            code,
            text: text.to_string(),
        });
    }
    let verb_end = s
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .unwrap_or(s.len());
    if verb_end == 0 || !s.as_bytes()[0].is_ascii_alphabetic() {
        return None;
    }
    let verb = &s[..verb_end];
    let rest = s.get(verb_end..)?;
    let arg = if rest.is_empty() {
        ""
    } else {
        rest.strip_prefix(' ')?
    };
    Some(Msg::Command {
        verb: verb.to_uppercase(),
        arg: arg.to_string(),
    })
}

/// Extract a multi-line response body up to the `.` terminator,
/// un-stuffing leading `..` escapes. `None` without the terminator.
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

/// Parse a transcript; `None` when empty or a line fails.
pub fn parse(d: &[u8]) -> Option<Vec<Msg>> {
    if d.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for line in d.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        out.push(parse_line(line)?);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines() {
        assert_eq!(
            parse_line(b"211 12 1 12 group sel").unwrap(),
            Msg::Response {
                code: 211,
                text: "12 1 12 group sel".into()
            }
        );
        assert_eq!(
            parse_line(b"XOVER 1-10").unwrap(),
            Msg::Command {
                verb: "XOVER".into(),
                arg: "1-10".into()
            }
        );
        assert_eq!(
            parse_line(b"quit").unwrap(),
            Msg::Command {
                verb: "QUIT".into(),
                arg: "".into()
            }
        );
        assert!(parse_line(b"099 bad").is_none());
        assert!(parse_line(b"700 bad").is_none());
        assert!(parse_line(b"123abc").is_none());
        assert!(parse_line(b"!cmd").is_none());
        assert!(parse_line(b"").is_none());
    }

    #[test]
    fn bodies() {
        let b = parse_multiline(b"Subject: x\r\n\r\nbody\r\n.\r\n").unwrap();
        assert_eq!(b.len(), 3);
        let b = parse_multiline(b"..dotline\r\n.\r\n").unwrap();
        assert_eq!(b, vec![".dotline"]);
        assert!(parse_multiline(b"no end\n").is_none());
        let t = parse(b"200 hi\r\nLIST\r\n215 list follows\r\n").unwrap();
        assert_eq!(t.len(), 3);
    }
}

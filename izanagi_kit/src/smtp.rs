//! SMTP wire lines (RFC 5321): commands `VERB [params]` and replies
//! `NNN-text` / `NNN text` (a `-` after the code means another reply
//! line follows). `parse_line` classifies one line; `parse` consumes
//! a whole transcript split on CR LF or LF.
//!
//! ```
//! use izanagi_kit::smtp::{parse_line, Msg};
//! let m = parse_line(b"MAIL FROM:<a@b>").unwrap();
//! assert_eq!(m, Msg::Command { verb: "MAIL".into(), arg: "FROM:<a@b>".into() });
//! let r = parse_line(b"250 OK").unwrap();
//! assert_eq!(r, Msg::Reply { code: 250, more: false, text: "OK".into() });
//! ```

use std::string::String;
use std::vec::Vec;

/// One SMTP protocol line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// Client command: verb (uppercased 4+ letters) and raw argument.
    Command {
        /// Command verb e.g. `EHLO`, `MAIL`, `RCPT`, `DATA`, `QUIT`.
        verb: String,
        /// Remainder of the line after the verb and one space.
        arg: String,
    },
    /// Server reply: 3-digit code, continuation flag, and text.
    Reply {
        /// Status code (e.g. 250, 354, 550).
        code: u16,
        /// `true` when the code is followed by `-` (more lines follow).
        more: bool,
        /// Text after the code and separator.
        text: String,
    },
}

/// Parse one SMTP line (no trailing CR/LF); `None` on empty or
/// malformed input.
pub fn parse_line(line: &[u8]) -> Option<Msg> {
    let s = std::str::from_utf8(line)
        .ok()?
        .trim_end_matches(['\r', '\n']);
    if s.is_empty() {
        return None;
    }
    // Reply: exactly three ASCII digits then ' ', '-', or end.
    if s.len() >= 3 && s[..3].bytes().all(|b| b.is_ascii_digit()) {
        let code = s[..3].parse().ok()?;
        let (more, text) = match s.as_bytes().get(3) {
            None => (false, ""),
            Some(b'-') => (true, s.get(4..)?),
            Some(b' ') => (false, s.get(4..)?),
            _ => return None,
        };
        return Some(Msg::Reply {
            code,
            more,
            text: text.to_string(),
        });
    }
    // Command: leading-alphabetic verb, then optional space + argument.
    let verb_end = s
        .find(|c: char| !(c.is_ascii_alphabetic()))
        .unwrap_or(s.len());
    if verb_end == 0 {
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

/// Parse a transcript (lines split on `\n`); `None` when empty or a
/// line fails `parse_line`.
pub fn parse(d: &[u8]) -> Option<Vec<Msg>> {
    if d.is_empty() {
        return None;
    }
    let mut msgs = Vec::new();
    for line in d.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        msgs.push(parse_line(line)?);
    }
    if msgs.is_empty() {
        None
    } else {
        Some(msgs)
    }
}

/// Parse a `MAIL FROM:`/`RCPT TO:` argument, returning the address
/// inside `<…>`; extension parameters after the bracket are ignored.
pub fn addr_arg(arg: &str) -> Option<&str> {
    let colon = arg.find(':')?;
    let rest = arg[colon + 1..].trim_start();
    let rest = rest.strip_prefix('<')?;
    let end = rest.find('>')?;
    let addr = &rest[..end];
    if addr.is_empty() {
        None
    } else {
        Some(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies() {
        let r = parse_line(b"220 mx.example ESMTP").unwrap();
        assert_eq!(
            r,
            Msg::Reply {
                code: 220,
                more: false,
                text: "mx.example ESMTP".into()
            }
        );
        let r = parse_line(b"250-PIPELINING").unwrap();
        assert_eq!(
            r,
            Msg::Reply {
                code: 250,
                more: true,
                text: "PIPELINING".into()
            }
        );
        let r = parse_line(b"250").unwrap();
        assert_eq!(
            r,
            Msg::Reply {
                code: 250,
                more: false,
                text: "".into()
            }
        );
    }

    #[test]
    fn commands() {
        let c = parse_line(b"EHLO client.example").unwrap();
        assert_eq!(
            c,
            Msg::Command {
                verb: "EHLO".into(),
                arg: "client.example".into()
            }
        );
        let c = parse_line(b"quit").unwrap();
        assert_eq!(
            c,
            Msg::Command {
                verb: "QUIT".into(),
                arg: "".into()
            }
        );
        assert!(parse_line(b"").is_none());
        assert!(parse_line(b"25X bad").is_none());
        assert!(parse_line(b"!!!").is_none());
    }

    #[test]
    fn transcript_and_addr() {
        let t =
            parse(b"220 mx ESMTP\r\nEHLO a\n250-OK\r\n250 DONE\r\nMAIL FROM:<x@y>\r\n").unwrap();
        assert_eq!(t.len(), 5);
        assert_eq!(addr_arg("FROM:<x@y>"), Some("x@y"));
        assert_eq!(addr_arg("TO: <x@y> SIZE=99"), Some("x@y"));
        assert!(addr_arg("FROM:").is_none());
        assert!(addr_arg("FROM:<>").is_none());
        assert!(parse(b"").is_none());
    }
}

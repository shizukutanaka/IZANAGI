//! EDIFACT (UN/EDIFACT) interchange parsing.
//!
//! Optional 6-byte `UNA:+.?x '` service string sets the
//! delimiters; otherwise the defaults `+:x?x'` apply and the file
//! must open with `UNB`. Segments end with the terminator,
//! elements split on `+`, components on `:`; `?` escapes a
//! delimiter.
//!
//! ```
//! use izanagi_kit::edi;
//! let d = b"UNB+UNOA:1+SENDER+RECEIVER+240101:0000+1'UNH+1+INVOIC:D:96A:UN'UNT+2+1'UNZ+1+1'";
//! let e = edi::parse(d).unwrap();
//! assert_eq!(e.segments.len(), 4);
//! assert_eq!(e.segments[0].tag, "UNB");
//! ```

use std::string::String;
use std::vec::Vec;

/// Delimiters used by the interchange.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Delims {
    /// Component separator (default `:`).
    pub component: u8,
    /// Element separator (default `+`).
    pub element: u8,
    /// Decimal marker (default `.`).
    pub decimal: u8,
    /// Release/escape char (default `?`).
    pub escape: u8,
    /// Segment terminator (default `'`).
    pub terminator: u8,
}

/// One segment: tag + elements (each a list of components).
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    /// 3+ letter tag (`UNB`, `UNH`, `BGM`…).
    pub tag: String,
    /// Elements, each split into components.
    pub elements: Vec<Vec<String>>,
}

/// A parsed interchange.
#[derive(Clone, Debug, PartialEq)]
pub struct Edi {
    /// Delimiters in effect.
    pub delims: Delims,
    /// Segments in order.
    pub segments: Vec<Segment>,
    /// `UNB` sender id when present.
    pub sender: Option<String>,
    /// `UNB` receiver id when present.
    pub receiver: Option<String>,
    /// `UNH` message types (`INVOIC`, `ORDERS`…).
    pub message_types: Vec<String>,
    /// Whether `UNT`/`UNZ` counters closed the envelope.
    pub closed: bool,
}

fn split_esc(s: &[u8], sep: u8, esc: u8) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        let b = s[i];
        if b == esc && i + 1 < s.len() {
            cur.push(s[i + 1]);
            i += 2;
            continue;
        }
        if b == sep {
            out.push(String::from_utf8_lossy(&cur).into_owned());
            cur = Vec::new();
        } else {
            cur.push(b);
        }
        i += 1;
    }
    out.push(String::from_utf8_lossy(&cur).into_owned());
    out
}

/// Parses an EDIFACT interchange: optional `UNA` service string,
/// mandatory `UNB` first segment, `'` (or declared) segment
/// terminator, escapes honoured.
pub fn parse(d: &[u8]) -> Option<Edi> {
    let mut delims = Delims {
        component: b':',
        element: b'+',
        decimal: b'.',
        escape: b'?',
        terminator: b'\'',
    };
    let mut at = 0usize;
    if d.starts_with(b"UNA") {
        if d.len() < 9 {
            return None;
        }
        delims = Delims {
            component: d[3],
            element: d[4],
            decimal: d[5],
            escape: d[6],
            terminator: d[8],
        };
        at = 9;
    }
    while at < d.len() && (d[at] == b'\n' || d[at] == b'\r' || d[at] == b' ') {
        at += 1;
    }
    if d.get(at..at + 3) != Some(b"UNB") {
        return None;
    }
    let mut segments = Vec::new();
    let mut cur = Vec::new();
    let mut i = at;
    while i < d.len() {
        let b = d[i];
        if b == delims.escape && i + 1 < d.len() {
            cur.push(b);
            cur.push(d[i + 1]);
            i += 2;
            continue;
        }
        if b == delims.terminator {
            // finish segment
            if !cur.is_empty() {
                let mut it = split_esc(&cur, delims.element, delims.escape).into_iter();
                let tag = it.next()?;
                if !(tag.len() >= 3
                    && tag
                        .bytes()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()))
                {
                    return None;
                }
                segments.push(Segment {
                    tag,
                    elements: it
                        .map(|e| split_esc(e.as_bytes(), delims.component, delims.escape))
                        .collect(),
                });
                cur = Vec::new();
            }
            i += 1;
            // skip whitespace between segments
            while i < d.len() && (d[i] == b'\n' || d[i] == b'\r' || d[i] == b' ') {
                i += 1;
            }
            continue;
        }
        cur.push(b);
        i += 1;
        if cur.len() > 1 << 20 {
            return None;
        }
    }
    if segments.is_empty() || segments[0].tag != "UNB" || segments[0].elements.is_empty() {
        return None;
    }
    let unb = &segments[0];
    let sender = unb.elements.get(1).and_then(|e| e.first()).cloned();
    let receiver = unb.elements.get(2).and_then(|e| e.first()).cloned();
    let message_types: Vec<String> = segments
        .iter()
        .filter(|s| s.tag == "UNH")
        .filter_map(|s| s.elements.get(1).and_then(|e| e.first()).cloned())
        .collect();
    let closed = segments.iter().any(|s| s.tag == "UNZ");
    Some(Edi {
        delims,
        segments,
        sender,
        receiver,
        message_types,
        closed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    const DOC: &[u8] = b"UNB+UNOA:1+SENDER+RECEIVER+240101:0000+REF'UNH+1+INVOIC:D:96A:UN'BGM+380+INV001'UNT+3+1'UNZ+1+REF'";

    #[test]
    fn parses_default_delims() {
        let e = parse(DOC).unwrap();
        assert_eq!(e.segments.len(), 5);
        assert_eq!(e.sender.as_deref(), Some("SENDER"));
        assert_eq!(e.receiver.as_deref(), Some("RECEIVER"));
        // first component of UNH element 2 is the message-type code
        assert_eq!(e.message_types, vec!["INVOIC".to_string()]);
        assert!(e.closed);
    }

    #[test]
    fn parses_una_and_rejects() {
        let d = b"UNA:+.? '\nUNB+UNOA:1+A+B+240101:0000+R'UNZ+0+R'";
        let e = parse(d).unwrap();
        assert_eq!(e.delims.element, b'+');
        assert_eq!(e.segments.len(), 2);
        assert!(parse(b"ORDERS+1'").is_none()); // no UNB
        assert!(parse(b"UNB'").is_none());
    }
}

//! FIXML (FIX session layer in XML) — `<FIXML>` root carrying `v=`/`r=`/`s=`
//! version attributes; direct children are FIX message elements
//! (`NewOrdSingle`, `TrdCaptRpt`, `ExecutionReport`, …), and `<Batch>` wraps a
//! multi-message element. A tiny depth scanner collects the root's immediate
//! children as the message list — components nested inside a message stay out.
//!
//! ```
//! let d = b"<FIXML v=\"FIX\x2e5\x2e0SP2\" r=\"20090223\" s=\"20090914\"><NewOrdSingle ClOrdID=\"x\"><OrderQtyData OrderQty=\"100\"/></NewOrdSingle><TrdCaptRpt RptID=\"1\"/></FIXML>";
//! let f = izanagi_kit::fixml::parse(d).unwrap();
//! assert_eq!(f.version, "FIX\x2e5\x2e0SP2");
//! assert_eq!(f.messages, vec!["NewOrdSingle".to_string(), "TrdCaptRpt".to_string()]);
//! assert_eq!(f.msg_count, 2);
//! assert!(!f.batched);
//! assert!(izanagi_kit::fixml::detect(d));
//! ```

/// A censused FIXML document.
pub struct Fixml {
    /// `v=` version attribute (e.g. `FIX.4.4`, `FIX.5.0SP2`).
    pub version: String,
    /// `r=` release / `s=` service-pack attributes (raw).
    pub release: String,
    /// Top-level message element names, in document order.
    pub messages: Vec<String>,
    /// `messages.len()` as a count.
    pub msg_count: u32,
    /// Whether a `<Batch>` wrapper element appears.
    pub batched: bool,
}

fn attr<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let i = s.find(&format!("{name}="))? + name.len() + 1;
    let rest = &s[i..];
    let q = rest.as_bytes().first().copied()?;
    if q != b'"' && q != b'\'' {
        return None;
    }
    let rest = &rest[1..];
    let j = rest.find(q as char)?;
    Some(&rest[..j])
}

/// `<FIXML` root element (with or without a namespace prefix).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("<FIXML") || s.contains(":FIXML")
}

/// Parses the document; `None` without a `FIXML` root element.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Fixml> {
    let s = core::str::from_utf8(b).ok()?;
    let root = s.find("<FIXML").or_else(|| s.find(":FIXML"))?;
    let head = s.get(root..root + 2048).unwrap_or(&s[root..]);
    // Depth scanner: record element names at depth 1 (children of the root).
    let mut depth: i32 = -1;
    let mut messages = Vec::new();
    let mut i = root;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let gt = match s[i..].find('>') {
            Some(k) => i + k,
            None => break,
        };
        let tag = &s[i + 1..gt];
        if tag.starts_with('!') || tag.starts_with('?') {
            i = gt + 1;
            continue;
        }
        if let Some(_close) = tag.strip_prefix('/') {
            depth -= 1;
            i = gt + 1;
            continue;
        }
        let self_closing = tag.ends_with('/');
        let name: String = tag
            .trim_end_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        depth += 1;
        if depth == 1 && !name.is_empty() {
            messages.push(name);
        }
        if self_closing {
            depth -= 1;
        }
        i = gt + 1;
        if depth < 0 {
            break;
        }
    }
    Some(Fixml {
        version: attr(head, "v").unwrap_or("").to_string(),
        release: attr(head, "r").unwrap_or("").to_string(),
        msg_count: u32::try_from(messages.len()).unwrap_or(u32::MAX),
        batched: s.contains("<Batch"),
        messages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = b"<FIXML v=\"FIX\x2e5\x2e0SP2\" r=\"20090223\" s=\"20090914\"><NewOrdSingle ClOrdID=\"x\"><OrderQtyData OrderQty=\"100\"/></NewOrdSingle><TrdCaptRpt RptID=\"1\"/></FIXML>";

    #[test]
    fn detect_works() {
        assert!(detect(FIXTURE));
        assert!(detect(b"<f:FIXML/>"));
        assert!(!detect(b"8=FIX\x2e4\x2e4\x01"));
    }

    #[test]
    fn parses() {
        let f = parse(FIXTURE).unwrap();
        assert_eq!(f.version, "FIX\x2e5\x2e0SP2");
        assert_eq!(f.release, "20090223");
        assert_eq!(f.messages.len(), 2);
        assert_eq!(f.messages[0], "NewOrdSingle");
        assert_eq!(f.messages[1], "TrdCaptRpt");
        assert_eq!(f.msg_count, 2);
        assert!(!f.batched);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<xml/>").is_none());
    }
}

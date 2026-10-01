//! vCard (RFC 6350 / 2426) — `BEGIN:VCARD`/`VERSION:`/`END:VCARD` blocks
//! of `PROP;params:value` lines with CRLF folding; counts the standard
//! property names.
//!
//! ```
//! let d = b"BEGIN:VCARD\r\nVERSION:3\x2e0\r\nFN:Jane Doe\r\nEND:VCARD\r\n";
//! let v = izanagi_kit::vcard::parse(d).unwrap();
//! assert_eq!(v.version, Some(3));
//! assert_eq!(v.props, 1);
//! assert!(izanagi_kit::vcard::detect(d));
//! ```

/// Census of a vCard stream.
#[derive(Debug, Clone)]
pub struct Vcard {
    /// `BEGIN:VCARD` count.
    pub cards: usize,
    /// VERSION major digit (first digit after `VERSION:`).
    pub version: Option<u32>,
    /// Property lines (excluding BEGIN/END/VERSION bookkeeping).
    pub props: usize,
    /// Standard props found (FN/N/EMAIL/TEL/ORG/TITLE/ADR/URL/NOTE/BDAY).
    pub standard: usize,
    /// `X-` extension props.
    pub x_props: usize,
    /// Grouped props `name.PROP:`.
    pub grouped: usize,
    /// `PHOTO`/`LOGO`/`SOUND`/`KEY` media props.
    pub media: usize,
    /// Folded continuation lines (start with space/tab).
    pub folded: usize,
    /// Params `;` chunks before `:`.
    pub params: usize,
    /// `END:VCARD` count.
    pub ends: usize,
}

const STANDARD: &[&[u8]] = &[
    b"FN", b"N", b"EMAIL", b"TEL", b"ORG", b"TITLE", b"ADR", b"URL", b"NOTE", b"BDAY",
];

/// Detects a vCard: `BEGIN:VCARD` + `VERSION:` + `END:VCARD`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.windows(11).any(|w| w == b"BEGIN:VCARD")
        && b.windows(8).any(|w| w == b"VERSION:")
        && b.windows(9).any(|w| w == b"END:VCARD")
}

/// Parses a vCard stream; `None` when no card block is present.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Vcard> {
    if !detect(b) {
        return None;
    }
    let t = String::from_utf8_lossy(b);
    let mut v = Vcard {
        cards: t.matches("BEGIN:VCARD").count(),
        version: None,
        props: 0,
        standard: 0,
        x_props: 0,
        grouped: 0,
        media: 0,
        folded: 0,
        params: 0,
        ends: t.matches("END:VCARD").count(),
    };
    for line in t.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            v.folded += 1;
            continue;
        }
        let head = line.split(':').next().unwrap_or("");
        if head.starts_with("VERSION") && v.version.is_none() {
            v.version = line
                .chars()
                .find(|c| c.is_ascii_digit())
                .and_then(|c| c.to_digit(10));
            continue;
        }
        if head.starts_with("BEGIN") || head.starts_with("END") || head.is_empty() {
            continue;
        }
        if head.is_empty() || !head.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            continue;
        }
        v.props += 1;
        let name = head.split(';').next().unwrap_or("");
        if head.contains(';') {
            v.params += 1;
        }
        if name.contains('.') {
            v.grouped += 1;
        }
        let bare = name.rsplit('.').next().unwrap_or(name);
        if STANDARD.contains(&bare.as_bytes()) {
            v.standard += 1;
        }
        if bare.starts_with("X-") {
            v.x_props += 1;
        }
        if matches!(bare, "PHOTO" | "LOGO" | "SOUND" | "KEY") {
            v.media += 1;
        }
    }
    (v.cards > 0).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"BEGIN:VCARD\r\nVERSION:3\x2e0\r\nFN:Jane\r\nN:Doe;Jane;;;\r\nEMAIL;TYPE=HOME:j@x.y\r\nX-CUSTOM:z\r\nEND:VCARD\r\n";
        let v = parse(d).unwrap();
        assert_eq!(v.cards, 1);
        assert_eq!(v.version, Some(3));
        assert_eq!(v.standard, 3);
        assert_eq!(v.x_props, 1);
        assert_eq!(v.params, 1);
    }

    #[test]
    fn folded() {
        let d = b"BEGIN:VCARD\nVERSION:4\x2e0\nNOTE:aaa\n  bbb\nEND:VCARD\n";
        let v = parse(d).unwrap();
        assert_eq!(v.folded, 1);
        assert_eq!(v.version, Some(4));
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"BEGIN:VCARD\nVERSION:2\x2e1\nEND:VCARD"));
        assert!(!detect(b"BEGIN:VCARD"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello").is_none());
    }
}

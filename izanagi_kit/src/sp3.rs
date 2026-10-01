//! SP3 (Standard Product 3) precise-orbit file scanner.
//!
//! IGS SP3 files start with a version/format line (`#aP`, `#bP`,
//! `#cP`, `#dP`) where the second char is the version letter and the
//! third `P`(position) or `V`(position+velocity), followed by a
//! `##` continuation line and epoch records `*  YYYY MM DD …`.
//!
//! ```
//! let d = b"#aP2026  1  1  0  0  0\x2e00000000      96   d   ITRF\n## 1000  0\x2e00000000  900\x2e00000000 60000 0\x2e0000000000000\n*  2026  1  1  0  0  0\x2e00000000\n";
//! let s = izanagi_kit::sp3::parse(d).unwrap();
//! assert_eq!(s.version, "a");
//! assert_eq!(s.has_velocity, false);
//! ```
//!
//! Reference: SP3-c/d format documentation (IGS/NGA "The Standard
//! Product Format") — `#` header lines, `##` aux line, `*` epoch
//! records, `P`/`V` satellite lines, `EOF` terminator.

/// Parsed SP3 fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Sp3 {
    /// Version letter (`a`–`d`).
    pub version: String,
    /// `true` for `V` files (position + velocity records).
    pub has_velocity: bool,
    /// Epoch count declared on the first line, if parsed.
    pub epochs_declared: Option<u32>,
    /// `* ` epoch record count.
    pub epochs: usize,
    /// `P`/`V` satellite record count.
    pub records: usize,
    /// `EOF` terminator present.
    pub eof: bool,
}

/// Parse an SP3 file; `None` unless the `#xP`/`#xV` signature and a
/// `##` aux line exist.
pub fn parse(d: &[u8]) -> Option<Sp3> {
    let first = d.split(|&b| b == b'\n').next()?;
    if first.len() < 3 || first[0] != b'#' || !matches!(first[1], b'a'..=b'd') {
        return None;
    }
    let (version, has_velocity) = match first[2] {
        b'P' => (String::from_utf8_lossy(&first[1..2]).into_owned(), false),
        b'V' => (String::from_utf8_lossy(&first[1..2]).into_owned(), true),
        _ => return None,
    };
    // the declared epoch count is the last all-digit field of the
    // first line (before the orbit/agency letter columns).
    let epochs_declared = first
        .split(|b| *b == b' ')
        .rfind(|f| !f.is_empty() && f.iter().all(|b| b.is_ascii_digit()))
        .and_then(|f| core::str::from_utf8(f).ok()?.parse().ok());
    let mut epochs = 0usize;
    let mut records = 0usize;
    let mut eof = false;
    let mut saw_aux = false;
    for l in d.split(|&b| b == b'\n').skip(1) {
        if l.starts_with(b"##") {
            saw_aux = true;
        }
        if l.starts_with(b"*  ") || l.starts_with(b"* ") {
            epochs += 1;
        }
        if l.starts_with(b"P") || l.starts_with(b"V") {
            records += 1;
        }
        if l.starts_with(b"EOF") {
            eof = true;
        }
    }
    if !saw_aux {
        return None;
    }
    Some(Sp3 {
        version,
        has_velocity,
        epochs_declared,
        epochs,
        records,
        eof,
    })
}

/// `true` if the buffer looks like an SP3 file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"#aP2026  1  1  0  0  0\x2e00000000      96   d   ITRF\n## 1000  0\x2e00000000  900\x2e00000000 60000 0\x2e0000000000000\n*  2026  1  1  0  0  0\x2e00000000\nP  1  1000\x2e000 -2000\x2e000 3000\x2e000\nEOF\n";

    #[test]
    fn parses() {
        let s = parse(DOC).unwrap();
        assert_eq!(s.version, "a");
        assert!(!s.has_velocity);
        assert_eq!(s.epochs_declared, Some(96));
        assert_eq!(s.epochs, 1);
        assert_eq!(s.records, 1);
        assert!(s.eof);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#xP\n").is_none());
        assert!(parse(b"#aP x\n*  2026\n").is_none()); // no ## line
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"#aP"));
    }
}

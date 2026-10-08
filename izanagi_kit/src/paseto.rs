//! PASETO token — `v{1..4}.{public|local}.payload[.footer]` where the
//! payload is base64url (plus implicit/asserted footer segment).
//!
//! ```
//! let d = b"v2.public.aGVsbG8.footerdata";
//! let p = izanagi_kit::paseto::parse(d).unwrap();
//! assert_eq!(p.version, Some(2));
//! assert!(p.public);
//! assert_eq!(p.footer, Some(10));
//! assert!(izanagi_kit::paseto::detect(d));
//! ```

use crate::textutil::strip_bom;
/// A PASETO token census.
#[derive(Debug, Clone)]
pub struct Paseto {
    /// Protocol version 1–4.
    pub version: Option<u8>,
    /// `public` (asymmetric) vs `local` (symmetric).
    pub public: bool,
    /// Payload segment length (base64url chars).
    pub payload_len: usize,
    /// Optional footer segment length.
    pub footer: Option<usize>,
    /// Segment count (3 or 4).
    pub segments: usize,
}

fn seg_ok(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'=')
}

/// Detects a PASETO: `vN.purpose.` with purpose `local`/`public`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let t = t.trim();
    let mut it = t.split('.');
    let ver = it.next().unwrap_or("");
    let pur = it.next().unwrap_or("");
    ver.len() == 2
        && ver.starts_with('v')
        && matches!(ver.as_bytes()[1], b'1'..=b'4')
        && (pur == "local" || pur == "public")
        && it.next().is_some_and(seg_ok)
}

/// Parses a PASETO; `None` on bad header or segments.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Paseto> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    if !detect(b) {
        return None;
    }
    let t = t.trim();
    let segs: Vec<&str> = t.split('.').collect();
    if !(3..=4).contains(&segs.len()) || !segs[2..].iter().all(|s| seg_ok(s)) {
        return None;
    }
    Some(Paseto {
        version: segs[0].as_bytes().get(1).map(|c| c - b'0'),
        public: segs[1] == "public",
        payload_len: segs[2].len(),
        footer: segs.get(3).map(|s| s.len()),
        segments: segs.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let p = parse(b"v2.public.aGVsbG8.footerdata").unwrap();
        assert_eq!(p.version, Some(2));
        assert!(p.public);
        assert_eq!(p.payload_len, 7);
        assert_eq!(p.footer, Some(10));
        assert_eq!(p.segments, 4);
    }

    #[test]
    fn parses_nof() {
        let p = parse(b"v4.local.aXY=").unwrap();
        assert!(!p.public);
        assert_eq!(p.footer, None);
        assert_eq!(p.segments, 3);
    }

    #[test]
    fn detect_works() {
        assert!(detect(b"v1.local.xx"));
        assert!(detect(b"v3.public.yy.zz"));
        assert!(!detect(b"v5.local.xx"));
        assert!(!detect(b"local.v2.xx"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"v2.public.").is_none());
        assert!(parse(b"plain text").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

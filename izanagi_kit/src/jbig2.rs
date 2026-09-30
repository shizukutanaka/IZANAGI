//! JBIG2 bi-level image stream scanner.
//!
//! A JBIG2 *file* (as opposed to an embedded stream) starts with
//! the 8-byte signature `97 4A 42 32 0D 0A 1A 0A`, then a flags
//! byte (bit 0 = sequential organization, bit 1 = unknown page
//! count) and, when the page count is known, a BE `u32` page
//! count.
//!
//! ```
//! let mut d = b"\x97JB2\r\n\x1a\n".to_vec();
//! d.push(1);            // sequential, page count present
//! d.extend_from_slice(&[0, 0, 0, 3]);
//! let j = izanagi_kit::jbig2::parse(&d).unwrap();
//! assert!(j.sequential);
//! assert_eq!(j.pages, Some(3));
//! ```
//!
//! Reference: ITU-T T.88 / ISO 14492 JBIG2 — Annex D file
//! signature `0x97 4A 42 32 0D 0A 1A 0A` and the flags/pages
//! header layout.

/// Parsed JBIG2 header fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Jbig2 {
    /// Sequential organization flag (flags bit 0).
    pub sequential: bool,
    /// `true` when the page count field is absent (flags bit 1).
    pub pages_unknown: bool,
    /// Declared page count when present.
    pub pages: Option<u32>,
    /// Bytes remaining after the header (segment data).
    pub body_bytes: usize,
}

const SIG: &[u8; 8] = b"\x97JB2\r\n\x1a\n";

fn u32be(d: &[u8], i: usize) -> u32 {
    ((d[i] as u32) << 24) | ((d[i + 1] as u32) << 16) | ((d[i + 2] as u32) << 8) | d[i + 3] as u32
}

/// Parse a JBIG2 file header; `None` unless the signature is
/// present and the flags/pages region is complete.
pub fn parse(d: &[u8]) -> Option<Jbig2> {
    if d.len() < 9 || &d[..8] != SIG {
        return None;
    }
    let flags = d[8];
    let sequential = flags & 1 != 0;
    let pages_unknown = flags & 2 != 0;
    let (pages, body_start) = if pages_unknown {
        (None, 9)
    } else {
        if d.len() < 13 {
            return None;
        }
        (Some(u32be(d, 9)), 13)
    };
    Some(Jbig2 {
        sequential,
        pages_unknown,
        pages,
        body_bytes: d.len() - body_start,
    })
}

/// `true` if the buffer looks like a JBIG2 file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut d = SIG.to_vec();
        d.push(1);
        d.extend_from_slice(&[0, 0, 0, 3]);
        let j = parse(&d).unwrap();
        assert!(j.sequential);
        assert!(!j.pages_unknown);
        assert_eq!(j.pages, Some(3));
        assert_eq!(j.body_bytes, 0);
    }

    #[test]
    fn unknown_pages() {
        let mut d = SIG.to_vec();
        d.push(0x23); // sequential + unknown pages + some bits
        let j = parse(&d).unwrap();
        assert!(j.sequential);
        assert!(j.pages_unknown);
        assert_eq!(j.pages, None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&SIG[..]).is_none()); // no flags byte
        let mut d = SIG.to_vec();
        d.push(0); // pages promised but absent
        assert!(parse(&d).is_none());
        d.extend_from_slice(&[0, 0, 0]); // only 3 of 4
        assert!(parse(&d).is_none());
    }

    #[test]
    fn detects() {
        let mut d = SIG.to_vec();
        d.extend_from_slice(&[3, 0, 0, 0, 1]);
        assert!(detect(&d));
        assert!(!detect(b"JBIG2"));
    }
}

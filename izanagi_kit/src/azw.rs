//! AZW3 / KF8 detection on top of PalmDOC: an AZW3 container is a MOBI
//! file that additionally carries a `BOUNDARY` record splitting the old
//! (MOBI6) and new (KF8) sections.
//!
//! ```
//! use izanagi_kit::azw::parse;
//!
//! let mut d = vec![0u8; 200];
//! d[77] = 2;                    // records
//! d[79] = 2;                    // PalmDOC compression
//! d[94..98].copy_from_slice(b"MOBI");
//! d[150..158].copy_from_slice(b"BOUNDARY");
//! let a = parse(&d).unwrap();
//! assert!(a.kf8);
//! ```

/// Parsed AZW/KF8 classification over the PalmDOC base.
#[derive(Debug, Clone)]
pub struct Azw {
    /// Underlying PalmDOC header.
    pub base: crate::mobi::Mobi,
    /// True when the `BOUNDARY` KF8 section record exists.
    pub kf8: bool,
    /// Raw record offsets are not fully decoded — expose count only.
    pub records: u16,
}

/// Parse PalmDOC + classify: `MOBI` marker ⇒ AZW/MOBI, plus `BOUNDARY` ⇒ KF8.
pub fn parse(d: &[u8]) -> Option<Azw> {
    let base = crate::mobi::parse(d)?;
    if !base.is_mobi() {
        return None;
    }
    Some(Azw {
        records: base.records,
        kf8: base.is_kf8(),
        base,
    })
}

/// Byte offset of the KF8 `BOUNDARY` record, if any.
pub fn boundary(d: &[u8]) -> Option<usize> {
    d.windows(crate::mobi::BOUNDARY.len())
        .position(|w| w == crate::mobi::BOUNDARY)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(with_boundary: bool) -> Vec<u8> {
        let mut d = vec![0u8; 200];
        d[77] = 2;
        d[79] = 2;
        d[94..98].copy_from_slice(b"MOBI");
        if with_boundary {
            d[150..158].copy_from_slice(b"BOUNDARY");
        }
        d
    }

    #[test]
    fn kf8_and_plain() {
        assert!(parse(&fixture(true)).unwrap().kf8);
        assert!(!parse(&fixture(false)).unwrap().kf8);
        assert_eq!(boundary(&fixture(true)), Some(150));
        let m = parse(&fixture(true)).unwrap();
        assert_eq!(m.records, 2);
        assert!(m.base.is_mobi());
    }

    #[test]
    fn rejects() {
        let mut d = fixture(true);
        d[94..98].copy_from_slice(b"XXXX"); // no MOBI marker
        assert!(parse(&d).is_none());
        assert!(parse(b"").is_none());
    }
}

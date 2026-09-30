//! Brother/Babylock PES embroidery file (`#PES` + 4-digit version:
//! `0001`–`0060`+). The PES section is followed by an embedded PEC
//! stitch block — detected by `#PEC0001` or the `LA:` label marker.
//!
//! ```
//! let d = b"#PES0060\x00\x00\x01stuffstuff#PEC0001LA:  design  \r";
//! let p = izanagi_kit::pes::parse(d).unwrap();
//! assert_eq!(p.version, 60);
//! assert!(p.has_pec);
//! ```

/// A parsed PES embroidery file.
#[derive(Clone, Debug)]
pub struct Pes {
    /// PES version (`#PES00VV` → `VV`, e.g. 1, 40, 60).
    pub version: u32,
    /// Byte offset of the embedded PEC block.
    pub pec_offset: usize,
    /// Whether a PEC block was located.
    pub has_pec: bool,
}

/// Parse a PES file; `None` without the `#PES` signature + 4 decimal
/// version digits.
pub fn parse(d: &[u8]) -> Option<Pes> {
    if d.len() < 8 || !d.starts_with(b"#PES") {
        return None;
    }
    let mut version = 0u32;
    for &b in &d[4..8] {
        if !b.is_ascii_digit() {
            return None;
        }
        version = version * 10 + u32::from(b - b'0');
    }
    if version == 0 || version > 99 {
        return None;
    }
    let mut pec_offset = d.len();
    for (i, w) in d.windows(4).enumerate() {
        if w == b"#PEC" || (w.starts_with(b"LA:") && i > 8) {
            pec_offset = i;
            break;
        }
    }
    Some(Pes {
        version,
        pec_offset,
        has_pec: pec_offset < d.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v6() {
        let d = b"#PES0060blahblah#PEC0001rest";
        let p = parse(d).unwrap();
        assert_eq!(p.version, 60);
        assert_eq!(p.pec_offset, 16);
        assert!(p.has_pec);
    }

    #[test]
    fn v1_la_marker() {
        let d = b"#PES0001xxxxLA:name  \rmore";
        let p = parse(d).unwrap();
        assert_eq!(p.version, 1);
        assert!(p.has_pec);
    }

    #[test]
    fn no_pec() {
        let d = b"#PES0040 only the header section";
        let p = parse(d).unwrap();
        assert!(!p.has_pec);
        assert_eq!(p.pec_offset, d.len());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#PES").is_none());
        assert!(parse(b"#PES00XX").is_none()); // non-digit version
        assert!(parse(b"!PES0001").is_none()); // bad sig
        assert!(parse(b"#PES0000rest").is_none()); // version 0
    }
}

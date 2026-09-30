//! SAS `.sas7bdat` — proprietary datasets start with 32 zero bytes,
//! then the 16-byte `a8` magic (offsets per parso/pyreadstat). The
//! encoding tag lives at offset 70 of the 288-byte page-0 header.
//!
//! ```
//! let mut d = vec![0u8; 288];
//! d[32..48].copy_from_slice(&izanagi_kit::sas7bdat::A8_MAGIC);
//! d[70] = 20; // UTF-8 encoding tag
//! let s = izanagi_kit::sas7bdat::parse(&d).unwrap();
//! assert_eq!(s.encoding, 20);
//! assert_eq!(s.prolog_zeros, 32);
//! ```

/// The 16-byte magic at offset 32 (per parso/pyreadstat field offsets).
pub const A8_MAGIC: [u8; 16] = [
    0xC2, 0xEA, 0x81, 0x60, 0xB3, 0x14, 0x11, 0xCF, 0xBD, 0x92, 0x08, 0x00, 0x09, 0xC7, 0x31, 0x8C,
];

/// Parsed `.sas7bdat` header hints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sas7bdat {
    /// Count of leading zero bytes before the magic (32 for well-formed files).
    pub prolog_zeros: usize,
    /// Encoding tag byte at offset 70 (20 = UTF-8 in parso's table).
    pub encoding: u8,
}

/// Parse a `.sas7bdat` header; `None` without zero prolog + a8 magic.
pub fn parse(d: &[u8]) -> Option<Sas7bdat> {
    if d.len() < 288 {
        return None;
    }
    let prolog_zeros = d.iter().take_while(|&&b| b == 0).count();
    if prolog_zeros < 32 {
        return None;
    }
    if d[32..48] != A8_MAGIC {
        return None;
    }
    Some(Sas7bdat {
        prolog_zeros,
        encoding: *d.get(70)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(encoding: u8) -> Vec<u8> {
        let mut d = vec![0u8; 288];
        d[32..48].copy_from_slice(&A8_MAGIC);
        d[70] = encoding;
        d
    }

    #[test]
    fn basic() {
        let s = parse(&hdr(20)).unwrap();
        assert_eq!(s.encoding, 20);
        assert_eq!(s.prolog_zeros, 32);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 100]).is_none()); // too short
        let mut d = hdr(0);
        d[0] = 1; // prolog must be zero
        assert!(parse(&d).is_none());
        let mut d = hdr(0);
        d[47] = 0;
        assert!(parse(&d).is_none()); // broken magic tail
    }
}

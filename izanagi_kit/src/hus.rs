//! Husqvarna `.hus` embroidery file — member of the "VSM" family of
//! Viking/Pfaff formats, identified by the documented leading bytes
//! `5D FC C8 37` (followed by a small version byte such as `0x10`).
//! The remainder is a length-prefixed structure of stitch/color
//! blocks, exposed here only as a detected header.
//!
//! ```
//! let mut d = vec![0x5D, 0xFC, 0xC8, 0x37, 0x10];
//! d.extend_from_slice(&[0u8; 32]);
//! let h = izanagi_kit::hus::parse(&d).unwrap();
//! assert_eq!(h.version, 0x10);
//! ```

/// A detected HUS file.
#[derive(Clone, Debug)]
pub struct Hus {
    /// Version byte following the signature.
    pub version: u8,
    /// File length declared inside the header region when present
    /// (0 when absent — format-dependent).
    pub declared_len: usize,
}

/// Parse a HUS header; `None` without the `5D FC C8` family prefix.
pub fn parse(d: &[u8]) -> Option<Hus> {
    if d.len() < 8 || d[..4] != [0x5D, 0xFC, 0xC8, 0x37] {
        return None;
    }
    // Version/subformat byte following the 4-byte signature.
    let version = d[4];
    Some(Hus {
        version,
        declared_len: d.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut d = vec![0x5D, 0xFC, 0xC8, 0x37, 0x10];
        d.extend_from_slice(&[0u8; 16]);
        let h = parse(&d).unwrap();
        assert_eq!(h.version, 0x10);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x5D, 0xFC]).is_none()); // short
        assert!(parse(&[0x5D, 0xFC, 0x00, 0x00]).is_none()); // bad 3rd byte
        assert!(parse(&[0x5D, 0xFC, 0xC8, 0x00, 0, 0, 0, 0]).is_none()); // bad 4th
        assert!(parse(&[0x80, 0x5D, 0xFC, 0xC8]).is_none()); // that's VIP order
    }
}

//! Husqvarna Viking `.vip` embroidery file — VSM-family sibling of
//! `.hus`, identified by the leading `0x80` byte in front of the
//! shared `5D FC C8 37` signature, then a version byte. The body is
//! a compressed/encoded block structure; this module only detects
//! and versions the header.
//!
//! ```
//! let mut d = vec![0x80, 0x5D, 0xFC, 0xC8, 0x37, 0x10];
//! d.extend_from_slice(&[0u8; 16]);
//! let v = izanagi_kit::vip::parse(&d).unwrap();
//! assert_eq!(v.version, 0x10);
//! ```

/// A detected VIP file.
#[derive(Clone, Debug)]
pub struct Vip {
    /// Version byte following the signature.
    pub version: u8,
}

/// Parse a VIP signature; `None` unless `80 5D FC C8 37` leads the
/// file (a bare `5D FC C8 37` is `.hus`/`.shv` — see `crate::hus`).
pub fn parse(d: &[u8]) -> Option<Vip> {
    if d.len() < 6 || d[..5] != [0x80, 0x5D, 0xFC, 0xC8, 0x37] {
        return None;
    }
    Some(Vip { version: d[5] })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vip() {
        let mut d = vec![0x80, 0x5D, 0xFC, 0xC8, 0x37, 0x10];
        d.extend_from_slice(&[0; 16]);
        assert_eq!(parse(&d).unwrap().version, 0x10);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x80, 0x5D, 0xFC]).is_none()); // short
        assert!(parse(&[0x5D, 0xFC, 0xC8, 0x37, 0x10, 0]).is_none()); // no 0x80 → HUS
        assert!(parse(&[0x80, 0x5D, 0xFC, 0xC8, 0x00, 0x10]).is_none()); // bad 5th
    }
}

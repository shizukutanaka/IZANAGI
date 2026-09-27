//! Microsoft XNA `.xnb` content-file header parsing.
//!
//! `XNB` + platform byte (`w` Windows / `x` Xbox 360 / `m` Windows
//! Phone) + `version` u8 + `flags` u8 (bit0/0x01 = HiDef profile,
//! bit7/0x80 = compressed) + `size` u32LE (uncompressed: total file
//! size; compressed: compressed-size then decompressed u32).
//!
//! ```
//! use izanagi_kit::xnb;
//! let mut d = b"XNBw".to_vec();
//! d.push(5); // version
//! d.push(0); // flags: uncompressed
//! d.extend_from_slice(&20u32.to_le_bytes()); // total file size
//! d.extend_from_slice(&[0u8; 10]); // payload
//! let x = xnb::parse(&d).unwrap();
//! assert_eq!(x.platform, b'w');
//! assert!(!x.compressed());
//! ```

/// `XNB` magic.
pub const MAGIC: &[u8; 3] = b"XNB";

/// A parsed `.xnb` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Xnb {
    /// Platform byte (`w`/`x`/`m`).
    pub platform: u8,
    /// Format `version` (5 for XNA 4 / MonoGame).
    pub version: u8,
    /// `flags` — bit0 HiDef profile, bit7 compressed.
    pub flags: u8,
    /// Declared on-disk size (compressed or total).
    pub size: u32,
    /// Compressed only: declared decompressed size.
    pub decompressed_size: Option<u32>,
    /// Offset of the content payload.
    pub data_offset: usize,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

impl Xnb {
    /// Whether flags bit7 marks the payload as compressed.
    pub fn compressed(&self) -> bool {
        self.flags & 0x80 != 0
    }
}

/// Parses an `.xnb` header: magic + legal platform + declared sizes
/// consistent with the input length.
pub fn parse(d: &[u8]) -> Option<Xnb> {
    if d.len() < 10 {
        return None;
    }
    if d.get(..3)? != MAGIC {
        return None;
    }
    let platform = *d.get(3)?;
    if !matches!(platform, b'w' | b'x' | b'm') {
        return None;
    }
    let version = *d.get(4)?;
    let flags = *d.get(5)?;
    let size = u32le(d, 6)?;
    if flags & 0x80 != 0 {
        let decompressed_size = Some(u32le(d, 10)?);
        if size as usize > d.len() {
            return None;
        }
        Some(Xnb {
            platform,
            version,
            flags,
            size,
            decompressed_size,
            data_offset: 14,
        })
    } else {
        // Uncompressed: `size` is the whole file size.
        if size as usize > d.len() {
            return None;
        }
        Some(Xnb {
            platform,
            version,
            flags,
            size,
            decompressed_size: None,
            data_offset: 10,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(flags: u8) -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.push(b'w');
        d.push(5);
        d.push(flags);
        if flags & 0x80 != 0 {
            d.extend_from_slice(&24u32.to_le_bytes()); // compressed size
            d.extend_from_slice(&64u32.to_le_bytes()); // decompressed
            d.extend_from_slice(&[0u8; 10]);
        } else {
            d.extend_from_slice(&30u32.to_le_bytes()); // file size
            d.extend_from_slice(&[0u8; 20]);
        }
        d
    }

    #[test]
    fn parses_header() {
        let x = parse(&fixture(0)).unwrap();
        assert_eq!(x.platform, b'w');
        assert_eq!(x.version, 5);
        assert!(!x.compressed());
        assert_eq!(x.data_offset, 10);
    }

    #[test]
    fn parses_compressed() {
        let x = parse(&fixture(0x80)).unwrap();
        assert!(x.compressed());
        assert_eq!(x.decompressed_size, Some(64));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 6]).is_none());
        let mut d = fixture(0);
        d[3] = b'z'; // bad platform
        assert!(parse(&d).is_none());
    }
}

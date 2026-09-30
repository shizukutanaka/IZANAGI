//! IRCAM / Berkeley / CSOUND sound files — 28-byte binary header, both byte orders.
//!
//! ```
//! let mut d = Vec::new();
//! d.extend_from_slice(&[0x64, 0xa3, 0x01, 0x00]); // little-endian magic
//! d.extend_from_slice(&[0, 0, 0xac, 0x44]); // rate f32 bits 44100\x2e0
//! d.extend_from_slice(&[0x02, 0, 0, 0]); // channels
//! d.resize(1024, 0);
//! let i = izanagi_kit::ircam::parse(&d).unwrap();
//! assert!(!i.big_endian);
//! assert_eq!(i.channels, 2);
//! assert!(izanagi_kit::ircam::detect(&d));
//! ```

/// Parsed IRCAM header.
#[derive(Debug, Clone)]
pub struct Ircam {
    /// `true` when the magic read `00 01 a3 64` (big-endian file).
    pub big_endian: bool,
    /// Raw 32 bits of the float sampling-rate field (no float math in kit).
    pub rate_bits: u32,
    /// Channel count field.
    pub channels: u32,
    /// Packing word (usually 0).
    pub packing: u32,
    /// Declared header size in bytes (16 / 28 / 1024 variants).
    pub header_size: u32,
}

fn w32(b: &[u8], o: usize, be: bool) -> u32 {
    if be {
        ((b[o] as u32) << 24)
            | ((b[o + 1] as u32) << 16)
            | ((b[o + 2] as u32) << 8)
            | (b[o + 3] as u32)
    } else {
        (b[o] as u32)
            | ((b[o + 1] as u32) << 8)
            | ((b[o + 2] as u32) << 16)
            | ((b[o + 3] as u32) << 24)
    }
}

/// Detects either IRCAM magic byte order.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 4 && (b[..4] == [0x00, 0x01, 0xa3, 0x64] || b[..4] == [0x64, 0xa3, 0x01, 0x00])
}

/// Parses an IRCAM header; `None` without the magic or when shorter than 16 bytes.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ircam> {
    if b.len() < 16 || !detect(b) {
        return None;
    }
    let be = b[..4] == [0x00, 0x01, 0xa3, 0x64];
    Some(Ircam {
        big_endian: be,
        rate_bits: w32(b, 4, be),
        channels: w32(b, 8, be),
        packing: w32(b, 12, be),
        header_size: if b.len() >= 28 { w32(b, 24, be) } else { 16 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_le() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0x64, 0xa3, 0x01, 0x00]);
        d.extend_from_slice(&[0, 0, 0xac, 0x44]);
        d.extend_from_slice(&[0x02, 0, 0, 0]);
        d.extend_from_slice(&[0x00, 0, 0, 0]);
        d.resize(1024, 0);
        let f = parse(&d).unwrap();
        assert!(!f.big_endian);
        assert_eq!(f.channels, 2);
        assert_eq!(f.rate_bits, 0x44ac0000);
        assert_eq!(f.header_size, 0);
    }

    #[test]
    fn parses_be() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0x00, 0x01, 0xa3, 0x64]);
        d.extend_from_slice(&[0x44, 0xac, 0x00, 0x00]);
        d.extend_from_slice(&[0, 0, 0, 1]);
        d.resize(1024, 0);
        let f = parse(&d).unwrap();
        assert!(f.big_endian);
        assert_eq!(f.channels, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x64, 0xa3, 0x01]).is_none());
        assert!(parse(&[0u8; 32]).is_none());
        assert!(!detect(&[0u8; 16]));
    }
}

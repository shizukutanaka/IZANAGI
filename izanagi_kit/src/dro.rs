//! DOSBox Raw OPL capture (DRO) parser.
//!
//! `.dro` files log OPL2/OPL3 register writes. Version 1 headers are
//! `DBRAWOPL` + `u16 major u16 minor u32 length_ms u32 length_bytes
//! u32 hardware u32 format u32 compression` (major 1). Version 2 adds
//! `u16 hardware_type u8 format u8 compression u8 short_delay
//! u8 long_delay u8 codemap_len` after the same magic plus a
//! `u32 data_len`. This parser reads the common prefix and reports the
//! detected version shape.
//!
//! ```
//! let mut f = b"DBRAWOPL".to_vec();
//! f.extend_from_slice(&[1, 0, 0, 0]); // v1: major=1 minor=0
//! f.extend_from_slice(&[0xE8, 0x03, 0, 0]); // 1000 ms
//! f.extend_from_slice(&[4, 0, 0, 0]); // 4 data bytes
//! f.extend_from_slice(&[0; 12]);
//! let d = izanagi_kit::dro::parse(&f).unwrap();
//! assert_eq!(d.version_major, 1);
//! assert_eq!(d.length_ms, 1000);
//! ```

/// Parsed DRO header.
#[derive(Debug, Clone, PartialEq)]
pub struct Dro {
    /// `1` for original DOSBox captures, `2` for the v2 layout.
    pub version_major: u16,
    /// Minor version byte/word.
    pub version_minor: u16,
    /// Duration of the capture in milliseconds.
    pub length_ms: u32,
    /// Length of the register-data section in bytes.
    pub data_len: u32,
    /// Hardware type (0 = OPL2, 1 = OPL3, 2 = dual OPL2).
    pub hardware: u32,
    /// Byte offset where the register-write data begins.
    pub data_offset: usize,
}

fn u16le(d: &[u8], off: usize) -> u16 {
    d[off] as u16 | ((d[off + 1] as u16) << 8)
}

fn u32le(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

/// Parse a DRO file; `None` without `DBRAWOPL` or truncation.
pub fn parse(d: &[u8]) -> Option<Dro> {
    if d.len() < 12 || &d[..8] != b"DBRAWOPL" {
        return None;
    }
    let major = u16le(d, 8);
    let minor = u16le(d, 10);
    match major {
        1 => {
            if d.len() < 28 {
                return None;
            }
            Some(Dro {
                version_major: 1,
                version_minor: minor,
                length_ms: u32le(d, 12),
                data_len: u32le(d, 16),
                hardware: u32le(d, 20),
                data_offset: 28,
            })
        }
        2 => {
            if d.len() < 21 {
                return None;
            }
            // v2: u32 data_len then u8 hw u8 fmt u8 cmpr u8 short u8 long
            //     u8 codemap_len, codemap bytes
            Some(Dro {
                version_major: 2,
                version_minor: minor,
                length_ms: u32le(d, 12),
                data_len: u32le(d, 16),
                hardware: d[20] as u32,
                data_offset: 27 + d.get(25).copied().unwrap_or(0) as usize,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1() {
        let mut f = b"DBRAWOPL".to_vec();
        f.extend_from_slice(&[1, 0, 0, 0]);
        f.extend_from_slice(&[0xE8, 0x03, 0, 0]);
        f.extend_from_slice(&[4, 0, 0, 0]);
        f.extend_from_slice(&[0; 12]);
        let d = parse(&f).unwrap();
        assert_eq!(d.version_major, 1);
        assert_eq!(d.length_ms, 1000);
        assert_eq!(d.data_len, 4);
        assert_eq!(d.data_offset, 28);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"DBRAWOPL").is_none());
        let mut f = b"DBRAWOPL".to_vec();
        f.extend_from_slice(&[9, 0, 0, 0]);
        f.resize(28, 0);
        assert!(parse(&f).is_none()); // unknown major
        let mut v2 = b"DBRAWOPL\x02\x00\x00\x00".to_vec();
        v2.resize(20, 0);
        assert!(parse(&v2).is_none()); // v2 hardware byte missing
    }
}

//! Lua precompiled chunk (`.luac`) header.
//!
//! Every chunk opens with ESC `Lua`, a version byte (`0x51`–`0x54`),
//! a format byte (0 = official) and then a version-dependent header:
//! 5.1 carries endianness + five `sizeof` bytes directly; 5.2+ add
//! the `LUAC_DATA` signature `\x19\x93\r\n\x1a\n`; 5.3/5.4 append
//! the `LUAC_INT`/`LUAC_NUM` conformance values (the number is kept
//! as raw bits — no floats in this crate).
//!
//! ```
//! use izanagi_kit::luac::{parse, V};
//! let mut d = b"\x1BLua\x53\x00\x19\x93\r\n\x1a\n".to_vec();
//! d.extend_from_slice(&[4, 8, 4, 8, 8]);
//! d.extend_from_slice(&[0x78, 0x56, 0, 0, 0, 0, 0, 0]);
//! d.extend_from_slice(&[0; 8]); // LUAC_NUM bits
//! assert_eq!(parse(&d).unwrap().version, V::V53);
//! ```

/// `ESC 'Lua'` signature.
pub const SIG: &[u8; 4] = b"\x1BLua";
/// `LUAC_DATA` tail signature for 5.3+.
pub const DATA: &[u8; 6] = b"\x19\x93\r\n\x1a\n";
/// `LUAC_INT` conformance value (`0x5678`, little-endian on disk).
pub const CHECK_INT: u64 = 0x5678;

/// Lua bytecode generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum V {
    /// Lua 5.1 chunk.
    V51,
    /// Lua 5.2 chunk.
    V52,
    /// Lua 5.3 chunk.
    V53,
    /// Lua 5.4 chunk.
    V54,
}

/// Parsed luac header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Luac {
    /// Bytecode version.
    pub version: V,
    /// `format` byte (0 = official luac output).
    pub format: u8,
    /// Byte-order flag (5.1 only: 1 = little-endian).
    pub endianness: Option<u8>,
    /// `sizeof(int)`, `sizeof(size_t)`, `sizeof(Instruction)`,
    /// `sizeof(lua_Number)`, plus (5.1) the integral flag or
    /// (5.2+) `sizeof(lua_Integer)`.
    pub sizes: [u8; 5],
    /// (5.3/5.4) `LUAC_INT` check word — should equal [`CHECK_INT`].
    pub check_int: Option<u64>,
    /// (5.3/5.4) `LUAC_NUM` check value as raw 8 bytes (a float
    /// `370.5` bit pattern when the chunk is conformant).
    pub check_num: Option<[u8; 8]>,
    /// Byte offset just past the header.
    pub header_end: usize,
}

fn le64(d: &[u8], at: usize) -> Option<u64> {
    let mut v = 0u64;
    for i in 0..8 {
        v |= u64::from(*d.get(at + i)?) << (i * 8);
    }
    Some(v)
}

/// Parse a luac chunk header. `None` on bad signature or an
/// unrecognized version.
pub fn parse(d: &[u8]) -> Option<Luac> {
    if d.get(..4)? != SIG {
        return None;
    }
    let version = match *d.get(4)? {
        0x51 => V::V51,
        0x52 => V::V52,
        0x53 => V::V53,
        0x54 => V::V54,
        _ => return None,
    };
    let format = *d.get(5)?;
    match version {
        V::V51 => {
            // 5.1: format u8, endianness u8, then int/sizet/instr/
            // number/integral sizes — a 12-byte header, no markers.
            let mut sizes = [0u8; 5];
            sizes.copy_from_slice(d.get(7..12)?);
            Some(Luac {
                version,
                format,
                endianness: Some(*d.get(6)?),
                sizes,
                check_int: None,
                check_num: None,
                header_end: 12,
            })
        }
        V::V52 | V::V53 | V::V54 => {
            // 5.2+: format u8, LUAC_DATA 6B, 5 size bytes; 5.3/5.4
            // append LUAC_INT + LUAC_NUM conformance values, and
            // 5.4 a trailing upvalue-count byte.
            if d.get(6..12)? != DATA {
                return None;
            }
            let mut sizes = [0u8; 5];
            sizes.copy_from_slice(d.get(12..17)?);
            if version == V::V52 {
                if d.len() < 17 {
                    return None;
                }
                return Some(Luac {
                    version,
                    format,
                    endianness: None,
                    sizes,
                    check_int: None,
                    check_num: None,
                    header_end: 17,
                });
            }
            let mut check_num = [0u8; 8];
            check_num.copy_from_slice(d.get(25..33)?);
            let header_end = if version == V::V54 { 34 } else { 33 };
            if d.len() < header_end {
                return None;
            }
            Some(Luac {
                version,
                format,
                endianness: None,
                sizes,
                check_int: Some(le64(d, 17)?),
                check_num: Some(check_num),
                header_end,
            })
        }
    }
}

impl Luac {
    /// True for an official-format 5.3/5.4 chunk whose `LUAC_INT`
    /// is the canonical `0x5678`; other versions check `format` only.
    pub fn conformant(&self) -> bool {
        self.format == 0
            && match self.check_int {
                Some(c) => c == CHECK_INT,
                None => true,
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f53() -> Vec<u8> {
        let mut d = b"\x1BLua\x53\x00\x19\x93\r\n\x1a\n".to_vec();
        d.extend_from_slice(&[4, 8, 4, 8, 8]); // int/sizet/instr/int64/number
        d.extend_from_slice(&[0x78, 0x56, 0, 0, 0, 0, 0, 0]); // LUAC_INT
        d.extend_from_slice(&[0, 0, 0, 0, 0, 0x28, 0x77, 0x40]); // LUAC_NUM bits
        d
    }

    #[test]
    fn v53_fields() {
        let l = parse(&f53()).unwrap();
        assert_eq!(l.version, V::V53);
        assert_eq!(l.format, 0);
        assert_eq!(l.endianness, None);
        assert_eq!(l.sizes, [4, 8, 4, 8, 8]);
        assert_eq!(l.check_int, Some(0x5678));
        assert_eq!(l.check_num, Some([0, 0, 0, 0, 0, 0x28, 0x77, 0x40]));
        assert_eq!(l.header_end, 33);
        assert!(l.conformant());
    }

    #[test]
    fn v54_and_v51() {
        let mut d = f53();
        d[4] = 0x54;
        d.push(1); // upvalue count
        let l = parse(&d).unwrap();
        assert_eq!(l.version, V::V54);
        assert_eq!(l.header_end, 34);
        // 5.2 header (DATA marker, no check values)
        let mut d52 = b"\x1BLua\x52\x00\x19\x93\r\n\x1a\n".to_vec();
        d52.extend_from_slice(&[4, 8, 4, 8, 8]);
        let l52 = parse(&d52).unwrap();
        assert_eq!(l52.version, V::V52);
        assert_eq!(l52.header_end, 17);
        assert!(l52.check_int.is_none());
        // 5.1 minimal header: version, format, endianness, 5 sizes
        let d51 = [0x1B, b'L', b'u', b'a', 0x51, 0, 1, 4, 8, 4, 8, 0];
        let l51 = parse(&d51).unwrap();
        assert_eq!(l51.version, V::V51);
        assert_eq!(l51.endianness, Some(1));
        assert_eq!(l51.sizes, [4, 8, 4, 8, 0]);
        assert_eq!(l51.header_end, 12);
        assert!(l51.check_int.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x1BLub\x51").is_none()); // bad sig
        assert!(parse(b"\x1BLua\x50\x00").is_none()); // 5.0 unsupported
        let mut d = f53();
        d[6] = 0xFF; // bad LUAC_DATA
        assert!(parse(&d).is_none());
        assert!(parse(&f53()[..20]).is_none()); // truncated
    }
}

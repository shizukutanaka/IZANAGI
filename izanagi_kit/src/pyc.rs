//! CPython `.pyc` bytecode-cache header parsing.
//!
//! 16-byte header: magic u32LE (`0x0D0A` low 16 bits — the upper bits
//! encode the interpreter's `magic_number`), `flags` u32 @4 (bit0 =
//! hash-based), then either `mtime`+`size` u32 @8/@12 (timestamp-based)
//! or an 8-byte source-hash @8. The marshalled code object follows.
//!
//! ```
//! use izanagi_kit::pyc;
//! // CPython 3.7+ magic for 3.11 is 0xA70D0D0A; use 0x12340D0A here.
//! let mut d = Vec::new();
//! d.extend_from_slice(&0x12340D0Au32.to_le_bytes());
//! d.extend_from_slice(&0u32.to_le_bytes()); // flags: timestamp-based
//! d.extend_from_slice(&0x5F00u32.to_le_bytes()); // mtime
//! d.extend_from_slice(&100u32.to_le_bytes()); // source size
//! d.extend_from_slice(b"c"); // marshal 'c'ode marker
//! let p = pyc::parse(&d).unwrap();
//! assert_eq!(p.magic_low(), 0x0D0A);
//! assert!(!p.hash_based());
//! ```

/// `.pyc` magic low bits (`\r\n`).
pub const MAGIC_LOW: u16 = 0x0D0A;

/// Timestamp `.pyc` header size.
pub const HEADER_LEN: usize = 16;

/// A parsed `.pyc` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Pyc {
    /// Full u32 magic (version tag + `0x0D0A`).
    pub magic: u32,
    /// `flags`/`bit_field` @4 (bit0 = hash-based, bit1 = check source).
    pub flags: u32,
    /// Timestamp-based: `mtime`. Hash-based: `None`.
    pub mtime: Option<u32>,
    /// Timestamp-based: source `size`. Hash-based: `None`.
    pub source_size: Option<u32>,
    /// Hash-based: the 8-byte source hash. Timestamp: `None`.
    pub source_hash: Option<u64>,
    /// Byte offset of the marshalled code object (16 or 20).
    pub code_offset: usize,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

fn u64le(d: &[u8], at: usize) -> Option<u64> {
    Some(u32le(d, at)? as u64 | (u32le(d, at + 4)? as u64) << 32)
}

impl Pyc {
    /// Low 16 bits of the magic — always `0x0D0A`.
    pub fn magic_low(&self) -> u16 {
        self.magic as u16
    }

    /// Whether flags bit0 selects the hash-based format.
    pub fn hash_based(&self) -> bool {
        self.flags & 1 != 0
    }
}

/// Parses a `.pyc` header; requires magic `??0D0A` and ≥16 bytes.
pub fn parse(d: &[u8]) -> Option<Pyc> {
    if d.len() < HEADER_LEN {
        return None;
    }
    let magic = u32le(d, 0)?;
    if magic as u16 != MAGIC_LOW {
        return None;
    }
    let flags = u32le(d, 4)?;
    if flags & 1 != 0 {
        let hash = u64le(d, 8)?;
        if d.len() < HEADER_LEN + 1 {
            return None;
        }
        return Some(Pyc {
            magic,
            flags,
            mtime: None,
            source_size: None,
            source_hash: Some(hash),
            code_offset: HEADER_LEN,
        });
    }
    Some(Pyc {
        magic,
        flags,
        mtime: Some(u32le(d, 8)?),
        source_size: Some(u32le(d, 12)?),
        source_hash: None,
        code_offset: HEADER_LEN,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(flags: u32) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&0xA70D0D0Au32.to_le_bytes()); // py3.11
        d.extend_from_slice(&flags.to_le_bytes());
        if flags & 1 != 0 {
            d.extend_from_slice(&0x11223344u64.to_le_bytes());
        } else {
            d.extend_from_slice(&0x6200u32.to_le_bytes());
            d.extend_from_slice(&256u32.to_le_bytes());
        }
        d.extend_from_slice(b"c\x00");
        d
    }

    #[test]
    fn parses_timestamp_based() {
        let p = parse(&fixture(0)).unwrap();
        assert_eq!(p.magic_low(), 0x0D0A);
        assert_eq!(p.mtime, Some(0x6200));
        assert_eq!(p.source_size, Some(256));
        assert_eq!(p.code_offset, 16);
    }

    #[test]
    fn parses_hash_based() {
        let p = parse(&fixture(1)).unwrap();
        assert!(p.hash_based());
        assert_eq!(p.source_hash, Some(0x11223344));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut d = fixture(0);
        d[0] = 0x0C; // low byte wrong
        assert!(parse(&d).is_none());
    }
}

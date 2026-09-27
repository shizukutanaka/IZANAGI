//! Android ODEX (optimized DEX) header parsing.
//!
//! Header: magic `dey\n` + version `0xx\0` (e.g. `035\0`, `036\0`,
//! `039\0`), then `checksum` u32, `dex_offset`/`dex_length`,
//! `deps_offset`/`deps_length`, `opt_offset`/`opt_length` (all u32LE).
//! VDEX variants (`vdex` magic) are out of scope.
//!
//! ```
//! use izanagi_kit::odex;
//! let mut d = b"dey\n036\0".to_vec();
//! d.extend_from_slice(&0xABCDu32.to_le_bytes()); // checksum
//! d.extend_from_slice(&40u32.to_le_bytes()); // dex_offset
//! d.extend_from_slice(&100u32.to_le_bytes()); // dex_length
//! d.extend_from_slice(&0u32.to_le_bytes()); // deps_offset
//! d.extend_from_slice(&0u32.to_le_bytes()); // deps_length
//! d.extend_from_slice(&0u32.to_le_bytes()); // opt_offset
//! d.extend_from_slice(&0u32.to_le_bytes()); // opt_length
//! d.extend_from_slice(&0u32.to_le_bytes()); // flags @36
//! d.extend_from_slice(&[0u8; 100]); // dex bytes at @40
//! let o = odex::parse(&d).unwrap();
//! assert_eq!(o.dex_len, 100);
//! ```

/// ODEX magic `dey\n`.
pub const MAGIC: &[u8; 4] = b"dey\n";

/// A parsed ODEX header.
#[derive(Clone, Debug, PartialEq)]
pub struct Odex {
    /// Version text (`036`, `039`, ...), raw 3 bytes.
    pub version: Vec<u8>,
    /// `dex_checksum`.
    pub checksum: u32,
    /// `dex_offset` — offset of the embedded DEX.
    pub dex_offset: u32,
    /// `dex_length`.
    pub dex_len: u32,
    /// `deps_offset`/`deps_length` — dependency table (0 = none).
    pub deps_offset: u32,
    /// Dependency table length.
    pub deps_len: u32,
    /// `opt_offset`/`opt_length` — optimized/auxiliary region.
    pub opt_offset: u32,
    /// Optimized region length.
    pub opt_len: u32,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses an ODEX header: magic + 3-digit version + NUL, ≥40 bytes,
/// `dex_offset`/`dex_len` must fit the input.
pub fn parse(d: &[u8]) -> Option<Odex> {
    if d.len() < 40 {
        return None;
    }
    if d.get(..4)? != MAGIC {
        return None;
    }
    let ver = d.get(4..8)?;
    if !ver[..3].iter().all(|b| b.is_ascii_digit()) || ver[3] != 0 {
        return None;
    }
    let dex_offset = u32le(d, 12)?;
    let dex_len = u32le(d, 16)?;
    if (dex_offset as usize).checked_add(dex_len as usize)? > d.len() {
        return None;
    }
    Some(Odex {
        version: ver[..3].to_vec(),
        checksum: u32le(d, 8)?,
        dex_offset,
        dex_len,
        deps_offset: u32le(d, 20)?,
        deps_len: u32le(d, 24)?,
        opt_offset: u32le(d, 28)?,
        opt_len: u32le(d, 32)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"dey\n035\0".to_vec();
        d.extend_from_slice(&0x1234u32.to_le_bytes());
        d.extend_from_slice(&40u32.to_le_bytes());
        d.extend_from_slice(&64u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes()); // flags @36
        d.extend_from_slice(&[0u8; 64]);
        d
    }

    #[test]
    fn parses_header() {
        let o = parse(&fixture()).unwrap();
        assert_eq!(o.version, b"035".to_vec());
        assert_eq!(o.checksum, 0x1234);
        assert_eq!(o.dex_offset, 40);
        assert_eq!(o.dex_len, 64);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = fixture();
        d[1] = b'x'; // "dex\n" not "dey\n"
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[12..16].copy_from_slice(&9999u32.to_le_bytes()); // dex_offset too far
        assert!(parse(&d).is_none());
    }
}

//! XZ stream header (`.xz`, liblzma container).
//!
//! An `.xz` file is a chain of *streams*, each opening with
//! `FD 37 7A 58 5A 00`, two *stream flags* bytes (reserved zero
//! bits + a 4-bit check id), and the CRC32 of the flags. Blocks
//! follow; each opens with a byte encoding `(n+1)*4` header bytes
//! plus a flag byte. `parse` reports the stream check type and the
//! first block header (when present, i.e. before the index).
//!
//! ```
//! use izanagi_kit::xz::{parse, MAGIC, Check};
//!
//! let mut d = vec![0u8; 32];
//! d[..6].copy_from_slice(&MAGIC);
//! d[7] = 1;                     // check = CRC32
//! let x = parse(&d).unwrap();
//! assert_eq!(x.check, Check::Crc32);
//! assert!(x.first_block.is_none());
//! ```

/// Magic bytes.
pub const MAGIC: [u8; 6] = [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00];
/// Stream header size (magic + flags + CRC32).
pub const STREAM_HEADER: usize = 12;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// Integrity check type from the stream flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// No check field.
    None,
    /// CRC32 (4B).
    Crc32,
    /// CRC64 (8B).
    Crc64,
    /// SHA-256 (32B).
    Sha256,
    /// Reserved or unknown id.
    Other(u8),
}

fn check_of(v: u8) -> Check {
    match v {
        0 => Check::None,
        1 => Check::Crc32,
        4 => Check::Crc64,
        10 => Check::Sha256,
        c => Check::Other(c),
    }
}

/// Byte length of a check field.
pub fn check_len(c: Check) -> usize {
    match c {
        Check::None => 0,
        Check::Crc32 => 4,
        Check::Crc64 => 8,
        Check::Sha256 => 32,
        Check::Other(_) => 0,
    }
}

/// A block header's leading fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    /// File offset of the block.
    pub at: usize,
    /// Total block-header size `(n+1)*4` bytes.
    pub header_len: usize,
    /// Block flags (compressed/uncompressed sizes, filter count).
    pub flags: u8,
}

/// A parsed XZ stream header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Xz {
    /// Check type.
    pub check: Check,
    /// CRC32 stored for the flags bytes.
    pub crc32: u32,
    /// First block header after the stream header, when the byte
    /// there is not the index marker (`0x00`).
    pub first_block: Option<Block>,
}

/// Parse one stream header. Returns `None` on bad magic, non-zero
/// reserved flag bits, or a truncated header.
pub fn parse(d: &[u8]) -> Option<Xz> {
    if d.get(..6)? != MAGIC.as_slice() {
        return None;
    }
    let f0 = d.get(6).copied()?;
    let f1 = d.get(7).copied()?;
    if f0 != 0 || f1 & 0xf0 != 0 {
        return None;
    }
    let crc32 = le32(d, 8)?;
    let first_block = if d.len() > STREAM_HEADER {
        let n = *d.get(STREAM_HEADER)?;
        if n == 0 {
            None
        } else {
            let header_len = (usize::from(n) + 1) * 4;
            let at = STREAM_HEADER;
            if at + header_len > d.len() {
                return None;
            }
            Some(Block {
                at,
                header_len,
                flags: d.get(at + 1).copied()?,
            })
        }
    } else {
        None
    };
    Some(Xz {
        check: check_of(f1 & 0x0f),
        crc32,
        first_block,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_header() {
        let mut d = vec![0u8; 12];
        d[..6].copy_from_slice(&MAGIC);
        d[7] = 4; // CRC64
        d[8] = 0x78;
        d[9] = 0x56;
        d[10] = 0x34;
        d[11] = 0x12;
        let x = parse(&d).unwrap();
        assert_eq!(x.check, Check::Crc64);
        assert_eq!(check_len(x.check), 8);
        assert_eq!(x.crc32, 0x1234_5678);
        assert!(x.first_block.is_none());
        assert_eq!(check_len(Check::None), 0);
        assert_eq!(check_len(Check::Sha256), 32);
    }

    #[test]
    fn block_header() {
        let mut d = vec![0u8; 48];
        d[..6].copy_from_slice(&MAGIC);
        d[7] = 1;
        d[12] = 7; // header size (7+1)*4 = 32
        d[13] = 0x40; // flags: one filter
        let x = parse(&d).unwrap();
        let b = x.first_block.unwrap();
        assert_eq!(b.header_len, 32);
        assert_eq!(b.at, 12);
        assert_eq!(b.flags, 0x40);
        assert_eq!(x.check, Check::Crc32);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 5]).is_none());
        let mut d = vec![0u8; 12];
        d[..6].copy_from_slice(&MAGIC);
        d[6] = 1; // reserved flag bits set
        assert!(parse(&d).is_none());
        let mut d2 = vec![0u8; 12];
        d2[..6].copy_from_slice(&MAGIC);
        d2[7] = 0x40; // high nibble reserved
        assert!(parse(&d2).is_none());
        // block size beyond buffer
        let mut d3 = vec![0u8; 14];
        d3[..6].copy_from_slice(&MAGIC);
        d3[12] = 9; // (9+1)*4 = 40 > 14-12
        assert!(parse(&d3).is_none());
        // check ids
        assert_eq!(check_of(0), Check::None);
        assert_eq!(check_of(10), Check::Sha256);
        assert_eq!(check_of(2), Check::Other(2));
    }
}

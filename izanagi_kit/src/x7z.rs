//! 7z archive signature header.
//!
//! A `.7z` opens with `37 7A BC AF 27 1C`, a 2-byte version
//! (currently `00 04`), the start-header CRC32, then the
//! *next header* pointer: offset + size (u64 LE) and its CRC32.
//! The real metadata (streams/files) lives in that next header —
//! `next_header` exposes where it sits in the file.
//!
//! ```
//! use izanagi_kit::x7z::{parse, MAGIC, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 16];
//! d[..6].copy_from_slice(&MAGIC);
//! d[6] = 0; d[7] = 4;                    // version 0.4
//! let w32 = |d: &mut [u8], o: usize, v: u32| {
//!     d[o] = v as u8; d[o + 1] = (v >> 8) as u8;
//!     d[o + 2] = (v >> 16) as u8; d[o + 3] = (v >> 24) as u8;
//! };
//! let w64 = |d: &mut [u8], o: usize, v: u64| {
//!     for i in 0..8 { d[o + i] = (v >> (i * 8)) as u8; }
//! };
//! w64(&mut d, 12, 0);                    // next header right after ours
//! w64(&mut d, 20, 16);                   // 16 bytes long
//! w32(&mut d, 28, 0x1234_5678);          // its crc
//! let z = parse(&d).unwrap();
//! assert_eq!(z.next_header(), Some((HEADER, 16)));
//! ```

/// Signature bytes.
pub const MAGIC: [u8; 6] = [0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c];
/// Signature-header size.
pub const HEADER: usize = 32;
/// `kEnd` — the property id terminating a header.
pub const K_END: u8 = 0x00;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}
fn le64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(le32(d, at)?) | u64::from(le32(d, at + 4)?) << 32)
}

/// A parsed 7z signature header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SevenZ {
    /// Major version byte.
    pub version_major: u8,
    /// Minor version byte.
    pub version_minor: u8,
    /// CRC32 over bytes 12..32 of this header.
    pub start_crc: u32,
    /// Offset of the next header, measured from the end of the
    /// signature header (i.e. file offset = `HEADER + off`).
    pub next_offset: u64,
    /// Next-header size.
    pub next_size: u64,
    /// Next-header CRC32.
    pub next_crc: u32,
}

impl SevenZ {
    /// File offset and length of the next header, when it fits
    /// inside the input.
    pub fn next_header(&self) -> Option<(usize, usize)> {
        let at = HEADER.checked_add(usize::try_from(self.next_offset).ok()?)?;
        let len = usize::try_from(self.next_size).ok()?;
        Some((at, len))
    }
}

/// Parse the 32-byte signature header. Returns `None` on a bad
/// magic or a header whose next-header range lies past the input.
pub fn parse(d: &[u8]) -> Option<SevenZ> {
    if d.get(..6)? != MAGIC.as_slice() {
        return None;
    }
    let h = SevenZ {
        version_major: d.get(6).copied()?,
        version_minor: d.get(7).copied()?,
        start_crc: le32(d, 8)?,
        next_offset: le64(d, 12)?,
        next_size: le64(d, 20)?,
        next_crc: le32(d, 28)?,
    };
    let (at, len) = h.next_header()?;
    if at.checked_add(len)? > d.len() {
        return None;
    }
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(off: u64, size: u64, total: usize) -> Vec<u8> {
        let mut d = vec![0u8; total];
        d[..6].copy_from_slice(&MAGIC);
        d[6] = 0;
        d[7] = 4;
        let w64 = |d: &mut [u8], o: usize, v: u64| {
            for i in 0..8 {
                d[o + i] = (v >> (i * 8)) as u8;
            }
        };
        w64(&mut d, 12, off);
        w64(&mut d, 20, size);
        d[28] = 0xef;
        d[29] = 0xbe;
        d[30] = 0xad;
        d[31] = 0xde;
        d
    }

    #[test]
    fn fields() {
        let d = fixture(96, 32, 160);
        let z = parse(&d).unwrap();
        assert_eq!(z.version_major, 0);
        assert_eq!(z.version_minor, 4);
        assert_eq!(z.next_offset, 96);
        assert_eq!(z.next_size, 32);
        assert_eq!(z.next_crc, 0xdead_beef);
        assert_eq!(z.next_header(), Some((128, 32)));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 10]).is_none());
        let mut d = fixture(96, 32, 160);
        d[0] = b'X';
        assert!(parse(&d).is_none());
        // next header past EOF
        let d2 = fixture(200, 32, 160);
        assert!(parse(&d2).is_none());
        let d3 = fixture(96, 320, 160);
        assert!(parse(&d3).is_none());
    }
}

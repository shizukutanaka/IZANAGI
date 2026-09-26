//! VHDX — the Hyper-V virtual disk image. Byte 0 carries the ASCII
//! signature `vhdxfile` followed by a 512-byte UTF-16LE creator
//! string; two redundant log headers sit at 64 KiB and 128 KiB, each
//! `{u32 "head", u32 crc32, u64 sequence, u16 log_version,
//! u16 version, u32 log_length, u64 log_offset}`. The *active* header
//! is the one with the higher sequence number.
//!
//! ```
//! use izanagi_kit::vhdx::{parse, header, active};
//! let mut d = vec![0u8; 3 * 65536];
//! d[..8].copy_from_slice(b"vhdxfile");
//! for i in 0..2 {
//!     let h = (1 + i) * 65536;
//!     d[h..h + 4].copy_from_slice(b"head");
//!     d[h + 8..h + 16].copy_from_slice(&(i as u64).to_le_bytes()); // seq
//!     d[h + 18..h + 20].copy_from_slice(&1u16.to_le_bytes());     // version
//! }
//! let v = parse(&d).unwrap();
//! assert_eq!(header(&d, 0).unwrap().sequence, 0);
//! assert_eq!(active(&d, &v).unwrap().sequence, 1);
//! ```

/// Where the two log headers live (64 KiB apart, starting at 64 KiB).
pub const HEADER_BASE: usize = 65536;
/// Distance between header 0 and header 1.
pub const HEADER_STRIDE: usize = 65536;

/// A parsed file signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vhdx {
    /// UTF-16LE creator field (raw bytes, up to a NUL).
    pub creator: Vec<u8>,
}

/// One `head` log header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Monotonic sequence; highest wins as active.
    pub sequence: u64,
    /// Header format version (1).
    pub version: u16,
    /// Log area byte length.
    pub log_length: u32,
    /// Log area offset.
    pub log_offset: u64,
}

fn u16l(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32l(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64l(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Parse the file signature + creator.
pub fn parse(d: &[u8]) -> Option<Vhdx> {
    if d.get(..8)? != b"vhdxfile" {
        return None;
    }
    let raw = d.get(8..8 + 512)?;
    let end = raw
        .chunks_exact(2)
        .position(|w| w == [0, 0])
        .map(|p| p * 2)
        .unwrap_or(raw.len());
    Some(Vhdx {
        creator: raw[..end].to_vec(),
    })
}

/// Read header `i` (0 or 1); `None` when its `head` magic is absent.
pub fn header(d: &[u8], i: usize) -> Option<Header> {
    let at = HEADER_BASE.checked_add(i.checked_mul(HEADER_STRIDE)?)?;
    if d.get(at..at + 4)? != b"head" {
        return None;
    }
    Some(Header {
        sequence: u64l(d, at + 8)?,
        version: u16l(d, at + 18)?,
        log_length: u32l(d, at + 20)?,
        log_offset: u64l(d, at + 24)?,
    })
}

/// The active header = the highest-sequence valid one. `None` when
/// neither header parses.
pub fn active(d: &[u8], _sig: &Vhdx) -> Option<Header> {
    let a = header(d, 0);
    let b = header(d, 1);
    match (a, b) {
        (Some(a), Some(b)) => Some(if b.sequence >= a.sequence { b } else { a }),
        (x, None) | (None, x) => x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 3 * 65536];
        d[..8].copy_from_slice(b"vhdxfile");
        d[8..18].copy_from_slice(&[0x44, 0, 0x65, 0, 0x76, 0, 0x69, 0, 0x6E, 0]); // "Devin"
        for i in 0..2 {
            let h = (1 + i) * 65536;
            d[h..h + 4].copy_from_slice(b"head");
            d[h + 8..h + 16].copy_from_slice(&(i as u64).to_le_bytes());
            d[h + 18..h + 20].copy_from_slice(&1u16.to_le_bytes());
            d[h + 20..h + 24].copy_from_slice(&4096u32.to_le_bytes());
            d[h + 24..h + 32].copy_from_slice(&(0x300000u64 + i as u64).to_le_bytes());
        }
        d
    }

    #[test]
    fn signature_and_headers() {
        let d = fixture();
        let v = parse(&d).unwrap();
        assert_eq!(v.creator.len(), 10); // 5 UTF-16 units
        let h0 = header(&d, 0).unwrap();
        assert_eq!(h0.sequence, 0);
        assert_eq!(h0.version, 1);
        assert_eq!(h0.log_length, 4096);
        let act = active(&d, &v).unwrap();
        assert_eq!(act.sequence, 1);
        assert_eq!(act.log_offset, 0x300001);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        // both headers corrupt → no active
        let mut d2 = fixture();
        d2[HEADER_BASE] = 0;
        d2[HEADER_BASE + HEADER_STRIDE] = 0;
        assert!(header(&d2, 0).is_none());
        assert!(active(&d2, &parse(&d2).unwrap()).is_none());
    }
}

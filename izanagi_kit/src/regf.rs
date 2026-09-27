//! Windows registry hive (`regf`) — the NT hive format documented
//! by the libyal `winreg-kb` spec and the Samba `regf` reader.
//!
//! Layout: 4096-byte header (`regf` magic, primary/secondary
//! sequence numbers at +4/+8, major/minor version, root-cell offset
//! at +36 relative to the first `hbin` data area at file offset
//! `0x1000 + 32`), then 4096-aligned `hbin` blocks
//! (`hbin` magic, file offset, size), each holding cells with a
//! signed i32 size — negative means allocated.
//!
//! ```
//! use izanagi_kit::regf::{parse, hbins, cells, root_cell_abs};
//! let mut d = vec![0u8; 0x2000];
//! d[0..4].copy_from_slice(b"regf");
//! d[4..8].copy_from_slice(&1u32.to_le_bytes()); // primary seq
//! d[8..12].copy_from_slice(&1u32.to_le_bytes());
//! d[20..24].copy_from_slice(&1u32.to_le_bytes()); // version 1.x
//! d[36..40].copy_from_slice(&0u32.to_le_bytes()); // root cell rel
//! d[0x1000..0x1004].copy_from_slice(b"hbin");
//! d[0x1008..0x100C].copy_from_slice(&0x1000u32.to_le_bytes()); // size
//! // first cell: -32 (allocated), id "nk"
//! let sz = (-32i32).to_le_bytes();
//! d[0x1020..0x1024].copy_from_slice(&sz);
//! d[0x1024..0x1026].copy_from_slice(b"nk");
//! let r = parse(&d).unwrap();
//! assert_eq!(hbins(&d).len(), 1);
//! assert_eq!(cells(&d, 0x1000).len(), 1); // one allocated cell
//! assert_eq!(root_cell_abs(&r), 0x1020);
//! ```

/// The file offset where hbin 0 begins.
pub const HBIN_BASE: usize = 0x1000;
/// hbin header size; cell offsets are relative to `HBIN_BASE + 32`.
pub const HBIN_HDR: usize = 32;

/// Parsed hive header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Regf {
    /// Major version (`u32` at +20).
    pub major: u32,
    /// Minor version (`u32` at +24).
    pub minor: u32,
    /// Root-cell offset relative to `HBIN_BASE + HBIN_HDR`.
    pub root_rel: u32,
    /// Declared hive size (`u32` at +40).
    pub hive_size: u32,
    /// `true` when primary/secondary sequence numbers match.
    pub seq_ok: bool,
}

/// One `hbin` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hbin {
    /// File offset.
    pub at: usize,
    /// Byte size (multiple of 4096).
    pub len: usize,
}

/// One cell inside an hbin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// File offset of the cell (its i32 size field).
    pub at: usize,
    /// Cell byte size (absolute value of the signed size).
    pub len: usize,
    /// `true` when the size was negative (allocated).
    pub allocated: bool,
}

fn u32s(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn i32s(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Parse the `regf` header. `None` on wrong magic, truncated
/// header, or an empty hive.
pub fn parse(d: &[u8]) -> Option<Regf> {
    if d.get(0..4)? != b"regf" {
        return None;
    }
    if d.len() < HBIN_BASE {
        return None;
    }
    Some(Regf {
        seq_ok: u32s(d, 4)? == u32s(d, 8)?,
        major: u32s(d, 20)?,
        minor: u32s(d, 24)?,
        root_rel: u32s(d, 36)?,
        hive_size: u32s(d, 40)?,
    })
}

/// Absolute file offset of the root cell.
pub fn root_cell_abs(r: &Regf) -> usize {
    HBIN_BASE + HBIN_HDR + r.root_rel as usize
}

/// Walk `hbin` blocks starting at `HBIN_BASE`, stopping at the
/// first malformed or misaligned header.
pub fn hbins(d: &[u8]) -> Vec<Hbin> {
    let mut out = Vec::new();
    let mut at = HBIN_BASE;
    while at + HBIN_HDR <= d.len() {
        if d.get(at..at + 4) != Some(b"hbin") {
            break;
        }
        let len = match u32s(d, at + 8) {
            Some(n) if n as usize >= HBIN_HDR && n % 0x1000 == 0 => n as usize,
            _ => break,
        };
        if at + len > d.len() {
            break;
        }
        out.push(Hbin { at, len });
        at += len;
    }
    out
}

/// Walk cells inside the hbin at `hbin_at`. Sizes are signed i32;
/// the walk stops at a zero or non-multiple-of-8 remainder.
pub fn cells(d: &[u8], hbin_at: usize) -> Vec<Cell> {
    let mut out = Vec::new();
    let hbin_len = match u32s(d, hbin_at + 8) {
        Some(n) => n as usize,
        None => return out,
    };
    let mut at = hbin_at + HBIN_HDR;
    let end = (hbin_at + hbin_len).min(d.len());
    while at + 4 <= end {
        let sz = match i32s(d, at) {
            Some(s) if s != 0 => s,
            _ => break,
        };
        let len = sz.unsigned_abs() as usize;
        if len < 4 || len % 8 != 0 || at + len > end {
            break;
        }
        out.push(Cell {
            at,
            len,
            allocated: sz < 0,
        });
        at += len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 0x3000];
        d[0..4].copy_from_slice(b"regf");
        d[4..8].copy_from_slice(&7u32.to_le_bytes());
        d[8..12].copy_from_slice(&7u32.to_le_bytes());
        d[20..24].copy_from_slice(&1u32.to_le_bytes());
        d[24..28].copy_from_slice(&5u32.to_le_bytes());
        d[36..40].copy_from_slice(&32u32.to_le_bytes());
        d[40..44].copy_from_slice(&0x2000u32.to_le_bytes());
        // hbin 0
        d[0x1000..0x1004].copy_from_slice(b"hbin");
        d[0x1008..0x100C].copy_from_slice(&0x1000u32.to_le_bytes());
        // allocated nk cell then a free cell
        d[0x1020..0x1024].copy_from_slice(&(-32i32).to_le_bytes());
        d[0x1024..0x1026].copy_from_slice(b"nk");
        d[0x1040..0x1044].copy_from_slice(&32i32.to_le_bytes());
        // hbin 1
        d[0x2000..0x2004].copy_from_slice(b"hbin");
        d[0x2008..0x200C].copy_from_slice(&0x1000u32.to_le_bytes());
        d
    }

    #[test]
    fn header_and_bins() {
        let d = fixture();
        let r = parse(&d).unwrap();
        assert!(r.seq_ok);
        assert_eq!((r.major, r.minor), (1, 5));
        assert_eq!(r.hive_size, 0x2000);
        assert_eq!(root_cell_abs(&r), 0x1020 + 32);
        let hs = hbins(&d);
        assert_eq!(hs.len(), 2);
        assert_eq!(hs[0].at, 0x1000);
        assert_eq!(hs[1].at, 0x2000);
        let cs = cells(&d, hs[0].at);
        assert_eq!(cs.len(), 2);
        assert!(cs[0].allocated);
        assert!(!cs[1].allocated);
        assert_eq!(cs[0].at, 0x1020);
    }

    #[test]
    fn seq_mismatch() {
        let mut d = fixture();
        d[8..12].copy_from_slice(&9u32.to_le_bytes());
        assert!(!parse(&d).unwrap().seq_ok);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2.truncate(0x1000); // header only
        assert!(hbins(&d2).is_empty());
        // corrupt cell chain: odd size
        let mut d3 = fixture();
        d3[0x1020..0x1024].copy_from_slice(&(-31i32).to_le_bytes());
        assert!(cells(&d3, 0x1000).is_empty());
    }
}

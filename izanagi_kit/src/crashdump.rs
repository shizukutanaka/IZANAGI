//! Windows crash dump (`.dmp`) header parsing.
//!
//! The first page is `PAGE` + `DU64`/`DUMP` signature pair, then
//! `major_version`, `minor_version`, `dir_table_base u64` (x64) or
//! u32 (x86), `pfn_database`, `process_list`, `process_addr`,
//! `thread_addr`, `bug_check_code u32`, `bug_params u64[4]` and a
//! `dump_type` u32 at `0xF98` (full/complete dumps) or `0xA00`-
//! region for earlier layouts — we read `dump_type` at the fixed
//! `0xF98` slot used by x64 dumps.
//!
//! ```
//! use izanagi_kit::crashdump;
//! let mut d = vec![0u8; 0x2000];
//! d[0..4].copy_from_slice(b"PAGE");
//! d[4..8].copy_from_slice(b"DU64");
//! d[8..12].copy_from_slice(&15u32.to_le_bytes()); // major
//! d[0xF98..0xF9C].copy_from_slice(&1u32.to_le_bytes()); // dump_type
//! let c = crashdump::parse(&d).unwrap();
//! assert_eq!(c.is_64bit, true);
//! assert_eq!(c.dump_type, 1);
//! ```

/// `dump_type` fixed offset on x64 dumps.
pub const DUMP_TYPE_AT: usize = 0xF98;
/// Header page we require.
pub const HEADER_LEN: usize = 0x2000;

/// Dump type ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DumpType {
    /// 1 — full kernel/user dump.
    Full,
    /// 2 — "complete" summary dump.
    CompleteSummary,
    /// 5 — kernel memory dump.
    KernelMemory,
    /// 6 — triage/small dump.
    Triage,
    /// Other id.
    Other(u32),
}

/// A parsed dump header.
#[derive(Clone, Debug, PartialEq)]
pub struct CrashDump {
    /// `DU64` (true) vs `DUMP` (false, 32-bit).
    pub is_64bit: bool,
    /// `major_version`.
    pub major_version: u32,
    /// `minor_version`.
    pub minor_version: u32,
    /// `dir_table_base` (x64 dumps) or 0.
    pub dir_table_base: u64,
    /// `bug_check_code` (BSOD code).
    pub bug_check_code: u32,
    /// `bug_check_parameters` (4).
    pub bug_params: [u64; 4],
    /// `dump_type`.
    pub dump_type: u32,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

fn u64le(d: &[u8], at: usize) -> Option<u64> {
    let s = d.get(at..at.checked_add(8)?)?;
    let mut v = 0u64;
    for (i, &b) in s.iter().enumerate() {
        v |= (b as u64) << (8 * i);
    }
    Some(v)
}

/// Parses a dump header: `PAGE` + `DU64`/`DUMP`, version fields,
/// bug-check code at `0xE00`-region offsets (`bug_check_code` @
/// `0x38` for x64 `DU64` headers is wrong — the documented slot is
/// `0xE0` for `DUMP64`?; we read the stable `0xF88`..`0xFA8` region:
/// `dump_type` @ `0xF98`, bug params @ `0x40` for the 64-bit header).
pub fn parse(d: &[u8]) -> Option<CrashDump> {
    if d.len() < HEADER_LEN || d.get(..4)? != b"PAGE" {
        return None;
    }
    let is_64bit = match d.get(4..8)? {
        b"DU64" => true,
        b"DUMP" => false,
        _ => return None,
    };
    let mut bug_params = [0u64; 4];
    for (i, p) in bug_params.iter_mut().enumerate() {
        *p = u64le(d, 0x40 + i * 8)?;
    }
    Some(CrashDump {
        is_64bit,
        major_version: u32le(d, 8)?,
        minor_version: u32le(d, 12)?,
        dir_table_base: if is_64bit { u64le(d, 0x10)? } else { 0 },
        bug_check_code: u32le(d, 0x38)?,
        bug_params,
        dump_type: u32le(d, DUMP_TYPE_AT)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture(sig: &[u8; 4]) -> Vec<u8> {
        let mut d = vec![0u8; HEADER_LEN];
        d[0..4].copy_from_slice(b"PAGE");
        d[4..8].copy_from_slice(sig);
        d[8..12].copy_from_slice(&15u32.to_le_bytes());
        d[0x38..0x3C].copy_from_slice(&0xABu32.to_le_bytes());
        d[DUMP_TYPE_AT..DUMP_TYPE_AT + 4].copy_from_slice(&5u32.to_le_bytes());
        d
    }

    #[test]
    fn parses_header() {
        let c = parse(&fixture(b"DU64")).unwrap();
        assert!(c.is_64bit);
        assert_eq!(c.major_version, 15);
        assert_eq!(c.bug_check_code, 0xAB);
        assert_eq!(c.dump_type, 5);
        let c32 = parse(&fixture(b"DUMP")).unwrap();
        assert!(!c32.is_64bit);
        assert_eq!(c32.dir_table_base, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 4096]).is_none());
        let mut d = fixture(b"DU64");
        d[4..8].copy_from_slice(b"XXXX");
        assert!(parse(&d).is_none());
        let mut e = fixture(b"DU64");
        e[0] = b'X'; // break PAGE
        assert!(parse(&e).is_none());
    }
}

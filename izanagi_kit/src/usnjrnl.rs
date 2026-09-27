//! NTFS USN journal (`$Extend\$UsnJrnl:$J`) record parsing.
//!
//! A v2/v3 record is `record_len u32LE`, `major u16`, `minor u16`,
//! `file_ref u64`, `parent_ref u64`, `usn i64`, `timestamp i64`
//! (FILETIME), `reason u32`, `source_info u32`, `security_id u32`,
//! `file_attr u32`, `name_len u16`, `name_offset u16`, then a
//! UTF-16LE name; v3 replaces the two refs with 128-bit
//! `FileReferenceNumber`s.
//!
//! ```
//! use izanagi_kit::usnjrnl;
//! let mut d = vec![0u8; 62 + 12];
//! d[0..4].copy_from_slice(&74u32.to_le_bytes()); // record_len
//! d[4..6].copy_from_slice(&2u16.to_le_bytes()); // major
//! d[6..8].copy_from_slice(&0u16.to_le_bytes()); // minor
//! d[56..58].copy_from_slice(&6u16.to_le_bytes()); // name_len
//! d[58..60].copy_from_slice(&60u16.to_le_bytes()); // name_offset
//! d[60..72].copy_from_slice(&[0x61,0,0x62,0,0x63,0,0,0,0,0,0,0]); // "abc" utf16
//! let r = usnjrnl::parse_record(&d, 0).unwrap();
//! assert_eq!(r.name, "abc");
//! ```

use std::string::String;

/// Reason-code bit names stay out of scope; raw `reason` is kept.
#[derive(Clone, Debug, PartialEq)]
pub struct UsnRecord {
    /// `record_len` (includes padding to 8-byte alignment).
    pub record_len: u32,
    /// `major_version` (2 or 3).
    pub major: u16,
    /// `minor_version`.
    pub minor: u16,
    /// `file_reference` (low 8 bytes; v3 uses 16).
    pub file_ref: u64,
    /// `parent_file_reference`.
    pub parent_ref: u64,
    /// `usn`.
    pub usn: i64,
    /// `timestamp` FILETIME (100-ns since 1601).
    pub timestamp: i64,
    /// `reason` bitmask.
    pub reason: u32,
    /// `file_attributes`.
    pub file_attr: u32,
    /// UTF-16 file name (lossy decode).
    pub name: String,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) | (s[1] as u16) << 8)
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

fn i64le(d: &[u8], at: usize) -> Option<i64> {
    Some(u64le(d, at)? as i64)
}

fn utf16(d: &[u8]) -> String {
    let mut s = String::new();
    for c in d.chunks(2) {
        let v = (c[0] as u16) | ((c.get(1).copied().unwrap_or(0) as u16) << 8);
        if v == 0 {
            break;
        }
        s.push(char::from_u32(v as u32).unwrap_or('\u{FFFD}'));
    }
    s
}

/// Parses one journal record at `at`: version 2 or 3, length and
/// name window bounds-checked.
pub fn parse_record(d: &[u8], at: usize) -> Option<UsnRecord> {
    let record_len = u32le(d, at)?;
    let major = u16le(d, at + 4)?;
    let minor = u16le(d, at + 6)?;
    if !(2..=3).contains(&major) || record_len < 60 {
        return None;
    }
    if (at as u64 + record_len as u64) > d.len() as u64 {
        return None;
    }
    let end = at + record_len as usize;
    let (file_ref, parent_ref) = if major == 3 {
        (u64le(d, at + 8)?, u64le(d, at + 24)?)
    } else {
        (u64le(d, at + 8)?, u64le(d, at + 16)?)
    };
    // v2 layout: refs @8/@16, usn @24, ts @32, reason @40, src @44,
    // sid @48, attr @52, name_len @56, name_off @58.
    // v3: refs @8(16B)/@24(16B), usn @40, ts @48, reason @56,
    // src @60, sid @64, attr @68, name_len @72, name_off @74.
    let (usn_at, base) = if major == 3 {
        (at + 40, at + 56)
    } else {
        (at + 24, at + 40)
    };
    let usn = i64le(d, usn_at)?;
    let timestamp = i64le(d, usn_at + 8)?;
    let reason = u32le(d, base)?;
    let file_attr = u32le(d, base + 12)?;
    let name_len = u16le(d, base + 16)? as usize;
    let name_off = u16le(d, base + 18)? as usize;
    let name_start = at + name_off;
    if name_len > 0 {
        if name_start.checked_add(name_len)? > end {
            return None;
        }
        if name_len % 2 != 0 {
            return None;
        }
    }
    let name = utf16(d.get(name_start..name_start + name_len)?);
    Some(UsnRecord {
        record_len,
        major,
        minor,
        file_ref,
        parent_ref,
        usn,
        timestamp,
        reason,
        file_attr,
        name,
    })
}

/// True when a v2/v3 record parses at the given offset.
pub fn is_usn_record(d: &[u8], at: usize) -> bool {
    parse_record(d, at).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture_v2() -> Vec<u8> {
        let mut d = vec![0u8; 72];
        d[0..4].copy_from_slice(&72u32.to_le_bytes());
        d[4..6].copy_from_slice(&2u16.to_le_bytes());
        d[24..32].copy_from_slice(&0x1122i64.to_le_bytes()); // usn
        d[40..44].copy_from_slice(&0x80000200u32.to_le_bytes()); // reason
        d[56..58].copy_from_slice(&6u16.to_le_bytes()); // name_len 6
        d[58..60].copy_from_slice(&60u16.to_le_bytes());
        d[60..66].copy_from_slice(&[b'f', 0, b'o', 0, b'o', 0]);
        d
    }

    #[test]
    fn parses_v2() {
        let r = parse_record(&fixture_v2(), 0).unwrap();
        assert_eq!(r.major, 2);
        assert_eq!(r.usn, 0x1122);
        assert_eq!(r.name, "foo");
        assert!(is_usn_record(&fixture_v2(), 0));
    }

    #[test]
    fn rejects() {
        assert!(parse_record(&[0u8; 64], 0).is_none());
        let mut d = fixture_v2();
        d[4..6].copy_from_slice(&9u16.to_le_bytes()); // major 9
        assert!(parse_record(&d, 0).is_none());
        let mut d = fixture_v2();
        d[56..58].copy_from_slice(&600u16.to_le_bytes()); // name overflows
        assert!(parse_record(&d, 0).is_none());
    }
}

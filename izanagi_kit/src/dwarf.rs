//! DWARF debug-info section parsing (compilation-unit headers only).
//!
//! A `.debug_info` stream is a sequence of compilation units:
//! `unit_length` u32LE (or `0xFFFFFFFF` + u64LE for DWARF64),
//! `version` u16, then v4 `abbrev_offset` + `address_size` or
//! v5 `unit_type` u8 + `address_size` u8 + `abbrev_offset` u32.
//! Dies/form codes are left to the abbrev layer — out of scope.
//!
//! ```
//! use izanagi_kit::dwarf;
//! let mut d = Vec::new();
//! d.extend_from_slice(&16u32.to_le_bytes()); // unit_length (2+4+1+9)
//! d.extend_from_slice(&4u16.to_le_bytes()); // version 4
//! d.extend_from_slice(&0u32.to_le_bytes()); // abbrev_offset
//! d.push(8); // address_size
//! d.extend_from_slice(&[0u8; 9]); // die data
//! let units = dwarf::parse(&d).unwrap();
//! assert_eq!(units.len(), 1);
//! assert_eq!(units[0].version, 4);
//! ```

use std::vec::Vec;

/// `unit_length` sentinel selecting DWARF64.
pub const DWARF64_SENTINEL: u32 = 0xFFFF_FFFF;

/// A compilation-unit header.
#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    /// Declared `unit_length` (excluding the 4-byte field itself).
    pub length: u64,
    /// DWARF version (2–5 seen in practice).
    pub version: u16,
    /// v5 only: `unit_type` (`DW_UT_compile` = 0x01, ...).
    pub unit_type: u8,
    /// `address_size` (typically 4 or 8).
    pub address_size: u8,
    /// `abbrev_offset` into `.debug_abbrev`.
    pub abbrev_offset: u64,
    /// Byte offset where this unit's DIE data begins.
    pub data_offset: usize,
    /// Byte offset just past this unit (`header + length`).
    pub end: usize,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | (*d.get(at + 1)? as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

fn u64le(d: &[u8], at: usize) -> Option<u64> {
    Some(u32le(d, at)? as u64 | (u32le(d, at + 4)? as u64) << 32)
}

/// Parses a `.debug_info` blob as a sequence of compilation units.
/// Units must tile the input exactly.
pub fn parse(d: &[u8]) -> Option<Vec<Unit>> {
    if d.is_empty() {
        return None;
    }
    let mut at = 0usize;
    let mut out = Vec::new();
    while at < d.len() {
        let length32 = u32le(d, at)?;
        let (length, mut p);
        if length32 == DWARF64_SENTINEL {
            length = u64le(d, at + 4)?;
            p = at + 12;
        } else {
            length = length32 as u64;
            p = at + 4;
        }
        let end = p.checked_add(length as usize)?;
        if end > d.len() || end <= p {
            return None;
        }
        let version = u16le(d, p)?;
        p += 2;
        let (unit_type, address_size, abbrev_offset);
        if version >= 5 {
            // v5: unit_type u8, address_size u8, abbrev u32/u64
            unit_type = *d.get(p)?;
            address_size = *d.get(p + 1)?;
            p += 2;
            abbrev_offset = if length32 == DWARF64_SENTINEL {
                let v = u64le(d, p)?;
                p += 8;
                v
            } else {
                let v = u32le(d, p)? as u64;
                p += 4;
                v
            };
        } else {
            abbrev_offset = if length32 == DWARF64_SENTINEL {
                let v = u64le(d, p)?;
                p += 8;
                v
            } else {
                let v = u32le(d, p)? as u64;
                p += 4;
                v
            };
            unit_type = 0;
            address_size = *d.get(p)?;
            p += 1;
        }
        if p > end {
            return None;
        }
        out.push(Unit {
            length,
            version,
            unit_type,
            address_size,
            abbrev_offset,
            data_offset: p,
            end,
        });
        at = end;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v4_unit(version: u16, body: usize) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&(2 + 4 + 1 + body as u32).to_le_bytes());
        d.extend_from_slice(&version.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.push(8);
        d.extend_from_slice(&std::vec![0u8; body]);
        d
    }

    #[test]
    fn parses_units() {
        let mut d = v4_unit(4, 3);
        d.extend_from_slice(&v4_unit(3, 5));
        let u = parse(&d).unwrap();
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].version, 4);
        assert_eq!(u[0].address_size, 8);
        assert_eq!(u[1].version, 3);
        assert_eq!(u[1].end, d.len());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut d = v4_unit(4, 3);
        d[0] = 0xFF; // huge length → out of bounds
        assert!(parse(&d).is_none());
    }
}

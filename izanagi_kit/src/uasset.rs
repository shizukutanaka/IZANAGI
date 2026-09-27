//! Unreal Engine 4 package (`.uasset`) header parsing.
//!
//! `FPackageFileSummary`: package tag u32LE `0x9E2A83C1` (bytes
//! `C1 83 2A 9E`), then `legacy_version_ue4`/`ue3_version`/
//! `file_version`/`licensee_version` (i32LE), then
//! `custom_version_count` u32 + `count × (guid 16B + version i32)`
//! custom-version records (format 2). Later FString/name-table fields
//! are version-dependent and left opaque.
//!
//! ```
//! use izanagi_kit::uasset;
//! let mut d = 0x9E2A83C1u32.to_le_bytes().to_vec();
//! d.extend_from_slice(&(-7i32).to_le_bytes()); // legacy_version_ue4
//! d.extend_from_slice(&0i32.to_le_bytes()); // ue3
//! d.extend_from_slice(&522i32.to_le_bytes()); // file_version_ue4
//! d.extend_from_slice(&0i32.to_le_bytes()); // licensee
//! d.extend_from_slice(&0u32.to_le_bytes()); // custom_version_count
//! let u = uasset::parse(&d).unwrap();
//! assert_eq!(u.file_version, 522);
//! ```

use std::vec::Vec;

/// Package tag (little-endian on disk).
pub const TAG: u32 = 0x9E2A_83C1;

/// A custom version record (`FCustomVersion` format 2).
#[derive(Clone, Debug, PartialEq)]
pub struct CustomVersion {
    /// 16-byte GUID.
    pub guid: [u8; 16],
    /// Version number.
    pub version: i32,
}

/// A parsed `.uasset` summary header.
#[derive(Clone, Debug, PartialEq)]
pub struct Uasset {
    /// `legacy_version_ue4` (negative for UE4-era packages).
    pub legacy_version: i32,
    /// `legacy_version_ue3`.
    pub ue3_version: i32,
    /// `file_version_ue4` (the UE4 serialization version).
    pub file_version: i32,
    /// `file_version_licensee_ue4`.
    pub licensee_version: i32,
    /// Custom version records (format 2: guid + i32).
    pub custom_versions: Vec<CustomVersion>,
    /// Byte offset past the custom-version table.
    pub end_offset: usize,
}

fn i32le(d: &[u8], at: usize) -> Option<i32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24) as i32)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses the package tag + version fields + custom-version table.
pub fn parse(d: &[u8]) -> Option<Uasset> {
    if d.len() < 20 {
        return None;
    }
    if u32le(d, 0)? != TAG {
        return None;
    }
    let legacy_version = i32le(d, 4)?;
    let ue3_version = i32le(d, 8)?;
    let file_version = i32le(d, 12)?;
    let licensee_version = i32le(d, 16)?;
    let count = u32le(d, 20)? as usize;
    let mut at: usize = 24;
    let need = count.checked_mul(20)?;
    if at.checked_add(need)? > d.len() || count > 0x10000 {
        return None;
    }
    let mut custom_versions = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let mut guid = [0u8; 16];
        guid.copy_from_slice(d.get(at..at + 16)?);
        at += 16;
        let version = i32le(d, at)?;
        at += 4;
        custom_versions.push(CustomVersion { guid, version });
    }
    Some(Uasset {
        legacy_version,
        ue3_version,
        file_version,
        licensee_version,
        custom_versions,
        end_offset: at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = TAG.to_le_bytes().to_vec();
        d.extend_from_slice(&(-7i32).to_le_bytes());
        d.extend_from_slice(&0i32.to_le_bytes());
        d.extend_from_slice(&522i32.to_le_bytes());
        d.extend_from_slice(&0i32.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes()); // 1 custom version
        d.extend_from_slice(&[0xAB; 16]);
        d.extend_from_slice(&3i32.to_le_bytes());
        d.extend_from_slice(&[0u8; 32]); // rest of summary
        d
    }

    #[test]
    fn parses_summary() {
        let u = parse(&fixture()).unwrap();
        assert_eq!(u.legacy_version, -7);
        assert_eq!(u.file_version, 522);
        assert_eq!(u.custom_versions.len(), 1);
        assert_eq!(u.custom_versions[0].version, 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 10]).is_none());
        let mut d = fixture();
        d[0] = 0xFF;
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[20..24].copy_from_slice(&999u32.to_le_bytes()); // too many versions
        assert!(parse(&d).is_none());
    }
}

//! Windows Prefetch `.pf` header — the SCCA layout documented by
//! the libyal `libscca` spec (versions 17/23/26/30/31).
//!
//! Layout: `u32 version` (17 = XP, 23 = Vista/7, 26 = 8.1,
//! 30 = 10, 31 = 11), `SCCA` signature at +4, u32 file size at +12,
//! a 60-byte UTF-16LE executable name at +16, a hash at +76
//! (v17/23) or +80 (v26+), and a version-dependent run-count
//! offset. Windows 10+ files are often MAM-compressed — those
//! start `MAM\x04` instead and are reported, not decoded.
//!
//! ```
//! use izanagi_kit::prefetch::{parse, name, run_count, Version};
//! let mut d = vec![0u8; 0x400];
//! d[0..4].copy_from_slice(&23u32.to_le_bytes()); // Win7
//! d[4..8].copy_from_slice(b"SCCA");
//! d[12..16].copy_from_slice(&0x400u32.to_le_bytes());
//! // UTF-16LE "APP.EXE"
//! for (i, ch) in b"APP.EXE".iter().enumerate() {
//!     d[16 + i * 2] = *ch;
//! }
//! d[76..80].copy_from_slice(&0xDEADu32.to_le_bytes()); // hash
//! d[0x98..0x9C].copy_from_slice(&3u32.to_le_bytes()); // runs
//! let p = parse(&d).unwrap();
//! assert_eq!(p.version, Version::V23);
//! assert_eq!(name(&d).as_slice(), b"APP.EXE");
//! assert_eq!(run_count(&d, &p), Some(3));
//! ```

/// Known `.pf` versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// 17 — Windows XP.
    V17,
    /// 23 — Windows Vista / 7.
    V23,
    /// 26 — Windows 8.1.
    V26,
    /// 30 — Windows 10.
    V30,
    /// 31 — Windows 11.
    V31,
    /// Any other version number.
    Other(u32),
}

/// Parsed prefetch header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prefetch {
    /// Format version.
    pub version: Version,
    /// Declared total file size (u32 at +12).
    pub file_size: u32,
    /// `true` when the file is MAM-compressed (`MAM\x04` magic).
    pub compressed: bool,
}

fn u32s(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Version id → enum.
pub fn version_of(v: u32) -> Version {
    match v {
        17 => Version::V17,
        23 => Version::V23,
        26 => Version::V26,
        30 => Version::V30,
        31 => Version::V31,
        n => Version::Other(n),
    }
}

/// Parse the header. `None` unless the file starts `MAM\x04`
/// (compressed — returned with `compressed` set) or carries the
/// `SCCA` signature.
pub fn parse(d: &[u8]) -> Option<Prefetch> {
    if d.get(0..4)? == b"MAM\x04" {
        return Some(Prefetch {
            version: Version::Other(0),
            file_size: 0,
            compressed: true,
        });
    }
    if d.get(4..8)? != b"SCCA" {
        return None;
    }
    Some(Prefetch {
        version: version_of(u32s(d, 0)?),
        file_size: u32s(d, 12)?,
        compressed: false,
    })
}

/// Executable name: UTF-16LE, up to 29 chars + NUL at +16,
/// decoded to lossy ASCII (non-ASCII units become `?`).
pub fn name(d: &[u8]) -> Vec<u8> {
    let raw = d.get(16..76).unwrap_or(&[]);
    let mut out = Vec::new();
    for pair in raw.chunks_exact(2) {
        if pair[0] == 0 && pair[1] == 0 {
            break;
        }
        if out.len() < 30 {
            out.push(if pair[1] == 0 { pair[0] } else { b'?' });
        }
    }
    out
}

/// Prefetch hash (u32): at +76 for v17/v23, +80 for v26+.
pub fn hash(d: &[u8], p: &Prefetch) -> Option<u32> {
    let at = match p.version {
        Version::V17 | Version::V23 => 76,
        _ => 80,
    };
    u32s(d, at)
}

/// Run count: +0x90 (v17), +0x98 (v23), +0xD0 (v26/30/31).
/// `None` for compressed or unknown versions.
pub fn run_count(d: &[u8], p: &Prefetch) -> Option<u32> {
    let at = match p.version {
        Version::V17 => 0x90,
        Version::V23 => 0x98,
        Version::V26 | Version::V30 | Version::V31 => 0xD0,
        Version::Other(_) => return None,
    };
    u32s(d, at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(ver: u32) -> Vec<u8> {
        let mut d = vec![0u8; 0x400];
        d[0..4].copy_from_slice(&ver.to_le_bytes());
        d[4..8].copy_from_slice(b"SCCA");
        d[12..16].copy_from_slice(&0x400u32.to_le_bytes());
        for (i, ch) in b"SVCHOST.EXE".iter().enumerate() {
            d[16 + i * 2] = *ch;
        }
        let (hat, rat) = match ver {
            17 => (76, 0x90),
            23 => (76, 0x98),
            _ => (80, 0xD0),
        };
        d[hat..hat + 4].copy_from_slice(&0xBEEFu32.to_le_bytes());
        d[rat..rat + 4].copy_from_slice(&9u32.to_le_bytes());
        d
    }

    #[test]
    fn versions_and_fields() {
        for (ver, want) in [
            (17, Version::V17),
            (23, Version::V23),
            (26, Version::V26),
            (30, Version::V30),
            (31, Version::V31),
            (99, Version::Other(99)),
        ] {
            let d = fixture(ver);
            let p = parse(&d).unwrap();
            assert_eq!(p.version, want);
            assert_eq!(version_of(ver), want);
            assert_eq!(p.file_size, 0x400);
            assert!(!p.compressed);
            assert_eq!(name(&d).as_slice(), b"SVCHOST.EXE");
        }
        let d = fixture(17);
        let p = parse(&d).unwrap();
        assert_eq!(hash(&d, &p), Some(0xBEEF));
        assert_eq!(run_count(&d, &p), Some(9));
        let d = fixture(30);
        let p = parse(&d).unwrap();
        assert_eq!(run_count(&d, &p), Some(9));
        assert_eq!(hash(&d, &p), Some(0xBEEF));
    }

    #[test]
    fn compressed_marker() {
        let mut d = vec![0u8; 16];
        d[0..4].copy_from_slice(b"MAM\x04");
        let p = parse(&d).unwrap();
        assert!(p.compressed);
        assert_eq!(run_count(&d, &p), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture(23);
        d[4] = b'X';
        assert!(parse(&d).is_none());
    }
}

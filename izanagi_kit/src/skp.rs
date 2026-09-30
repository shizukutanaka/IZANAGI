//! SketchUp `.skp` — Trimble's binary model container. The file opens
//! with the ASCII banner `SketchUp Model` followed by a version tag
//! and the model's geometry/type sections.
//!
//! `parse` requires the banner and extracts the version dword at
//! `0x10` plus the banner's own length byte (`0x0E`-style ASCII size
//! field is kept raw); model payloads themselves are proprietary —
//! only the envelope is decoded.
//!
//! ```
//! let mut f = b"SketchUp Model".to_vec();
//! f.extend_from_slice(&[0u8; 2]);       // pad to 16
//! f.extend_from_slice(&[0x15, 0, 0, 0]); // version 21
//! let s = izanagi_kit::skp::parse(&f).unwrap();
//! assert_eq!(s.version_raw, 0x15);
//! assert!(izanagi_kit::skp::parse(b"not a model").is_none());
//! ```

/// Magic banner.
pub const MAGIC: &[u8; 14] = b"SketchUp Model";

/// Parsed SketchUp file summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Skp {
    /// Raw u32 little-endian version field at `0x10` (e.g. `0x15` for
    /// SketchUp 2021-era files).
    pub version_raw: u32,
    /// Total file bytes.
    pub size: usize,
}

/// Parse a `.skp`; `None` without the `SketchUp Model` banner.
pub fn parse(d: &[u8]) -> Option<Skp> {
    if d.len() < 20 || !d.starts_with(MAGIC) {
        return None;
    }
    let version_raw = u32::from_le_bytes(d[16..20].try_into().ok()?);
    Some(Skp {
        version_raw,
        size: d.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut f = MAGIC.to_vec();
        f.extend_from_slice(&[0u8; 2]);
        f.extend_from_slice(&[0x18, 0, 0, 0]);
        let s = parse(&f).unwrap();
        assert_eq!(s.version_raw, 0x18);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(MAGIC).is_none()); // too short
        assert!(parse(b"SketchUp Made!").is_none());
    }
}

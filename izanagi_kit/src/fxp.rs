//! Steinberg VST preset/chunk `.fxp` / bank `.fxb` — big-endian header:
//! `chunkMagic "CcnK"`, `byteSize`, `fxMagic` (`FxCk` regular / `FxBk` bank /
//! `FxCB`/`FxBB` chunk variants), `version`, `fxID`, `numParams`.
//!
//! ```
//! let mut d = b"CcnK".to_vec();
//! d.extend_from_slice(&[0,0,0,0x3C]); // byteSize
//! d.extend_from_slice(b"FxCk");       // regular program
//! d.extend_from_slice(&[0,0,0,1]);    // version
//! d.extend_from_slice(&[0,0,0,7]);    // fxID
//! d.extend_from_slice(&[0,0,0,1]);    // fxVersion
//! d.extend_from_slice(&[0,0,0,2]);    // numParams
//! d.extend_from_slice(b"init patch\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"); // 28B name
//! d.extend_from_slice(&[0; 8]);       // two f32 params
//! let f = izanagi_kit::fxp::parse(&d).unwrap();
//! assert_eq!(f.kind, "FxCk");
//! assert_eq!(f.params, 2);
//! assert_eq!(f.fx_id, 7);
//! assert!(izanagi_kit::fxp::detect(&d));
//! ```

/// A parsed `.fxp`/`.fxb` preset.
#[derive(Debug, Clone)]
pub struct Fxp {
    /// `fxMagic` tag: `FxCk`, `FxBk`, `FxCB`, or `FxBB`.
    pub kind: String,
    /// Header `version` field.
    pub version: u32,
    /// `fxID` plugin identifier.
    pub fx_id: u32,
    /// `fxVersion`.
    pub fx_version: u32,
    /// `numParams`.
    pub params: u32,
    /// 28-byte program name (NUL-trimmed).
    pub name: String,
    /// `true` when the kind is a bank (`FxBk`/`FxBB`).
    pub bank: bool,
    /// `true` when the kind is the opaque-chunk variant (`FxCB`/`FxBB`).
    pub chunk: bool,
    /// `byteSize` as declared.
    pub byte_size: u32,
    /// Actual trailing bytes after the fixed header (params/chunk data).
    pub payload: usize,
}

fn be32(b: &[u8], off: usize) -> u32 {
    u32::from(b[off]) << 24
        | u32::from(b[off + 1]) << 16
        | u32::from(b[off + 2]) << 8
        | u32::from(b[off + 3])
}

/// Detects `.fxp`/`.fxb`: `CcnK` magic + known `fxMagic`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 28
        && b.starts_with(b"CcnK")
        && matches!(&b[8..12], b"FxCk" | b"FxBk" | b"FxCB" | b"FxBB")
}

/// Parses the preset; `None` on wrong magic or truncated header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Fxp> {
    if !detect(b) {
        return None;
    }
    let kind = std::str::from_utf8(&b[8..12]).ok()?.to_string();
    let bank = matches!(kind.as_str(), "FxBk" | "FxBB");
    let chunk = matches!(kind.as_str(), "FxCB" | "FxBB");
    let name_len = b[28..28 + 28.min(b.len().saturating_sub(28))]
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(28);
    let name =
        String::from_utf8_lossy(&b[28..28 + name_len.min(b.len().saturating_sub(28))]).to_string();
    Some(Fxp {
        kind,
        version: be32(b, 12),
        fx_id: be32(b, 16),
        fx_version: be32(b, 20),
        params: be32(b, 24),
        name,
        bank,
        chunk,
        byte_size: be32(b, 4),
        payload: b.len().saturating_sub(56),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"CcnK".to_vec();
        d.extend_from_slice(&[0, 0, 0, 0x3C]);
        d.extend_from_slice(b"FxCk");
        d.extend_from_slice(&[0, 0, 0, 1]);
        d.extend_from_slice(&[0, 0, 0, 7]);
        d.extend_from_slice(&[0, 0, 0, 1]);
        d.extend_from_slice(&[0, 0, 0, 2]);
        d.extend_from_slice(b"init\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
        d.extend_from_slice(&[0; 8]);
        d
    }

    #[test]
    fn parses() {
        let f = parse(&fixture()).unwrap();
        assert_eq!(f.kind, "FxCk");
        assert_eq!(f.version, 1);
        assert_eq!(f.fx_id, 7);
        assert_eq!(f.params, 2);
        assert_eq!(f.name, "init");
        assert!(!f.bank);
        assert!(!f.chunk);
        assert_eq!(f.payload, 8);
    }

    #[test]
    fn bank_form() {
        let mut d = fixture();
        d[8..12].copy_from_slice(b"FxBk");
        let f = parse(&d).unwrap();
        assert!(f.bank);
        assert!(detect(&d));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"CcnK").is_none());
        assert!(!detect(b"CcnK12345678XXXXrest"));
    }
}

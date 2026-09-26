//! MetaImage header (`.mhd` / `.mha`) — text `key = value` records.
//!
//! `NDims = n`, `DimSize = x y z`, `ElementType = MET_UCHAR` et al.,
//! `ElementDataFile = name.raw` (`LOCAL` in `.mha`). Unknown keys are
//! preserved in `pairs`; spacing/offset fields stay verbatim strings.
//!
//! ```
//! use izanagi_kit::mhd::parse;
//!
//! let m = parse(b"ObjectType = Image\nNDims = 3\nDimSize = 64 64 32\nElementType = MET_UCHAR\nElementDataFile = img.raw\n").unwrap();
//! assert_eq!(m.ndims, 3);
//! assert_eq!(m.dims, vec![64, 64, 32]);
//! assert_eq!(m.data_file.as_deref(), Some("img.raw"));
//! ```

/// A parsed `.mhd` header.
#[derive(Clone, Debug)]
pub struct Mhd {
    /// `NDims`.
    pub ndims: u32,
    /// `DimSize` per axis (len == ndims).
    pub dims: Vec<u32>,
    /// `ElementType` (e.g. `MET_UCHAR`, `MET_FLOAT`).
    pub element_type: Option<String>,
    /// `ElementSpacing` verbatim (per-axis floats).
    pub element_spacing: Option<String>,
    /// `ElementDataFile` — a filename or `LOCAL`.
    pub data_file: Option<String>,
    /// All `key = value` pairs in file order.
    pub pairs: Vec<(String, String)>,
}

impl Mhd {
    /// First value of `key` (case-sensitive).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Parse an `.mhd` header. `None` without `NDims` + `DimSize`, or on a
/// non-`key = value` content line.
pub fn parse(d: &[u8]) -> Option<Mhd> {
    let text = std::str::from_utf8(d).ok()?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut ndims = None;
    let mut dims = Vec::new();
    let mut element_type = None;
    let mut element_spacing = None;
    let mut data_file = None;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let (k, v) = t.split_once('=')?;
        let k = k.trim().to_string();
        let v = v.trim().to_string();
        match k.as_str() {
            "NDims" => ndims = Some(v.parse::<u32>().ok()?),
            "DimSize" => {
                dims = v
                    .split_whitespace()
                    .map(|s| s.parse::<u32>().ok())
                    .collect::<Option<Vec<_>>>()?;
            }
            "ElementType" => element_type = Some(v.clone()),
            "ElementSpacing" => element_spacing = Some(v.clone()),
            "ElementDataFile" => data_file = Some(v.clone()),
            _ => {}
        }
        pairs.push((k, v));
    }
    let ndims = ndims?;
    if dims.len() != ndims as usize {
        return None;
    }
    Some(Mhd {
        ndims,
        dims,
        element_type,
        element_spacing,
        data_file,
        pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let m = parse(b"# comment\nNDims = 2\nDimSize = 10 20\nElementType = MET_UCHAR\nElementSpacing = 0.5 0.5\nElementDataFile = LOCAL\nCustom = x\n").unwrap();
        assert_eq!(m.ndims, 2);
        assert_eq!(m.element_spacing.as_deref(), Some("0.5 0.5"));
        assert_eq!(m.get("Custom"), Some("x"));
        assert_eq!(m.get("nope"), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"DimSize = 1 2\n").is_none());
        assert!(parse(b"NDims = 3\nDimSize = 1 2\n").is_none());
        assert!(parse(b"NDims = 1\nno equals\nDimSize = 1\n").is_none());
    }
}

//! VTK legacy format (`.vtk`) — ASCII header + dataset keywords.
//!
//! Line 1 `# vtk DataFile Version x.y`, line 2 free-form title,
//! line 3 `ASCII`/`BINARY`, then `DATASET <kind>` and per-kind blocks:
//! `DIMENSIONS`, `POINTS n type`, `SCALARS`, `LOOKUP_TABLE`...
//!
//! ```
//! use izanagi_kit::vtk::parse;
//!
//! let v = parse(b"# vtk DataFile Version 3.0\ndemo\nASCII\nDATASET STRUCTURED_POINTS\nDIMENSIONS 2 3 4\n").unwrap();
//! assert_eq!(v.dataset, "STRUCTURED_POINTS");
//! assert_eq!(v.dims, Some([2, 3, 4]));
//! assert!(!v.binary);
//! ```

/// A parsed legacy `.vtk` file.
#[derive(Clone, Debug)]
pub struct Vtk {
    /// Version text from line 1 (e.g. `"3.0"`).
    pub version: String,
    /// Title line.
    pub title: String,
    /// `BINARY` mode (false = ASCII).
    pub binary: bool,
    /// `DATASET` kind (e.g. `STRUCTURED_POINTS`, `POLYDATA`).
    pub dataset: String,
    /// `DIMENSIONS` when present (structured grids).
    pub dims: Option<[u32; 3]>,
    /// `POINTS` declared count when present.
    pub points: Option<u64>,
    /// Other `KEYWORD ...` header lines (uppercased keyword, rest).
    pub keywords: Vec<(String, String)>,
}

/// Parse a legacy VTK file. `None` on a wrong banner, non-ASCII/BINARY
/// mode, missing `DATASET`, or malformed numeric fields.
pub fn parse(d: &[u8]) -> Option<Vtk> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.lines().map(str::trim);
    let banner = lines.next()?;
    let version = banner
        .strip_prefix("# vtk DataFile Version")?
        .trim()
        .to_string();
    if version.is_empty() {
        return None;
    }
    let title = lines.next()?.to_string();
    let binary = match lines.next()? {
        "ASCII" => false,
        "BINARY" => true,
        _ => return None,
    };
    let dataset_line = lines.next()?;
    let dataset = dataset_line.strip_prefix("DATASET")?.trim().to_string();
    if dataset.is_empty() {
        return None;
    }
    let mut dims = None;
    let mut points = None;
    let mut keywords = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (k, rest) = line
            .split_once(|c: char| c.is_whitespace())
            .unwrap_or((line, ""));
        let rest = rest.trim().to_string();
        let ku = k.to_uppercase();
        match ku.as_str() {
            "DIMENSIONS" => {
                let mut it = rest.split_whitespace();
                let a = it.next()?.parse().ok()?;
                let b = it.next()?.parse().ok()?;
                let c = it.next()?.parse().ok()?;
                dims = Some([a, b, c]);
            }
            "POINTS" => {
                points = Some(rest.split_whitespace().next()?.parse().ok()?);
            }
            _ => keywords.push((ku, rest)),
        }
    }
    Some(Vtk {
        version,
        title,
        binary,
        dataset,
        dims,
        points,
        keywords,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let v = parse(b"# vtk DataFile Version 3.0\nT\nBINARY\nDATASET POLYDATA\nPOINTS 8 float\nVERTICES 8 16\n").unwrap();
        assert_eq!(v.version, "3.0");
        assert!(v.binary);
        assert_eq!(v.points, Some(8));
        assert_eq!(v.keywords[0].0, "VERTICES");
        assert!(v.dims.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"# bad\nx\nASCII\nDATASET X\n").is_none());
        assert!(parse(b"# vtk DataFile Version 3.0\nx\nTEXT\n").is_none());
        assert!(parse(b"# vtk DataFile Version 3.0\nx\nASCII\nNODS\n").is_none());
    }
}

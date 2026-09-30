//! PCD — Point Cloud Library point cloud (text header + payload section).
//!
//! ```
//! let d = b"VERSION \x2e7\nFIELDS x y z\nSIZE 4 4 4\nTYPE F F F\nCOUNT 1 1 1\nWIDTH 2\nHEIGHT 1\nPOINTS 2\nDATA ascii\n0 0 0\n1 1 1\n";
//! let p = izanagi_kit::pcd::parse(d).unwrap();
//! assert_eq!(p.fields, vec!["x", "y", "z"]);
//! assert_eq!(p.points, 2);
//! assert_eq!(p.data, izanagi_kit::pcd::Data::Ascii);
//! ```

/// `DATA` payload encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Data {
    /// Whitespace-separated text rows.
    Ascii,
    /// Raw little-endian fields.
    Binary,
    /// LZF-compressed field-major block.
    BinaryCompressed,
}

/// Parsed PCD header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcd {
    /// `VERSION` value (usually `.7`), verbatim.
    pub version: String,
    /// `FIELDS` names, in order.
    pub fields: Vec<String>,
    /// `WIDTH` (row-major point count).
    pub width: u64,
    /// `HEIGHT`.
    pub height: u64,
    /// `POINTS` (explicit total; defaults to width*height when absent).
    pub points: u64,
    /// `DATA` mode.
    pub data: Data,
    /// Byte offset where the payload begins (just past the `DATA` line).
    pub data_offset: usize,
}

/// Parse a PCD file; `None` without `FIELDS` and `DATA` header lines.
pub fn parse(d: &[u8]) -> Option<Pcd> {
    let mut version = String::new();
    let mut fields = Vec::new();
    let (mut width, mut height, mut points) = (0u64, 0u64, None);
    let mut data = None;
    let mut data_offset = 0usize;
    let mut pos = 0usize;
    while pos < d.len() {
        let nl = d[pos..].iter().position(|&c| c == b'\n').map(|i| pos + i);
        let line_end = nl.unwrap_or(d.len());
        let line = std::str::from_utf8(&d[pos..line_end]).ok()?;
        let line = line.trim();
        let mut it = line.splitn(2, char::is_whitespace);
        match it.next().unwrap_or("") {
            "" | "#" => {}
            "VERSION" => version = it.next().unwrap_or("").trim().to_string(),
            "FIELDS" | "COLUMNS" => {
                fields = it
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
            }
            "WIDTH" => width = it.next().unwrap_or("").trim().parse().ok()?,
            "HEIGHT" => height = it.next().unwrap_or("").trim().parse().ok()?,
            "POINTS" => points = Some(it.next().unwrap_or("").trim().parse().ok()?),
            "DATA" => {
                data = Some(match it.next().unwrap_or("").trim() {
                    "ascii" => Data::Ascii,
                    "binary" => Data::Binary,
                    "binary_compressed" => Data::BinaryCompressed,
                    _ => return None,
                });
                data_offset = nl.map(|i| i + 1).unwrap_or(d.len());
                break;
            }
            _ => {} // SIZE / TYPE / COUNT / VIEWPOINT … kept opaque
        }
        pos = line_end + 1;
    }
    let data = data?;
    if fields.is_empty() {
        return None;
    }
    let points = points.unwrap_or(width.checked_mul(height)?);
    Some(Pcd {
        version,
        fields,
        width,
        height,
        points,
        data,
        data_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii() {
        let d = b"# comment\nVERSION .7\nFIELDS x y z intensity\nSIZE 4 4 4 4\nTYPE F F F F\nCOUNT 1 1 1 1\nWIDTH 3\nHEIGHT 1\nVIEWPOINT 0 0 0 1 0 0 0\nPOINTS 3\nDATA ascii\n0 0 0 0\n1 2 3 4\n5 6 7 8\n";
        let p = parse(d).unwrap();
        assert_eq!(p.version, ".7");
        assert_eq!(p.fields.len(), 4);
        assert_eq!(p.fields[3], "intensity");
        assert_eq!((p.width, p.height, p.points), (3, 1, 3));
        assert_eq!(p.data, Data::Ascii);
        assert!(p.data_offset > 0 && p.data_offset < d.len());
    }

    #[test]
    fn binary() {
        let d = b"VERSION .7\nFIELDS x\nSIZE 4\nTYPE F\nCOUNT 1\nWIDTH 1\nHEIGHT 2\nDATA binary\n\x00\x00\x80?";
        let p = parse(d).unwrap();
        assert_eq!(p.data, Data::Binary);
        assert_eq!(p.points, 2); // width*height fallback
        assert_eq!(&d[p.data_offset..], b"\x00\x00\x80?");
        let dc = b"VERSION .7\nFIELDS x\nDATA binary_compressed\nxx";
        assert_eq!(parse(dc).unwrap().data, Data::BinaryCompressed);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"DATA ascii\nFIELDS x\n").is_none()); // FIELDS after DATA never seen
        assert!(parse(b"FIELDS x\nDATA bogus\n").is_none()); // unknown data mode
    }
}

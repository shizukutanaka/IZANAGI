//! GXF — General eXchange Format (Geosoft-style seismic/grid
//! interchange): text header starting `#GRID`, `KEY value` lines
//! (`TITLE`, `ROWS`, `COLS`, `XORIGIN`, `YORIGIN`, `DX`, `DY`,
//! `POINTS`…), then a `#` sentinel line followed by binary doubles.
//! Grid geometry is integers; real-valued origin keys are kept as
//! text so no floating point enters the crate.
//!
//! ```
//! let d = b"#GRID\nTITLE test grid\nROWS 4\nCOLS 8\n#\n";
//! let g = izanagi_kit::gxf::parse(d).unwrap();
//! assert_eq!(g.title.as_deref(), Some("test grid"));
//! assert_eq!(g.rows, Some(4));
//! assert_eq!(g.cols, Some(8));
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed GXF header.
#[derive(Clone, Debug)]
pub struct Gxf {
    /// `TITLE` value.
    pub title: Option<String>,
    /// `ROWS` (number of rows).
    pub rows: Option<u64>,
    /// `COLS` (number of columns).
    pub cols: Option<u64>,
    /// `POINTS` (expected sample count).
    pub points: Option<u64>,
    /// All key/value lines in order (values verbatim).
    pub params: Vec<(String, String)>,
    /// Byte offset where the `#` sentinel/binary data begins.
    pub data_offset: usize,
}

/// Parse a GXF header; `None` unless the first line is `#GRID` and at
/// least one `KEY value` line follows before a `#` sentinel/EOF.
pub fn parse(d: &[u8]) -> Option<Gxf> {
    // The `#` sentinel marks the switch to binary (non-UTF-8) data —
    // find it by bytes first so the header can be decoded alone.
    let mut sent = None;
    for i in 1..d.len() {
        if d[i] == b'#'
            && d[i - 1] == b'\n'
            && (i + 1 == d.len() || d[i + 1] == b'\n' || d[i + 1] == b'\r' || d[i + 1] < 0x20)
        {
            sent = Some(i);
            break;
        }
    }
    let (head, data_offset) = match sent {
        Some(p) => (&d[..p], p),
        None => (d, d.len()),
    };
    let s = std::str::from_utf8(head).ok()?;
    let mut it = s.split('\n');
    let first = it.next()?.trim_end_matches('\r').trim();
    if first != "#GRID" {
        return None;
    }
    let mut params = Vec::new();
    let mut title = None;
    let mut rows = None;
    let mut cols = None;
    let mut points = None;
    for raw in it {
        let line = raw.trim_end_matches('\r').trim_end();
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let mut parts = t.splitn(2, |c: char| c.is_ascii_whitespace());
        let key = parts.next()?;
        let val = parts.next().map(str::trim).unwrap_or("");
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
            return None;
        }
        match key {
            "TITLE" => title = Some(val.to_string()),
            "ROWS" => rows = val.parse().ok(),
            "COLS" => cols = val.parse().ok(),
            "POINTS" => points = val.parse().ok(),
            _ => {}
        }
        params.push((key.to_string(), val.to_string()));
    }
    if params.is_empty() {
        return None;
    }
    Some(Gxf {
        title,
        rows,
        cols,
        points,
        params,
        data_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"#GRID\nTITLE my grid\nROWS 10\nCOLS 20\nPOINTS 200\nSENSE 1\n#\n\xFF\xFEbinary";
        let g = parse(d).unwrap();
        assert_eq!(g.title.as_deref(), Some("my grid"));
        assert_eq!(g.rows, Some(10));
        assert_eq!(g.cols, Some(20));
        assert_eq!(g.points, Some(200));
        assert_eq!(g.params.len(), 5);
        assert_eq!(&d[g.data_offset..], b"#\n\xFF\xFEbinary");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"#GRIDX\nA 1\n").is_none());
        assert!(parse(b"#GRID\n").is_none()); // no params
        assert!(parse(b"#GRID\nlower key\n").is_none());
    }
}

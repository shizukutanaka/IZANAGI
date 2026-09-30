//! MatrixMarket `.mtx`: `%%MatrixMarket <object> <format> <field>
//! <symmetry>` banner, `%` comment lines, one size line
//! (`rows cols nnz` for coordinate / `rows cols` for array), then
//! coordinate entries `i j [v]` or array values.
//!
//! ```
//! use izanagi_kit::mtx::{detect, parse};
//!
//! let d = b"%%MatrixMarket matrix coordinate real general\n\
//! % comment\n3 3 4\n1 1 1.0\n2 2 2.0\n2 3 0.5\n3 3 4.0\n";
//! assert!(detect(d));
//! let m = parse(d).unwrap();
//! assert_eq!(m.format.as_deref(), Some("coordinate"));
//! assert_eq!(m.rows, Some(3));
//! ```

/// Parsed MatrixMarket census.
#[derive(Debug, Clone, PartialEq)]
pub struct Mtx {
    /// Banner `object` field (`matrix`, `vector`).
    pub object: Option<String>,
    /// Banner `format` field (`coordinate`, `array`).
    pub format: Option<String>,
    /// Banner `field` field (`real`, `integer`, `complex`, `pattern`, `double`).
    pub field: Option<String>,
    /// Banner `symmetry` field (`general`, `symmetric`, `skew-symmetric`, `hermitian`).
    pub symmetry: Option<String>,
    /// `%` comment lines.
    pub comments: u32,
    /// Rows from the size line.
    pub rows: Option<u32>,
    /// Columns from the size line.
    pub cols: Option<u32>,
    /// Declared nonzeros (coordinate) or `None` for array.
    pub nnz: Option<u32>,
    /// Entry lines after the size line.
    pub entries: u32,
}

/// `true` on the `%%MatrixMarket` banner.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"%%MatrixMarket")
}

/// Census; `None` without the banner.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Mtx> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut m = Mtx {
        object: None,
        format: None,
        field: None,
        symmetry: None,
        comments: 0,
        rows: None,
        cols: None,
        nnz: None,
        entries: 0,
    };
    let mut size_done = false;
    for (i, line) in s.lines().enumerate() {
        let t = line.trim();
        if i == 0 {
            let mut it = t.split_whitespace().skip(1);
            m.object = it.next().map(str::to_string);
            m.format = it.next().map(str::to_string);
            m.field = it.next().map(str::to_string);
            m.symmetry = it.next().map(str::to_string);
            continue;
        }
        if t.is_empty() {
            continue;
        }
        if t.starts_with('%') {
            m.comments += 1;
            continue;
        }
        if !size_done {
            let mut it = t.split_whitespace();
            m.rows = it.next().and_then(|v| v.parse().ok());
            m.cols = it.next().and_then(|v| v.parse().ok());
            m.nnz = it.next().and_then(|v| v.parse().ok());
            size_done = true;
            continue;
        }
        m.entries += 1;
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"%%MatrixMarket matrix coordinate real general\n\
% comment\n3 3 4\n1 1 1.0\n2 2 2.0\n2 3 0.5\n3 3 4.0\n";
    const A: &[u8] = b"%%MatrixMarket matrix array integer symmetric\n2 2\n1\n2\n3\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"%MatrixMarket x"));
    }

    #[test]
    fn parses_coordinate() {
        let m = parse(D).unwrap();
        assert_eq!(m.object.as_deref(), Some("matrix"));
        assert_eq!(m.format.as_deref(), Some("coordinate"));
        assert_eq!(m.field.as_deref(), Some("real"));
        assert_eq!(m.symmetry.as_deref(), Some("general"));
        assert_eq!(m.rows, Some(3));
        assert_eq!(m.cols, Some(3));
        assert_eq!(m.nnz, Some(4));
        assert_eq!(m.entries, 4);
        assert_eq!(m.comments, 1);
    }

    #[test]
    fn parses_array() {
        let m = parse(A).unwrap();
        assert_eq!(m.format.as_deref(), Some("array"));
        assert_eq!(m.nnz, None);
        assert_eq!(m.entries, 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

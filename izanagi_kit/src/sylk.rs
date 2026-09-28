//! SYLK (Symbolic Link) — Microsoft's spreadsheet interchange text.
//!
//! Records are `;`-separated: `ID;P…` identity, `C;Xx;Yy;K"…"` cells,
//! `F`/`B`/`O` format sheets, `E` terminator.
//!
//! ```
//! let d = b"ID;PXL;N;E\nC;X1;Y1;K\"v\"\nE\n";
//! let s = izanagi_kit::sylk::parse(d).unwrap();
//! assert_eq!(s.cells, 1);
//! ```

/// Parsed SYLK summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sylk {
    /// `ID;P<name>` producer/product identifier (empty if absent).
    pub producer: String,
    /// `C` (cell) record count.
    pub cells: usize,
    /// `F` (format) record count.
    pub formats: usize,
    /// `E` terminator seen.
    pub terminated: bool,
}

/// First field value after `;KEY` on a record.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pos = line.find(key)?;
    let rest = &line[pos + key.len()..];
    let end = rest.find(';').unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Parse a SYLK file; `None` without a leading `ID;` record.
pub fn parse(d: &[u8]) -> Option<Sylk> {
    let s = std::str::from_utf8(d).ok()?;
    let mut lines = s.lines();
    let first = lines.next()?;
    if !first.starts_with("ID;") {
        return None;
    }
    let producer = field(first, ";P").unwrap_or("").to_string();
    let mut cells = 0usize;
    let mut formats = 0usize;
    let mut terminated = false;
    for l in lines {
        let l = l.trim();
        if l.starts_with('C') && l[1..].starts_with(';') {
            cells += 1;
        } else if l.starts_with('F') && l[1..].starts_with(';') {
            formats += 1;
        } else if l == "E" {
            terminated = true;
        }
    }
    Some(Sylk {
        producer,
        cells,
        formats,
        terminated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"ID;PXL;N;E\nB;X2;Y4\nC;X1;Y1;K\"a\"\nC;X2;Y1;K\"b\"\nF;P0\nE\n";
        let s = parse(d).unwrap();
        assert_eq!(s.producer, "XL");
        assert_eq!((s.cells, s.formats), (2, 1));
        assert!(s.terminated);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"C;X1;Y1\n").is_none()); // no ID first
    }
}

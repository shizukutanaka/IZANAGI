//! OpenDX `.dx` data description — IBM's text container for field
//! data. Statements look like `object <n> class <name>`, `object …
//! data …`, `attribute …`, and `#` comments; `data`/`object` clauses
//! carry the field/grid payload.
//!
//! `parse` requires at least one `object` statement and counts
//! objects, `data` statements and comments.
//!
//! ```
//! let f = b"# dx file\nobject 1 class gridpositions counts 4 4\ndata ...\nobject 2 class field\n";
//! let x = izanagi_kit::dx::parse(f).unwrap();
//! assert_eq!(x.objects, 2);
//! assert_eq!(x.data_statements, 1);
//! assert_eq!(x.comments, 1);
//! ```

/// Parsed OpenDX summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Dx {
    /// `object …` statements.
    pub objects: usize,
    /// `data` keywords seen (data statements / `data` verbs).
    pub data_statements: usize,
    /// `#` comment lines.
    pub comments: usize,
    /// `class field` statements.
    pub fields: usize,
}

/// Parse a `.dx`; `None` without an `object` statement.
pub fn parse(d: &[u8]) -> Option<Dx> {
    let s = std::str::from_utf8(d).ok()?;
    let mut x = Dx {
        objects: 0,
        data_statements: 0,
        comments: 0,
        fields: 0,
    };
    for line in s.lines() {
        let l = line.trim_start();
        if l.starts_with('#') {
            x.comments += 1;
            continue;
        }
        if l.starts_with("object ") || l.starts_with("object\t") {
            x.objects += 1;
            if l.contains("class field") {
                x.fields += 1;
            }
        }
        if l == "data" || l.starts_with("data ") || l.contains(" data ") {
            x.data_statements += 1;
        }
    }
    (x.objects > 0).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"object 1 class array\nobject 2 class field\ndata positions\n";
        let x = parse(f).unwrap();
        assert_eq!(x.objects, 2);
        assert_eq!(x.fields, 1);
        assert_eq!(x.data_statements, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"# only comments\n").is_none());
    }
}

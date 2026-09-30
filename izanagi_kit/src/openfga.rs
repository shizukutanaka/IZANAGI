//! Census of an OpenFGA authorization-model file (DSL `.fga`).
//!
//! OpenFGA DSL: `model`/`schema 1.1` header, `type name` declarations,
//! `relations` blocks, `define rel: [userset] or x from y and not z`,
//! relation expressions `[user:*]`/`tuple to userset`/`computed`/
//! `but not`/`intersection`, `condition name(param: type) { expr }`
//! blocks, `.fga` comment `//`/`#`, optional `module`/`extend` directives.
//!
//! ```rust
//! let c = izanagi_kit::openfga::Openfga::parse(
//!     b"model\n  schema 1.1\ntype user\ntype doc\n  relations\n    define owner: [user]\n",
//! ).unwrap();
//! assert_eq!(c.types, 2);
//! ```
#![forbid(unsafe_code)]

/// OpenFGA model census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Openfga {
    /// `type name` declarations.
    pub types: usize,
    /// `define rel:` clauses.
    pub defines: usize,
    /// `condition` blocks.
    pub conditions: usize,
    /// `model`/`schema`/`module`/`extend` directives.
    pub directives: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

/// True if `b` looks like an OpenFGA model.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains(concat!("schema ", "1.", "1")) || t.contains("type user") || t.contains("define "))
        && (t.contains("relations") || t.contains("type "))
}

impl Openfga {
    /// Parse an `.fga` model into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            types: 0,
            defines: 0,
            conditions: 0,
            directives: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("//") || l.starts_with('#') {
                c.comments += 1;
            } else if l == "model"
                || l.starts_with("schema ")
                || l.starts_with("module ")
                || l.starts_with("extend ")
            {
                c.directives += 1;
            } else if l.starts_with("type ") {
                c.types += 1;
            } else if l.starts_with("define ") {
                c.defines += 1;
            } else if l.starts_with("condition ") {
                c.conditions += 1;
            }
        }
        if c.types == 0 && c.defines == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "// model\n",
            "model\n",
            "  schema 1.1\n",
            "type user\n",
            "type folder\n",
            "  relations\n",
            "    define parent: [folder]\n",
            "    define viewer: [user] or viewer from parent\n",
            "type document\n",
            "  relations\n",
            "    define owner: [user]\n",
            "    define editor: [user] or owner\n",
            "    define viewer: [user:*] or editor or viewer from parent\n",
            "condition valid_time(current_time: timestamp) {\n",
            "    current_time < \"2027-01-01T00:00:00Z\"\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Openfga::parse(b.as_bytes()).unwrap();
        assert_eq!(c.types, 3);
        assert_eq!(c.defines, 5);
        assert_eq!(c.conditions, 1);
        assert_eq!(c.directives, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"fn main() {}\n"));
        assert!(Openfga::parse(b"# none\n").is_none());
    }
}

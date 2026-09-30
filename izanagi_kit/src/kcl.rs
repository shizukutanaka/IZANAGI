//! KCL (KusionStack Configuration Language) source parser.
//!
//! Detects KCL by schema/mixin/rule/protocol declarations or the
//! `import` + `name = value` + `key: value` shape, and counts imports,
//! lambdas, asserts, comprehensions, dicts and `#` comments.
//!
//! ```
//! use izanagi_kit::kcl::Kcl;
//! let src = b"schema Person:\n    name: str\n    age: int = 0\n";
//! assert!(izanagi_kit::kcl::detect(src));
//! let k = Kcl::parse(src).unwrap();
//! assert_eq!(k.schemas, 1);
//! ```

/// Parsed census of a KCL source file.
#[derive(Debug, Clone)]
pub struct Kcl {
    /// `schema` declarations.
    pub schemas: usize,
    /// `mixin` declarations.
    pub mixins: usize,
    /// `rule` declarations.
    pub rules: usize,
    /// `protocol` declarations.
    pub protocols: usize,
    /// `import` statements.
    pub imports: usize,
    /// `lambda` expressions.
    pub lambdas: usize,
    /// `assert` statements.
    pub asserts: usize,
    /// `if`/`elif`/`else`/`for`/`in`/`and`/`or`/`not` keyword hits.
    pub keywords: usize,
    /// `name = value` assignments.
    pub assignments: usize,
    /// `{` dict openings.
    pub dicts: usize,
    /// `[` list openings.
    pub lists: usize,
    /// `#` comment lines.
    pub comments: usize,
    /// `...` attribute-style `key: value` entries (heuristic `word:` inside file).
    pub attrs: usize,
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

/// Returns `true` when `b` looks like KCL source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut has_import = false;
    let mut assigns = 0usize;
    let mut colons = 0usize;
    for line in t.lines() {
        let l = line.trim_start();
        let first = l.split_whitespace().next().unwrap_or("");
        match first {
            "schema" | "schema:" | "mixin" | "rule" | "protocol" => return true,
            "import" => has_import = true,
            _ => {}
        }
        if l.contains(" = ") || l.contains("= \"") {
            assigns += 1;
        }
        if l.contains(':') {
            colons += 1;
        }
    }
    has_import && assigns >= 1 && colons >= 1
}

impl Kcl {
    /// Counts KCL constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut k = Self {
            schemas: 0,
            mixins: 0,
            rules: 0,
            protocols: 0,
            imports: 0,
            lambdas: 0,
            asserts: 0,
            keywords: 0,
            assignments: 0,
            dicts: t.matches('{').count(),
            lists: t.matches('[').count(),
            comments: 0,
            attrs: 0,
        };
        for line in t.lines() {
            let l = line.trim_start();
            if l.starts_with('#') {
                k.comments += 1;
                continue;
            }
            match l.split_whitespace().next().unwrap_or("") {
                "schema" | "schema:" => k.schemas += 1,
                "mixin" => k.mixins += 1,
                "rule" => k.rules += 1,
                "protocol" => k.protocols += 1,
                "import" => k.imports += 1,
                "assert" => k.asserts += 1,
                _ => {}
            }
            if l.contains(" = ") || l.contains("= \"") {
                k.assignments += 1;
            }
            // `key:` attribute (not `key =`, not `:` alone).
            if let Some(pos) = l.find(':') {
                let head = l[..pos].trim_end();
                if !head.is_empty()
                    && head
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
                {
                    k.attrs += 1;
                }
            }
        }
        for w in words(t) {
            match w {
                "lambda" => k.lambdas += 1,
                "if" | "elif" | "else" | "for" | "in" | "and" | "or" | "not" | "is" | "None"
                | "True" | "False" => k.keywords += 1,
                _ => {}
            }
        }
        Some(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"schema Person:\n    name: str\n    age: int = 0\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"import base\nx = {\n  a: 1\n}\n"));
        assert!(!detect(b"x = 1\ny = 2\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let k = Kcl::parse(SRC).unwrap();
        assert_eq!(k.schemas, 1);
        assert!(k.attrs >= 2);
        assert_eq!(k.assignments, 1);
    }

    #[test]
    fn counts_kinds() {
        let s =
            b"import a\nrule Check:\n    assert True\nmixin M:\n    pass\nx = lambda a: a\n# c\n";
        let k = Kcl::parse(s).unwrap();
        assert_eq!(k.imports, 1);
        assert_eq!(k.rules, 1);
        assert_eq!(k.mixins, 1);
        assert_eq!(k.asserts, 1);
        assert_eq!(k.lambdas, 1);
        assert_eq!(k.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(Kcl::parse(b"plain text").is_none());
        assert!(Kcl::parse(b"").is_none());
    }
}

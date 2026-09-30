//! Census of an OPA Rego policy file.
//!
//! `.rego` modules: `package a.b.c` header, `import` statements, rules
//! `name(args) := value if { … }` / `name if { … }` / `name = value`,
//! `default x := v`, `else` chains, comprehension keywords
//! (`some`/`every`/`if`/`else`/`contains`/`in`), function calls
//! `input.*`/`data.*` references, `deny`/`allow`/`violation`/`audit`
//! common rule names, `:=` vs `=` assignments, `#` comments, and
//! `with`/`trace`/`sprintf` builtins.
//!
//! ```rust
//! let c = izanagi_kit::rego::Rego::parse(
//!     b"package authz\nimport data.users\ndefault allow := false\nallow if { input.role == \"admin\" }\n",
//! ).unwrap();
//! assert_eq!(c.rules, 2);
//! ```
#![forbid(unsafe_code)]

/// Rego policy census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rego {
    /// `package` declarations.
    pub packages: usize,
    /// `import` statements.
    pub imports: usize,
    /// Rule heads (`name … {`, `name := …`, `default …`, `else`).
    pub rules: usize,
    /// `if`/`some`/`every`/`contains`/`else`/`in`/`with` keyword uses.
    pub keywords: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Rego keyword prefixes that count as keyword lines.
const KW: &[&str] = &["if", "else", "some", "every", "contains", "with", "not "];

/// True if `b` looks like Rego source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("package ") && (t.contains("input.") || t.contains("data.") || t.contains(" if ")))
        || t.contains("import rego.v1")
}

impl Rego {
    /// Parse a `.rego` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            packages: 0,
            imports: 0,
            rules: 0,
            keywords: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
            } else if l.starts_with("package ") {
                c.packages += 1;
            } else if l.starts_with("import ") {
                c.imports += 1;
            } else if KW.iter().any(|k| l.starts_with(k)) {
                c.keywords += 1;
            } else if l == "}" || l.starts_with('}') || l.starts_with('[') || l.starts_with('{') {
                // brace/close lines: skip
            } else if l.contains(":=") || l.contains(" if ") || l.ends_with('{') || l.contains('(')
            {
                // rule head or function call/body
                let head = l.split([' ', '{', '(']).next().unwrap_or("");
                if !head.is_empty()
                    && head
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                {
                    c.rules += 1;
                }
            } else if l.chars().next().is_some_and(|ch| ch.is_ascii_lowercase())
                && l.chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '[' || ch == ']')
            {
                // bare `allow`/`deny` style rule name
                c.rules += 1;
            }
        }
        if c.packages == 0 {
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
            "# policy\n",
            "package authz\n",
            "import data.users\n",
            "import rego.v1\n",
            "default allow := false\n",
            "allow if {\n",
            "    input.role == \"admin\"\n",
            "}\n",
            "deny if {\n",
            "    not allow\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Rego::parse(b.as_bytes()).unwrap();
        assert_eq!(c.packages, 1);
        assert_eq!(c.imports, 2);
        assert_eq!(c.comments, 1);
        assert!(c.rules >= 3);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"fn main() {}\n"));
        assert!(Rego::parse(b"# none\n").is_none());
    }
}

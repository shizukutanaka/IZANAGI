//! Census of a HashiCorp Sentinel policy file.
//!
//! `.sentinel` policies: `import "..."` modules, `param`/`const`/`var`
//! declarations, `main = rule { … }`/`rule { … }`/`func name(...) { … }`,
//! `policy "name" { … }` blocks, `when`/`all`/`any`/`for`/`map`/`filter`/
//! `length`/`print`/`times.*`/`sockaddr.*` builtins, `:=`/`-`/comparison
//! operators, `true`/`false`/`null` literals, `//`/`#` comments.
//!
//! ```rust
//! let c = izanagi_kit::sentinel::Sentinel::parse(
//!     b"main = rule {\n    length(servers) > 0\n}\n",
//! ).unwrap();
//! assert_eq!(c.rules, 1);
//! ```
#![forbid(unsafe_code)]

/// Sentinel policy census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sentinel {
    /// `import "x"` module imports.
    pub imports: usize,
    /// `param`/`const`/`var` declarations.
    pub decls: usize,
    /// `main = rule`/`rule {}`/`func`/`policy` bodies.
    pub rules: usize,
    /// `when`/`all`/`any`/`for`/`map`/`filter`/`else` control keywords.
    pub keywords: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

/// Control keywords.
const KW: &[&str] = &[
    "when", "all", "any", "for", "map", "filter", "else", "if", "return",
];

/// True if `b` looks like Sentinel source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("rule {") || t.contains("= rule") || t.contains("param "))
        && (t.contains("main") || t.contains("import") || t.contains("when"))
}

impl Sentinel {
    /// Parse a `.sentinel` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            imports: 0,
            decls: 0,
            rules: 0,
            keywords: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("//") || l.starts_with('#') {
                c.comments += 1;
            } else if l.starts_with("import ") {
                c.imports += 1;
            } else if l.starts_with("param ") || l.starts_with("const ") || l.starts_with("var ") {
                c.decls += 1;
            } else if l.starts_with("func ")
                || l.starts_with("policy ")
                || l.contains("= rule")
                || l.starts_with("rule ")
                || l == "rule {"
            {
                c.rules += 1;
            } else {
                let head = l
                    .split([' ', '('])
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(':');
                if KW.contains(&head) {
                    c.keywords += 1;
                }
            }
        }
        if c.rules == 0 && c.imports == 0 && c.decls == 0 {
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
            "// policy\n",
            "import \"time\"\n",
            "param ttl\n",
            "const threshold = 5\n",
            "var allowed = true\n",
            "main = rule {\n",
            "    allowed and threshold > 0\n",
            "}\n",
            "deny = rule {\n",
            "    when ttl > 0 {\n",
            "        ttl > 10\n",
            "    }\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Sentinel::parse(b.as_bytes()).unwrap();
        assert_eq!(c.imports, 1);
        assert_eq!(c.decls, 3);
        assert_eq!(c.rules, 2);
        assert_eq!(c.keywords, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"fn main() {}\n"));
        assert!(Sentinel::parse(b"# none\n").is_none());
    }
}

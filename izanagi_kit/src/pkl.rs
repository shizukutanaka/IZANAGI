//! Apple Pkl configuration language source parser.
//!
//! Detects Pkl by module clauses (`amends`/`extends`/`module`/`import`),
//! `typealias`/`class` declarations or `local` bindings with `=`, and
//! counts functions, `new` instantiations, annotations, `if`/`else` and
//! `//`-style comments.
//!
//! ```
//! use izanagi_kit::pkl::Pkl;
//! let src = b"amends \"base.pkl\"\n\nname = \"app\"\nport = 8080\n";
//! assert!(izanagi_kit::pkl::detect(src));
//! let p = Pkl::parse(src).unwrap();
//! assert_eq!(p.amends, 1);
//! assert_eq!(p.assignments, 2);
//! ```

/// Parsed census of a Pkl source file.
#[derive(Debug, Clone)]
pub struct Pkl {
    /// `amends "uri"` clauses.
    pub amends: usize,
    /// `extends "uri"` clauses.
    pub extends: usize,
    /// `module name` clauses.
    pub modules: usize,
    /// `import` / `import*` clauses.
    pub imports: usize,
    /// `class` declarations.
    pub classes: usize,
    /// `typealias` declarations.
    pub typealiases: usize,
    /// `function` declarations.
    pub functions: usize,
    /// `local` bindings.
    pub locals: usize,
    /// `new` instantiations.
    pub news: usize,
    /// `@Annotation` uses.
    pub annotations: usize,
    /// `name = value` assignments.
    pub assignments: usize,
    /// `//`, `///`, `/*` comments.
    pub comments: usize,
    /// Keyword hits (`if`/`else`/`for`/`when`/`as`/`is`/`hidden`/`fixed`/`const`/`abstract`/`open`/`out`/`in`/`this`/`outer`/`super`/`let`/`throw`/`trace`/`read`/`read?`/`read!`/`import`/`null`).
    pub keywords: usize,
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '?' || c == '!'))
        .filter(|w| !w.is_empty())
}

/// Returns `true` when `b` looks like Pkl source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(v) => v,
        Err(_) => return false,
    };
    for line in t.lines() {
        let l = line.trim_start();
        let first = l.split_whitespace().next().unwrap_or("");
        match first {
            "amends" | "extends" | "module" | "typealias" | "abstract" | "open" => {
                return true;
            }
            "import" | "import*" if l.contains('"') => return true,
            _ => {}
        }
    }
    t.contains("local ") && t.contains(" = ") && t.contains('{')
}

impl Pkl {
    /// Counts Pkl constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut p = Self {
            amends: 0,
            extends: 0,
            modules: 0,
            imports: 0,
            classes: 0,
            typealiases: 0,
            functions: 0,
            locals: 0,
            news: 0,
            annotations: 0,
            assignments: 0,
            comments: t.matches("//").count() + t.matches("/*").count(),
            keywords: 0,
        };
        for line in t.lines() {
            let l = line.trim_start();
            match l.split_whitespace().next().unwrap_or("") {
                "amends" => p.amends += 1,
                "extends" => p.extends += 1,
                "module" => p.modules += 1,
                "import" | "import*" => p.imports += 1,
                "class" | "abstract" | "open" => {
                    if l.contains("class") {
                        p.classes += 1;
                    }
                }
                "typealias" => p.typealiases += 1,
                _ => {}
            }
            if l.contains(" = ") || l.ends_with('=') {
                p.assignments += 1;
            }
        }
        for w in words(t) {
            match w {
                "function" => p.functions += 1,
                "local" => p.locals += 1,
                "new" => p.news += 1,
                "if" | "else" | "for" | "when" | "as" | "is" | "hidden" | "fixed" | "const"
                | "abstract" | "open" | "out" | "in" | "this" | "outer" | "super" | "let"
                | "throw" | "trace" | "read" | "read?" | "read!" | "null" => {
                    p.keywords += 1;
                }
                _ => {}
            }
        }
        p.annotations += t.matches('@').count();
        Some(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"amends \"base.pkl\"\n\nname = \"app\"\nport = 8080\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"import \"lib.pkl\"\nx = 1"));
        assert!(detect(b"typealias Name = String"));
        assert!(!detect(b"x = 1\ny = 2"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let p = Pkl::parse(SRC).unwrap();
        assert_eq!(p.amends, 1);
        assert_eq!(p.assignments, 2);
    }

    #[test]
    fn counts_kinds() {
        let s = b"module app\nimport \"a.pkl\"\nclass C {}\ntypealias T = String\nlocal x = new {}\n@Anno\nfunction f() = 1\n// c\n";
        let p = Pkl::parse(s).unwrap();
        assert_eq!(p.modules, 1);
        assert_eq!(p.imports, 1);
        assert_eq!(p.classes, 1);
        assert_eq!(p.typealiases, 1);
        assert_eq!(p.locals, 1);
        assert_eq!(p.functions, 1);
        assert_eq!(p.news, 1);
        assert!(p.annotations >= 1);
        assert_eq!(p.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(Pkl::parse(b"plain text").is_none());
        assert!(Pkl::parse(b"").is_none());
    }
}

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

/// `true` when some `{…}` pair in `t` holds an `=` member.
fn brace_has_member(t: &str) -> bool {
    let b = t.as_bytes();
    let mut depth = 0usize;
    let mut has_member = false;
    for &c in b {
        match c {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'=' if depth > 0 => has_member = true,
            _ => {}
        }
    }
    has_member
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
            "amends" | "extends" | "typealias" | "abstract" | "open" => {
                return true;
            }
            // `module foo` in Pkl is exactly a dotted identifier on its
            // own line — `module X { … }` (IDL/Verilog/Zeek/Move) is not
            "module" => {
                // a `//` comment may follow the module name
                let rest = l["module".len()..].split("//").next().unwrap_or("").trim();
                if !rest.is_empty()
                    && rest
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
                    && rest.chars().next().is_some_and(|c| c.is_alphabetic())
                {
                    return true;
                }
            }
            // Pkl imports are URIs (`"pkl:json"`, `"./x.pkl"`, `"@p/a"`);
            // a bare `"name"` is another language's import/require
            "import" | "import*" => {
                let rest = l.strip_prefix("import").unwrap_or("");
                if let Some(s) = rest.trim_start_matches('*').trim_start().strip_prefix('"') {
                    let inner = s.split('"').next().unwrap_or("");
                    // URI-shaped (`pkl:json`, `./x.pkl`, `@p/a`) or a
                    // plain relative name — not a hyphenated foreign
                    // token like `"generic-service"`
                    if !inner.is_empty()
                        && (inner.chars().any(|c| matches!(c, ':' | '/' | '.' | '@'))
                            || inner.chars().all(|c| c.is_alphanumeric() || c == '_'))
                    {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    // `local x = …` plus an object body — a `{…}` pair holding `=`
    // members or `new` instantiation is the Pkl part; Lua/jsonnet
    // bindings and empty `{}` tables are not
    t.contains("local ") && t.contains(" = ") && t.contains("new ")
        || (t.contains("local ") && t.contains(" = ") && brace_has_member(t))
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
        // `module X {`/`module X (`/`module X;` is IDL/Verilog/Zeek,
        // not Pkl — a Pkl module clause is a lone dotted identifier
        assert!(!detect(b"module Bank {\n  interface A {}\n};\n"));
        assert!(!detect(b"module top(input clk);\nendmodule\n"));
        assert!(!detect(b"module Site;\nexport {}\n"));
        assert!(!detect(b"module 0x1::coin {\n}\n"));
        assert!(detect(b"module app.x\nx = 1\n"));
        assert!(detect(b"module app // trailing comment\nx = 1\n"));
        // `import "name"` without a URI-shaped target is another
        // language's import/require — Pkl imports are URIs or
        // plain identifier-shaped relative names
        assert!(!detect(b"import \"generic-service\"\nx = 1\n"));
        assert!(!detect(b"declare v := 1\nimport module \"m\"\n"));
        assert!(detect(b"import \"settings\"\nvalue = settings.value\n"));
        // a multi-byte whitespace after `import` must not split a char
        assert!(detect("import\u{3000}\"settings\"\n".as_bytes()));
        // `local` + `=` is Lua/jsonnet; Pkl locals also bind scalars,
        // so the object-member or `new` shape is what counts
        assert!(!detect(b"local x = 1\nconfig.keys = {}\nreturn x\n"));
        assert!(detect(b"local cfg = new { a = 1 }\n"));
        assert!(detect(
            b"local greeting = \"hi\"\noutput { message = greeting }\n"
        ));
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

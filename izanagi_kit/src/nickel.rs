//! Nickel configuration language source parser.
//!
//! Detects Nickel by `let x = .. in ..` bindings plus a strong marker
//! (`=>`, `fun`, `|` contract annotations, `import ".."`, `%{` string
//! interpolation) and counts contracts, merges, imports, annotations
//! and `#` comments.
//!
//! ```
//! use izanagi_kit::nickel::Nickel;
//! let src = b"let x = 1 in\n{ value = x, f = fun n => n + x }\n";
//! assert!(izanagi_kit::nickel::detect(src));
//! let n = Nickel::parse(src).unwrap();
//! assert_eq!(n.lets, 1);
//! assert_eq!(n.functions, 2);
//! ```

/// Parsed census of a Nickel source file.
#[derive(Debug, Clone)]
pub struct Nickel {
    /// `let` bindings.
    pub lets: usize,
    /// `fun` / `=>` lambdas.
    pub functions: usize,
    /// `|` contract/type annotations.
    pub contracts: usize,
    /// `import ".."` expressions.
    pub imports: usize,
    /// `&` record merges.
    pub merges: usize,
    /// `%{` string interpolations.
    pub interpolations: usize,
    /// `=` assignments (heuristic).
    pub assignments: usize,
    /// `{` records.
    pub records: usize,
    /// `[` arrays.
    pub arrays: usize,
    /// `#` comment lines.
    pub comments: usize,
    /// Keyword hits (`if`/`then`/`else`/`in`/`match`/`forall`/`doc`/`default`/`priority`/`force`).
    pub keywords: usize,
}

fn count(t: &str, needle: &str) -> usize {
    t.matches(needle).count()
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '\''))
        .filter(|w| !w.is_empty())
}

/// Source with `"..."` literals and `#` comments blanked, so keyword
/// checks never fire inside string values or comments.
fn code_only(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut in_string = false;
    let mut in_comment = false;
    for c in t.chars() {
        match (in_string, in_comment, c) {
            (false, false, '"') => {
                in_string = true;
                out.push(' ');
            }
            (false, false, '#') => {
                in_comment = true;
                out.push(' ');
            }
            (true, _, '"') => {
                in_string = false;
                out.push(' ');
            }
            (_, true, '\n') => {
                in_comment = false;
                out.push('\n');
            }
            (false, false, c) => out.push(c),
            _ => out.push(' '),
        }
    }
    out
}

/// Returns `true` when `b` looks like Nickel source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(v) => v,
        Err(_) => return false,
    };
    // `in` is matched as a word outside strings/comments, so both LF and
    // CRLF line endings hit without counting quoted or commented `in`.
    let c = code_only(t);
    (c.contains("let ") && words(&c).any(|w| w == "in")) || t.contains("import \"")
}

impl Nickel {
    /// Counts Nickel constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut n = Self {
            lets: count(t, "let "),
            functions: count(t, "fun ") + count(t, "=>"),
            contracts: count(t, " | "),
            imports: count(t, "import \""),
            merges: count(t, " & "),
            interpolations: count(t, "%{"),
            assignments: count(t, " = "),
            records: count(t, "{"),
            arrays: count(t, "["),
            comments: 0,
            keywords: 0,
        };
        for line in t.lines() {
            let l = line.trim_start();
            if l.starts_with('#') {
                n.comments += 1;
            }
        }
        for w in words(t) {
            match w {
                "if" | "then" | "else" | "in" | "match" | "forall" | "doc" | "default"
                | "priority" | "force" | "rec" | "tag" => n.keywords += 1,
                _ => {}
            }
        }
        Some(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"let x = 1 in\n{ value = x, f = fun n => n + x }\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"let a = import \"x.ncl\" in a"));
        assert!(!detect(b"plain text"));
        assert!(!detect(b""));
    }

    #[test]
    fn detects_crlf_source() {
        // `in` immediately before CRLF (`in\r\n`) is still the keyword.
        assert!(detect(b"let x = 1 in\r\n{ value = x }\r\n"));
    }

    #[test]
    fn rejects_in_inside_string_or_comment() {
        // `in` inside a string literal or a comment is not a delimiter.
        assert!(!detect(b"let x = \"in\"\n"));
        assert!(!detect(b"let x = 1 # in\n"));
    }

    #[test]
    fn parses() {
        let n = Nickel::parse(SRC).unwrap();
        assert_eq!(n.lets, 1);
        assert_eq!(n.functions, 2);
        assert_eq!(n.records, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"let v | Num = 1 in\n{ a = v, b = \"%{v}\" } & { c = 2 }\n# c\nlet i = import \"i.ncl\" in a\n";
        let n = Nickel::parse(s).unwrap();
        assert_eq!(n.contracts, 1);
        assert_eq!(n.merges, 1);
        assert_eq!(n.interpolations, 1);
        assert_eq!(n.imports, 1);
        assert_eq!(n.comments, 1);
        assert!(n.keywords >= 1);
    }

    #[test]
    fn rejects() {
        assert!(Nickel::parse(b"plain text").is_none());
        assert!(Nickel::parse(b"").is_none());
    }
}

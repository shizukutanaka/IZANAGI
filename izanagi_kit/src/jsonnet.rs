//! Jsonnet configuration language source parser.
//!
//! Detects Jsonnet by `local` bindings plus `self.`/`super.`/`$.`/`std.`
//! references or `::`/`:::` hidden-field syntax, and counts functions,
//! asserts, imports, conditionals, comprehensions and comments.
//!
//! ```
//! use izanagi_kit::jsonnet::Jsonnet;
//! let src = b"local x = 1;\n{ a:: x, b: self.a + 1 }\n";
//! assert!(izanagi_kit::jsonnet::detect(src));
//! let j = Jsonnet::parse(src).unwrap();
//! assert_eq!(j.locals, 1);
//! assert_eq!(j.hidden_fields, 1);
//! ```

/// Parsed census of a Jsonnet source file.
#[derive(Debug, Clone)]
pub struct Jsonnet {
    /// `local` bindings (incl. `local f() = `).
    pub locals: usize,
    /// `function(` definitions/calls.
    pub functions: usize,
    /// `assert` statements.
    pub asserts: usize,
    /// `self.` references.
    pub self_refs: usize,
    /// `super.` references.
    pub super_refs: usize,
    /// `$.` outermost references.
    pub dollar_refs: usize,
    /// `std.` library calls.
    pub std_calls: usize,
    /// `import`/`importstr`/`importbin`/`importyaml`.
    pub imports: usize,
    /// Hidden fields `::` or `:::`.
    pub hidden_fields: usize,
    /// Visible fields `name:` / `+: `.
    pub fields: usize,
    /// `if`/`then`/`else`/`for`/`in`/`error`/`assert` keyword hits.
    pub keywords: usize,
    /// `//`/`/*`/`#` comment markers.
    pub comments: usize,
}

fn count(t: &str, needle: &str) -> usize {
    t.matches(needle).count()
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

/// Returns `true` when `b` looks like Jsonnet source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if !t.contains("local ") {
        return false;
    }
    t.contains("self.")
        || t.contains("super.")
        || t.contains("$.")
        || t.contains("std.")
        || t.contains(":: ")
        || t.contains("::,")
        || t.contains("::\n")
        || t.contains(":::")
        || t.contains("function(")
}

impl Jsonnet {
    /// Counts Jsonnet constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        // `:::` yields one non-overlapping `::` match — count directly.
        let hidden = count(t, "::");
        let mut j = Self {
            locals: count(t, "local "),
            functions: count(t, "function("),
            asserts: 0,
            self_refs: count(t, "self."),
            super_refs: count(t, "super."),
            dollar_refs: count(t, "$."),
            std_calls: count(t, "std."),
            imports: count(t, "import "),
            hidden_fields: hidden,
            fields: 0,
            keywords: 0,
            comments: count(t, "//") + count(t, "/*") + count(t, "#"),
        };
        for w in words(t) {
            match w {
                "assert" => {
                    j.asserts += 1;
                    j.keywords += 1;
                }
                "if" | "then" | "else" | "for" | "in" | "error" | "assertEqual" => {
                    j.keywords += 1;
                }
                _ => {}
            }
        }
        // field: identifier-ish char then ':' then space/value (skip '::').
        let b2 = t.as_bytes();
        for i in 1..b2.len().saturating_sub(1) {
            if b2[i] == b':' && b2[i - 1] != b':' && b2[i + 1] != b':' && b2[i + 1] != b'=' {
                j.fields += 1;
            }
        }
        Some(j)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"local x = 1;\n{ a:: x, b: self.a + 1 }\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"local a = 1; { x: std.join(',', [a]) }"));
        assert!(!detect(b"local x = 1; x"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let j = Jsonnet::parse(SRC).unwrap();
        assert_eq!(j.locals, 1);
        assert_eq!(j.hidden_fields, 1);
        assert_eq!(j.self_refs, 1);
        assert!(j.fields >= 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"local f(x) = x + 1;\nassert x > 0;\n{\n  y::: 1,\n  z: $.root,\n  w: if true then 1 else 2, // c\n}\n";
        let j = Jsonnet::parse(s).unwrap();
        assert_eq!(j.asserts, 1);
        assert_eq!(j.hidden_fields, 1);
        assert_eq!(j.dollar_refs, 1);
        assert!(j.keywords >= 4);
        assert_eq!(j.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(Jsonnet::parse(b"plain text").is_none());
        assert!(Jsonnet::parse(b"").is_none());
    }
}

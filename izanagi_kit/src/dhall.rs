//! Dhall configuration language source parser.
//!
//! Detects Dhall by `let`/ `in` bindings plus a strong marker
//! (`->`/`∀`/`λ`/`forall`/`env:`/`http(s)://`/`sha256:`) and counts
//! bindings, function arrows, quantifiers, assertions, merges,
//! imports, record/list delimiters and `--` / `{-` comments.
//!
//! ```
//! use izanagi_kit::dhall::Dhall;
//! let src = b"let x = 1\nlet f = \\(n : Natural) -> n + x\nin f 2 -- done\n";
//! assert!(izanagi_kit::dhall::detect(src));
//! let d = Dhall::parse(src).unwrap();
//! assert_eq!(d.lets, 2);
//! assert_eq!(d.arrows, 1);
//! ```

/// Parsed census of a Dhall source file.
#[derive(Debug, Clone)]
pub struct Dhall {
    /// `let` bindings.
    pub lets: usize,
    /// `in` keyword uses.
    pub ins: usize,
    /// `->` function arrows.
    pub arrows: usize,
    /// `forall` / `∀` / `λ` quantifiers or lambdas.
    pub quantifiers: usize,
    /// `assert` keyword.
    pub assertions: usize,
    /// `merge` keyword.
    pub merges: usize,
    /// Import markers (`env:`/`http://`/`https://`/`sha256:`/`missing`).
    pub imports: usize,
    /// `{` record/`{ }` openings.
    pub records: usize,
    /// `[` list openings.
    pub lists: usize,
    /// `--` line comments + `{-` block comments.
    pub comments: usize,
    /// Other keyword hits (`if`/`then`/`else`/`toMap`/`Some`/`None`/`constructors`).
    pub keywords: usize,
}

fn count(t: &str, needle: &str) -> usize {
    t.matches(needle).count()
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .filter(|w| !w.is_empty())
}

/// Returns `true` when `b` looks like a Dhall source file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if !t.contains("let ") || !(t.contains(" in ") || t.contains("\nin ")) {
        return false;
    }
    t.contains("->")
        || t.contains('∀')
        || t.contains('λ')
        || t.contains("forall")
        || t.contains("env:")
        || t.contains("://")
        || t.contains("sha256:")
        || t.contains("./")
}

impl Dhall {
    /// Counts Dhall constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut d = Self {
            lets: 0,
            ins: count(t, " in ") + count(t, "\nin "),
            arrows: count(t, "->"),
            quantifiers: count(t, "forall") + count(t, "∀") + count(t, "λ") + count(t, "\\("),
            assertions: 0,
            merges: 0,
            imports: count(t, "env:")
                + count(t, "http://")
                + count(t, "https://")
                + count(t, "sha256:")
                + count(t, " missing")
                + count(t, "./"),
            records: count(t, "{"),
            lists: count(t, "["),
            comments: count(t, "--") + count(t, "{-"),
            keywords: 0,
        };
        for line in t.lines() {
            if line.trim_start().starts_with("let ") {
                d.lets += 1;
            }
        }
        for w in words(t) {
            match w {
                "assert" => d.assertions += 1,
                "merge" => d.merges += 1,
                "if" | "then" | "else" | "toMap" | "Some" | "None" | "constructors" | "Text"
                | "Natural" | "Integer" | "Double" | "Bool" | "List" | "Optional" => {
                    d.keywords += 1;
                }
                _ => {}
            }
        }
        Some(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"let x = 1\nlet f = \\(n : Natural) -> n + x\nin f 2 -- done\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"let a = ./file.dhall\nin a"));
        assert!(detect(b"let k = env:HOME\nin k"));
        assert!(!detect(b"{\"a\": 1}"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let d = Dhall::parse(SRC).unwrap();
        assert_eq!(d.lets, 2);
        assert_eq!(d.arrows, 1);
        assert!(d.quantifiers >= 1);
        assert_eq!(d.comments, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"let m = merge { a = 1 } { b = 2 }\nlet c = assert : Natural\nlet l = [1, 2]\nlet i = ./x.dhall\nin m\n{- tail -}\n";
        let d = Dhall::parse(s).unwrap();
        assert_eq!(d.merges, 1);
        assert_eq!(d.assertions, 1);
        assert_eq!(d.lists, 1);
        assert_eq!(d.imports, 1);
        assert!(d.keywords >= 1);
    }

    #[test]
    fn rejects() {
        assert!(Dhall::parse(b"plain text").is_none());
        assert!(Dhall::parse(b"").is_none());
    }
}

//! XPath 1.0/2.0 expression — `//`/`/` steps, `@attr`, `[pred]` filters,
//! `node()`/`text()`/`name()`/`contains()`/`position()`/`last()` functions,
//! `|` union and axis `descendant::`/`ancestor::`/`parent::`/`self::`.
//!
//! ```
//! let d = b"//book[@id='a']/title[contains(.,'x')]";
//! let x = izanagi_kit::xpath::parse(d).unwrap();
//! assert_eq!(x.dslashes, 1);
//! assert_eq!(x.attrs, 1);
//! assert_eq!(x.predicates, 2);
//! assert_eq!(x.functions, 1);
//! assert!(izanagi_kit::xpath::detect(d));
//! ```

/// Census of an XPath expression.
#[derive(Debug, Clone)]
pub struct Xpath {
    /// `//` descendant-or-self steps.
    pub dslashes: usize,
    /// `/` child steps (single slashes excluding `//`).
    pub slashes: usize,
    /// `@` attribute steps.
    pub attrs: usize,
    /// `[` predicate opens.
    pub predicates: usize,
    /// `name(`/`node(`/`text(`/`comment(`/`processing-instruction(` etc.
    pub functions: usize,
    /// `axis::` explicit axes.
    pub axes: usize,
    /// `|` unions.
    pub unions: usize,
    /// `.`/`..` context steps.
    pub dots: usize,
    /// `$` variables.
    pub variables: usize,
    /// Comparison `=`/`!=`/`<`/`>`/`<=`/`>=`/`eq`/`lt`/`gt`.
    pub comparisons: usize,
    /// `and`/`or`/`not(`/`mod`/`div`.
    pub operators: usize,
    /// `*` wildcards.
    pub wildcards: usize,
    /// Numeric literals.
    pub numbers: usize,
    /// String literals `'`/`"`.
    pub strings: usize,
}

const FN: &[&str] = &[
    "node(",
    "text(",
    "comment(",
    "processing-instruction(",
    "name(",
    "local-name(",
    "namespace-uri(",
    "string(",
    "concat(",
    "contains(",
    "starts-with(",
    "position(",
    "last(",
    "count(",
    "id(",
    "not(",
    "true(",
    "false(",
    "number(",
    "sum(",
    "floor(",
    "ceiling(",
    "round(",
    "boolean(",
    "string-length(",
    "normalize-space(",
    "translate(",
    "substring(",
    "substring-before(",
    "substring-after(",
];

/// Detects XPath: an *expression-shaped* input — at most a few
/// non-empty lines — carrying an XPath-exclusive marker (`//step`,
/// `[@`, a `name::` axis or a `name(` function) or an absolute
/// `/step/step` path. Any `/` or `@` inside a longer document is not
/// a query.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = t.trim();
    if t.is_empty()
        || !t.bytes().all(|c| c.is_ascii())
        || t.contains("http")
        || t.contains("<?xml")
        || t.lines().filter(|l| !l.trim().is_empty()).count() > 3
    {
        return false;
    }
    // `//` followed by a node-test start (`//book`, `//*`, `//@`,
    // `//.`, `//[`). `// comment` prose and `a // b` division miss.
    let mut hits = 0usize;
    {
        let bb = t.as_bytes();
        let mut i = 0;
        while let Some(p) = t[i..].find("//") {
            let at = i + p;
            match bb.get(at + 2) {
                Some(&c)
                    if c.is_ascii_alphabetic() || matches!(c, b'*' | b'@' | b'.' | b'[' | b'(') =>
                {
                    hits += 1;
                }
                _ => {}
            }
            i = at + 2;
        }
    }
    hits += t.matches("[@").count();
    for name in FN {
        if t.contains(name) {
            hits += 1;
        }
    }
    for axis in [
        "ancestor::",
        "attribute::",
        "child::",
        "descendant::",
        "following-sibling::",
        "following::",
        "namespace::",
        "parent::",
        "preceding-sibling::",
        "preceding::",
        "self::",
    ] {
        if t.contains(axis) {
            hits += 1;
        }
    }
    // absolute location path `/a/b` — all steps name-ish
    let path = t.starts_with('/')
        && t.len() > 1
        && t[1..].split('/').all(|s| {
            !s.is_empty()
                && s.chars().all(|c| {
                    c.is_ascii_alphanumeric()
                        || matches!(
                            c,
                            '_' | '-'
                                | '.'
                                | '*'
                                | '@'
                                | '('
                                | ')'
                                | '['
                                | ']'
                                | '\''
                                | '"'
                                | '='
                                | ' '
                                | ':'
                        )
                })
        });
    hits >= 1 || path
}

/// Parses an XPath expression; `None` on non-UTF-8 or missing steps.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xpath> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let t = t.trim();
    let dslashes = t.matches("//").count();
    Some(Xpath {
        dslashes,
        slashes: t.matches('/').count() - 2 * dslashes,
        attrs: t.matches('@').count(),
        predicates: t.matches('[').count(),
        functions: FN.iter().map(|f| t.matches(f).count()).sum(),
        axes: t.matches("::").count(),
        unions: t.matches('|').count(),
        dots: t.matches("..").count()
            + if t.ends_with('.') || t.starts_with('.') {
                1
            } else {
                0
            },
        variables: t.matches('$').count(),
        comparisons: t.matches("!=").count()
            + t.matches("=~").count()
            + t.matches("<=").count()
            + t.matches(">=").count()
            + (t.matches('<').count() - t.matches("<=").count())
            + (t.matches('>').count() - t.matches(">=").count())
            + (t.matches('=').count()
                - t.matches("!=").count()
                - t.matches("<=").count()
                - t.matches(">=").count()
                - t.matches("=~").count())
            + count_word(t, "eq")
            + count_word(t, "lt")
            + count_word(t, "gt"),
        operators: count_word(t, "and")
            + count_word(t, "or")
            + count_word(t, "not")
            + count_word(t, "mod")
            + count_word(t, "div"),
        wildcards: t.matches('*').count(),
        numbers: t
            .split(|c: char| c.is_ascii_digit())
            .count()
            .saturating_sub(1),
        strings: t.matches('\'').count() / 2 + t.matches('"').count() / 2,
    })
}

fn count_word(t: &str, k: &str) -> usize {
    let mut n = 0;
    let mut from = 0;
    while let Some(slice) = t.get(from..) {
        let Some(p) = slice.find(k) else {
            break;
        };
        let a = from + p;
        let before = a == 0
            || t[..a]
                .chars()
                .last()
                .is_some_and(|c| !c.is_alphanumeric() && c != '_' && c != '-');
        let after = t[a + k.len()..].chars().next().map_or(true, |c| {
            !c.is_alphanumeric() && c != '_' && c != '-' && c != '('
        });
        if before && after {
            n += 1;
        }
        from = a + k.len();
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"//book[@id='a' and position()=last()]/title | //author/name[../@x]";

    #[test]
    fn parses() {
        let x = parse(D).unwrap();
        assert_eq!(x.dslashes, 2);
        assert_eq!(x.attrs, 2);
        assert_eq!(x.predicates, 2);
        assert_eq!(x.functions, 2); // position() + last()
        assert_eq!(x.unions, 1);
        assert_eq!(x.dots, 1);
        assert!(x.operators >= 1);
        assert!(x.strings >= 1);
    }

    #[test]
    fn axes() {
        let x = parse(b"descendant::p/ancestor::d").unwrap();
        assert_eq!(x.axes, 2);
        assert_eq!(x.slashes, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"/a/b"));
        assert!(!detect(b"http://x/y"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"just text").is_none());
        // `@` inside prose is not an attribute axis
        assert!(!detect(b"contact a@b.com or c@d.net\n"));
        // `//` inside a longer document is not a query
        assert!(!detect(
            b"line one\nsee //x/y docs\nline three\nline four\n"
        ));
    }
}

//! JSONPath (RFC 9535 / Goessner) — `$.` root, `.name`, `..name`,
//! `['quoted']`, `[0]`/`[*]` subscripts, slices `[a:b:c]`, filters `[?(@...)]`,
//! wildcards and function calls `@.length`.
//!
//! ```
//! let d = b"$.store.book[*].author";
//! let j = izanagi_kit::jsonpath::parse(d).unwrap();
//! assert!(j.root);
//! assert_eq!(j.names, 3);
//! assert_eq!(j.wildcards, 1);
//! assert!(izanagi_kit::jsonpath::detect(d));
//! ```

use crate::textutil::strip_bom;
/// Census of a JSONPath expression.
#[derive(Debug, Clone)]
pub struct Jsonpath {
    /// `$` root present.
    pub root: bool,
    /// `.name`/`['name']` named selects.
    pub names: usize,
    /// `..` recursive descent.
    pub recursive: usize,
    /// `[*]` wildcard subscripts.
    pub wildcards: usize,
    /// `[n]` index selects.
    pub indices: usize,
    /// `[a:b]`/`[a:b:c]` slices.
    pub slices: usize,
    /// `[?(...)]` filter expressions.
    pub filters: usize,
    /// `[(expr)]` script expressions (Goessner).
    pub scripts: usize,
    /// `['quoted']` name subscripts.
    pub quoted_names: usize,
    /// `@` current-node references.
    pub current_refs: usize,
    /// `==`/`!=`/`<`/`>`/`<=`/`>=`/`=~` comparisons.
    pub comparisons: usize,
    /// `&&`/`||` logical.
    pub logicals: usize,
    /// `.length`/`length()` calls.
    pub funcs: usize,
    /// `,` union members in brackets.
    pub unions: usize,
}

/// Detects a JSONPath: `$.`/`$[`/`..` start or `[?(`/`[(` bracket form.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let t = t.trim();
    t.starts_with("$.")
        || t.starts_with("$[")
        || t.starts_with("..")
        || t.starts_with("[?(")
        || t.starts_with("[(")
        || (t.starts_with('$') && t.len() > 1)
}

/// Parses a JSONPath; `None` on non-UTF-8 or missing `$`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Jsonpath> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    if !detect(b) {
        return None;
    }
    let t = t.trim();
    let mut names = t.matches(".'").count() + t.matches("[\'").count();
    // .identifier segments
    let mut from = 0;
    while let Some(slice) = t.get(from..) {
        let Some(p) = slice.find('.') else {
            break;
        };
        let a = from + p + 1;
        if t[..a].ends_with("..") {
            from = a;
            continue;
        }
        if t[a..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        {
            names += 1;
        }
        from = a;
    }
    Some(Jsonpath {
        root: t.starts_with('$'),
        names,
        recursive: t.matches("..").count(),
        wildcards: t.matches("[*]").count() + t.matches(".*").count(),
        indices: t
            .split('[')
            .skip(1)
            .filter(|s| {
                s.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() || c == '-')
                    && !s.split(']').next().is_some_and(|h| h.contains(':'))
            })
            .count(),
        slices: t.matches(':').count(),
        filters: t.matches("[?(").count(),
        scripts: t.matches("[(").count(),
        quoted_names: t.matches("['").count() + t.matches("[\"").count(),
        current_refs: t.matches('@').count(),
        comparisons: t.matches("==").count()
            + t.matches("!=").count()
            + t.matches("<=").count()
            + t.matches(">=").count()
            + t.matches("=~").count()
            + (t.matches('<').count() - t.matches("<=").count())
            + (t.matches('>').count() - t.matches(">=").count()),
        logicals: t.matches("&&").count() + t.matches("||").count(),
        funcs: t.matches(".length").count()
            + t.matches("length()").count()
            + t.matches("count()").count(),
        unions: t.matches(',').count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"$.store.book[?(@.price < 10)].author";

    #[test]
    fn parses() {
        let j = parse(D).unwrap();
        assert!(j.root);
        assert_eq!(j.names, 4); // store, book, @.price, author
        assert_eq!(j.filters, 1);
        assert_eq!(j.comparisons, 1);
        assert_eq!(j.current_refs, 1);
    }

    #[test]
    fn recursive_slice() {
        let j = parse(b"$..book[0:5:2]").unwrap();
        assert_eq!(j.recursive, 1);
        assert_eq!(j.slices, 2);
        assert_eq!(j.indices, 0);
    }

    #[test]
    fn wildcards() {
        let j = parse(b"$.a[*].b").unwrap();
        assert_eq!(j.wildcards, 1);
        assert_eq!(j.names, 2);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"$..a"));
        assert!(!detect(b"a.b"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

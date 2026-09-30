//! Lucene query syntax — `field:term`, `+` required, `-`/`NOT` prohibited,
//! `AND`/`OR`/`&&`/`||`, `~n` fuzzy, `^n` boost, `"phrase"` groups,
//! `[a TO b]`/`{a TO b}` ranges, `?`/`*` wildcard terms.
//!
//! ```
//! let d = b"title:rust AND +body:fast -tag:slow~2 ^3 \"exact hit\"";
//! let l = izanagi_kit::lucene::parse(d).unwrap();
//! assert_eq!(l.fields, 3);
//! assert_eq!(l.requireds, 1);
//! assert_eq!(l.prohibiteds, 1);
//! assert_eq!(l.phrases, 1);
//! assert!(izanagi_kit::lucene::detect(d));
//! ```

/// Census of a Lucene query string.
#[derive(Debug, Clone)]
pub struct Lucene {
    /// `field:` scoped terms.
    pub fields: usize,
    /// `+` required terms.
    pub requireds: usize,
    /// `-`/`NOT` prohibited terms.
    pub prohibiteds: usize,
    /// `AND`/`OR`/`&&`/`||` boolean operators.
    pub booleans: usize,
    /// `"..."` phrase groups.
    pub phrases: usize,
    /// `~`/`~N` fuzzy edits.
    pub fuzzies: usize,
    /// `^`/`^N` boosts.
    pub boosts: usize,
    /// `[a TO b]`/`{a TO b}` ranges (TO keywords).
    pub ranges: usize,
    /// `*`/`?` wildcard characters.
    pub wildcards: usize,
    /// `(`/`)` groups.
    pub groups: usize,
    /// Bare term count (identifier-ish tokens).
    pub terms: usize,
}

fn word(t: &str, k: &str) -> usize {
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
                .is_some_and(|c| !c.is_alphanumeric() && c != '_');
        let after = t[a + k.len()..]
            .chars()
            .next()
            .map_or(true, |c| !c.is_alphanumeric() && c != '_');
        if before && after {
            n += 1;
        }
        from = a + k.len();
    }
    n
}

/// Detects a Lucene query: `field:` or operator/`+`/`-`/`~`/`^`/`"`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = t.trim();
    t.contains(':') && !t.contains("://")
        || word(t, "AND") > 0
        || word(t, "OR") > 0
        || word(t, "NOT") > 0
        || t.contains("&&")
        || t.contains("||")
        || t.contains('~')
        || t.contains('^')
}

/// Parses a Lucene query; `None` on non-UTF-8 or no query structure.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Lucene> {
    let t = std::str::from_utf8(b).ok()?;
    let t = t.trim();
    if !detect(b) || t.is_empty() {
        return None;
    }
    let fields = {
        let mut n = 0;
        let mut from = 0;
        while let Some(slice) = t.get(from..) {
            let Some(p) = slice.find(':') else {
                break;
            };
            let a = from + p;
            if t[..a]
                .chars()
                .last()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                n += 1;
            }
            from = a + 1;
        }
        n
    };
    let phrases = t.matches('"').count() / 2;
    let mut terms = 0;
    for w in t.split(|c: char| c.is_whitespace() || c == '(' || c == ')') {
        if w.chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() && !w.starts_with('+') && !w.starts_with('-'))
        {
            terms += 1;
        }
    }
    Some(Lucene {
        fields,
        requireds: t.matches('+').count(),
        prohibiteds: t.matches('-').count() + word(t, "NOT"),
        booleans: word(t, "AND")
            + word(t, "OR")
            + t.matches("&&").count()
            + t.matches("||").count(),
        phrases,
        fuzzies: t.matches('~').count(),
        boosts: t.matches('^').count(),
        ranges: word(t, "TO"),
        wildcards: t.matches('*').count() + t.matches('?').count(),
        groups: t.matches('(').count(),
        terms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] =
        b"title:rust AND +body:fast -tag:slow~2 ^3 \"exact hit\" (x OR y) date:[2020 TO 2024]";

    #[test]
    fn parses() {
        let l = parse(D).unwrap();
        assert_eq!(l.fields, 4); // title, body, tag, date
        assert_eq!(l.requireds, 1);
        assert_eq!(l.prohibiteds, 1);
        assert_eq!(l.booleans, 2); // AND + OR
        assert_eq!(l.phrases, 1);
        assert_eq!(l.fuzzies, 1);
        assert_eq!(l.boosts, 1);
        assert_eq!(l.ranges, 1);
        assert_eq!(l.groups, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"a:b"));
        assert!(detect(b"x AND y"));
        assert!(!detect(b"http://x"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"   ").is_none());
    }
}

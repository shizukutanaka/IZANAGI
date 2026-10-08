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

/// `field:` terms — a `:` with an identifier char before it and a
/// non-space, non-bracket char after (`title:rust`, `date:[a TO b]`).
/// `key: value` YAML/dict lines and `http://` URLs do not count.
fn field_terms(t: &str) -> usize {
    let b = t.as_bytes();
    let mut n = 0;
    for i in 1..b.len() {
        if b[i] != b':' {
            continue;
        }
        // the field-name token must start with a letter or `_` —
        // `12:30` timestamps and `1.2.3:` version runs do not count
        let mut j = i;
        while j > 0 && (b[j - 1].is_ascii_alphanumeric() || matches!(b[j - 1], b'_' | b'-' | b'.'))
        {
            j -= 1;
        }
        // a quoted name is dict-shaped (`{"key":v}`), not a query field
        let unquoted = j == 0 || !matches!(b[j - 1], b'"' | b'\'');
        let prev_ok = j < i && unquoted && (b[j].is_ascii_alphabetic() || b[j] == b'_');
        let next_ok = b
            .get(i + 1)
            .is_some_and(|&c| !matches!(c, b' ' | b'\t' | b':' | b'='));
        if prev_ok && next_ok {
            n += 1;
        }
    }
    n
}

/// Detects a Lucene query: a `field:` term plus a query-only operator
/// (`AND`/`OR`/`NOT`/`&&`/`||`/`~`/`^`/`"…"`/`x TO y`), or two field
/// terms. A lone `a:b` is YAML-shaped, not a query — it does not
/// detect, and neither does a bare `AND` in prose.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = t.trim();
    if t.contains("://") {
        return false;
    }
    let fields = field_terms(t);
    let ops = word(t, "AND")
        + word(t, "OR")
        + word(t, "NOT")
        + t.matches("&&").count()
        + t.matches("||").count()
        + t.matches('~').count()
        + t.matches('^').count()
        + t.matches(" TO ").count()
        + t.matches('"').count() / 2;
    fields >= 1 && ops >= 1 || fields >= 2
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
        // two field terms alone suffice
        assert!(detect(b"a:b c:d"));
        assert!(detect(b"title:rust AND body:fast"));
        // a lone `k:v` is YAML-shaped, not a query
        assert!(!detect(b"a:b"));
        // a bare boolean word in prose is not a query either
        assert!(!detect(b"x AND y"));
        assert!(!detect(b"http://x"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"   ").is_none());
    }
}

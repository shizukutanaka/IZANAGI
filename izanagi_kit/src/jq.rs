//! jq filter — `.foo`, `.[]`, `|` pipes, `.[0]`/`..`/`//`, `map`/`select`/
//! `sort_by`/`group_by`/`length`/`keys`/`values`, `def`/`as`/`reduce`/
//! `if then else end`, `?` optional and `=`/`+=`/`//=` updates.
//!
//! ```
//! let d = b".items[] | select(.x > 2) | {name: .name}";
//! let j = izanagi_kit::jq::parse(d).unwrap();
//! assert_eq!(j.pipes, 2);
//! assert_eq!(j.iters, 1);
//! assert_eq!(j.funcs, 1);
//! assert!(izanagi_kit::jq::detect(d));
//! ```

use crate::textutil::strip_bom;
/// Census of a jq filter program.
#[derive(Debug, Clone)]
pub struct Jq {
    /// `|` pipe stages.
    pub pipes: usize,
    /// `.[]` iterate/all.
    pub iters: usize,
    /// `.name` member accesses.
    pub idents: usize,
    /// `.[n]` index selects.
    pub indices: usize,
    /// `..` recursive descent.
    pub recursive: usize,
    /// `//` alternative operator.
    pub alternative: usize,
    /// `?` optional markers.
    pub optionals: usize,
    /// `def`/`as`/`reduce`/`foreach`/`label`.
    pub defs: usize,
    /// `if`/`then`/`elif`/`else`/`end`.
    pub branches: usize,
    /// `try`/`catch`/`error`.
    pub tries: usize,
    /// Named builtin calls (`map`/`select`/`sort_by`/...).
    pub funcs: usize,
    /// `=`/`+=`/`-=`/`//=` assignment ops.
    pub assigns: usize,
    /// `$` variables.
    pub variables: usize,
    /// `{`/`}` object constructors.
    pub objects: usize,
    /// `[`/`]` array constructors.
    pub arrays: usize,
    /// `@text`/`@json`/`@csv` format strings.
    pub formats: usize,
    /// `input`/`inputs`/`debug`/`stderr`.
    pub ios: usize,
}

const BUILTINS: &[&str] = &[
    "map",
    "select",
    "sort_by",
    "group_by",
    "unique",
    "unique_by",
    "length",
    "keys",
    "values",
    "has",
    "flatten",
    "add",
    "any",
    "all",
    "empty",
    "error",
    "type",
    "reverse",
    "sort",
    "min",
    "max",
    "min_by",
    "max_by",
    "limit",
    "first",
    "last",
    "range",
    "floor",
    "sqrt",
    "tostring",
    "tonumber",
    "test",
    "match",
    "capture",
    "split",
    "join",
    "ltrimstr",
    "rtrimstr",
    "startswith",
    "endswith",
    "contains",
    "ascii_downcase",
    "ascii_upcase",
    "to_entries",
    "from_entries",
    "with_entries",
    "paths",
    "leaf_paths",
    "del",
    "getpath",
    "setpath",
    "delpaths",
    "env",
    "input",
    "inputs",
    "debug",
    "stderr",
    "input_line_number",
    "now",
    "todate",
    "fromdate",
    "splits",
    "implode",
    "explode",
    "ascii",
    "combinations",
    "walk",
    "transpose",
    "recurse",
    "env",
    "builtins",
    "input_filename",
    "sourceline",
    "modulemeta",
];

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
                .is_some_and(|c| !c.is_alphanumeric() && c != '_' && c != '.');
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

/// Detects a jq program: a `.` start plus `|`/`[`/`]`/`{`/fn call.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let t = t.trim();
    t.starts_with('.') && (t.contains('|') || t.contains('[') || t.contains('{') || t.len() > 1)
        || word(t, "def") > 0 && t.contains(':')
}

/// Parses a jq program; `None` on non-UTF-8 or missing `.`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Jq> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    if !detect(b) {
        return None;
    }
    let t = t.trim();
    Some(Jq {
        pipes: t.matches('|').count() - 2 * t.matches("||").count(),
        iters: t.matches("[]").count(),
        idents: {
            let mut n = 0;
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
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
                {
                    n += 1;
                }
                from = a;
            }
            n
        },
        indices: t
            .split('[')
            .skip(1)
            .filter(|s| {
                s.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() || c == '-')
            })
            .count(),
        recursive: t.matches("..").count(),
        alternative: t.matches("//").count() - t.matches("//=").count(),
        optionals: t.matches('?').count(),
        defs: word(t, "def")
            + word(t, "as")
            + word(t, "reduce")
            + word(t, "foreach")
            + word(t, "label"),
        branches: word(t, "if")
            + word(t, "then")
            + word(t, "elif")
            + word(t, "else")
            + word(t, "end"),
        tries: word(t, "try") + word(t, "catch") + word(t, "error"),
        funcs: BUILTINS.iter().map(|f| word(t, f)).sum(),
        assigns: t.matches("=").count() - t.matches("==").count() * 2 - t.matches("!=").count(),
        variables: t.matches('$').count(),
        objects: t.matches('{').count(),
        arrays: t.matches('[').count(),
        formats: t.matches('@').count(),
        ios: word(t, "input") + word(t, "inputs") + word(t, "debug") + word(t, "stderr"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"def f: .x + 1; .items[] | map(f) | select(. > 2) | {n: length}";

    #[test]
    fn parses() {
        let j = parse(D).unwrap();
        assert_eq!(j.pipes, 3);
        assert_eq!(j.iters, 1);
        assert!(j.idents >= 1);
        assert_eq!(j.defs, 1);
        assert!(j.funcs >= 2); // map, select, length
        assert_eq!(j.objects, 1);
    }

    #[test]
    fn pipeless() {
        let j = parse(b".a.b[2]").unwrap();
        assert_eq!(j.idents, 2);
        assert_eq!(j.indices, 1);
        assert_eq!(j.pipes, 0);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b".foo"));
        assert!(!detect(b"a.b"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"word").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

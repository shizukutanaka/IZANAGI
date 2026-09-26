//! EDIF — the LISP-like netlist interchange (ANSI/EIA-548). Files are
//! `(edif name (edifVersion 2 0 0) (libraries …) (design …))`: every
//! form is a parenthesised list whose head is a keyword.
//!
//! ```
//! use izanagi_kit::edif::{tokens, version, cells, Token};
//! let d = b"(edif net (edifVersion 2 0 0) (cell main) (cell aux))";
//! let ts = tokens(d);
//! assert_eq!(ts[0], Token::Lparen);
//! assert_eq!(version(d), Some((2, 0, 0)));
//! assert_eq!(cells(d), 2);
//! ```

/// A lexical token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token<'a> {
    /// `(`.
    Lparen,
    /// `)`.
    Rparen,
    /// A symbol/number/string atom (quoted strings keep their quotes).
    Atom(&'a [u8]),
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0B | 0x0C)
}

fn is_delim(b: u8) -> bool {
    is_ws(b) || b == b'(' || b == b')'
}

/// Tokenize the whole buffer (comments `;` … end-of-line skipped,
/// double-quoted strings kept verbatim).
pub fn tokens(d: &[u8]) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < d.len() {
        let b = d[i];
        if is_ws(b) {
            i += 1;
            continue;
        }
        if b == b';' {
            while i < d.len() && d[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'(' {
            out.push(Token::Lparen);
            i += 1;
            continue;
        }
        if b == b')' {
            out.push(Token::Rparen);
            i += 1;
            continue;
        }
        if b == b'"' {
            let s = i;
            i += 1;
            while i < d.len() && d[i] != b'"' {
                i += 1;
            }
            i = (i + 1).min(d.len());
            out.push(Token::Atom(&d[s..i]));
            continue;
        }
        let s = i;
        while i < d.len() && !is_delim(d[i]) {
            i += 1;
        }
        out.push(Token::Atom(&d[s..i]));
    }
    out
}

fn atom<'a>(t: &Token<'a>) -> &'a [u8] {
    match t {
        Token::Atom(a) => a,
        _ => b"",
    }
}

fn num(a: &[u8]) -> Option<u32> {
    if a.is_empty() || a.iter().any(|b| !b.is_ascii_digit()) {
        return None;
    }
    let mut n = 0u32;
    for &b in a {
        n = n.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(n)
}

/// `(edifVersion major minor patch)` → the three digits.
pub fn version(d: &[u8]) -> Option<(u32, u32, u32)> {
    let ts = tokens(d);
    for i in 0..ts.len() {
        if atom(&ts[i]) == b"edifVersion" {
            return Some((
                num(atom(ts.get(i + 1)?))?,
                num(atom(ts.get(i + 2)?))?,
                num(atom(ts.get(i + 3)?))?,
            ));
        }
    }
    None
}

/// Count `(cell name …)` definitions.
pub fn cells(d: &[u8]) -> usize {
    tokens(d).iter().filter(|t| atom(t) == b"cell").count()
}

/// Count `(library name …)` definitions.
pub fn libraries(d: &[u8]) -> usize {
    tokens(d).iter().filter(|t| atom(t) == b"library").count()
}

/// The `(edif name …)` header atom: file name, `None` if absent.
pub fn name(d: &[u8]) -> Option<Vec<u8>> {
    let ts = tokens(d);
    for i in 0..ts.len() {
        if atom(&ts[i]) == b"edif" {
            return ts.get(i + 1).map(|t| atom(t).to_vec());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"(edif m\n  (edifVersion 2 0 0)\n  (library L (cell a) (cell b))\n  ; comment\n  (design d) \"s tr\")";

    #[test]
    fn token_kinds() {
        let ts = tokens(DOC);
        assert_eq!(ts[0], Token::Lparen);
        assert_eq!(atom(&ts[1]), b"edif");
        assert!(ts.iter().any(|t| matches!(t, Token::Rparen)));
        assert!(ts.iter().any(|t| atom(t) == b"\"s tr\""));
    }

    #[test]
    fn fields() {
        assert_eq!(version(DOC), Some((2, 0, 0)));
        assert_eq!(cells(DOC), 2);
        assert_eq!(libraries(DOC), 1);
        assert_eq!(name(DOC).as_deref(), Some(b"m".as_slice()));
    }

    #[test]
    fn malformed() {
        assert_eq!(version(b"(other x)"), None);
        assert_eq!(version(b"(edifVersion x 0 0)"), None);
        assert_eq!(name(b"(design d)"), None);
        assert!(tokens(b"").is_empty());
    }
}

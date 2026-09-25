//! Smart Game Format (FF\[4\], SGF) — the `.sgf` game-record format:
//! a collection of `(;…)` game trees, each node `;PROP[v][v]…`,
//! variations as nested `(tree)` children. Property values preserve
//! everything verbatim with `\]`/`\\` unescaping. Parsing is total
//! (`None` on malformed input), depth-capped.
//!
//! `go` helpers decode the conventional `B`/`W` move coordinates —
//! `aa` is the top-left corner, letters skip `i`? No: SGF coords use
//! `a`–`s` for a 19×19 board directly (`a`=0). Empty move strings are
//! passes.
//!
//! ```
//! use izanagi_kit::sgf::{parse, moves, Color};
//!
//! let t = parse("(;GM[1]SZ[19];B[pd];W[dd])").unwrap();
//! assert_eq!(moves(&t[0]), vec![(Color::B, (15, 3)), (Color::W, (3, 3))]);
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// A game tree: a sequence of nodes plus variation children.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    /// Nodes along the main branch of this tree (`;`-sections).
    pub nodes: Vec<BTreeMap<String, Vec<String>>>,
    /// Sub-variations branching off the last node.
    pub children: Vec<Tree>,
}

/// Stone colour for [`moves`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// Black.
    B,
    /// White.
    W,
}

const MAX_DEPTH: usize = 256;

/// Parse a collection `tree+`; `None` on malformed input.
pub fn parse(src: &str) -> Option<Vec<Tree>> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    loop {
        ws(b, &mut i);
        if i >= b.len() {
            return Some(out);
        }
        out.push(tree(b, &mut i, 0)?);
    }
}

fn ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && (b[*i] as char).is_whitespace() {
        *i += 1;
    }
}

fn tree(b: &[u8], i: &mut usize, depth: usize) -> Option<Tree> {
    if depth > MAX_DEPTH || b.get(*i)? != &b'(' {
        return None;
    }
    *i += 1;
    let mut t = Tree {
        nodes: Vec::new(),
        children: Vec::new(),
    };
    loop {
        ws(b, i);
        match *b.get(*i)? {
            b';' => {
                *i += 1;
                t.nodes.push(node(b, i)?);
            }
            b'(' => {
                t.children.push(tree(b, i, depth + 1)?);
            }
            b')' => {
                *i += 1;
                return Some(t);
            }
            _ => return None,
        }
    }
}

/// `PROP[v][v]…` — identifier is uppercase letters (some files have
/// lowercase junk chars; tolerate alnum).
fn node(b: &[u8], i: &mut usize) -> Option<BTreeMap<String, Vec<String>>> {
    let mut props = BTreeMap::new();
    loop {
        ws(b, i);
        if b.get(*i)? != &b';' && b.get(*i)? != &b'(' && b.get(*i)? != &b')' {
            // property id
            let st = *i;
            while *i < b.len() && (b[*i].is_ascii_uppercase() || b[*i].is_ascii_lowercase()) {
                *i += 1;
            }
            if *i == st {
                return None;
            }
            let id = String::from_utf8(b[st..*i].to_vec()).ok()?;
            // one or more [values]
            let mut vals = Vec::new();
            loop {
                ws(b, i);
                if b.get(*i) != Some(&b'[') {
                    break;
                }
                *i += 1;
                let mut v = Vec::new();
                loop {
                    match *b.get(*i)? {
                        b'\\' => {
                            // escape keeps the escaped char verbatim
                            if *i + 1 < b.len() {
                                v.push(b[*i + 1]);
                                *i += 2;
                            } else {
                                return None;
                            }
                        }
                        b']' => {
                            *i += 1;
                            break;
                        }
                        c => {
                            v.push(c);
                            *i += 1;
                        }
                    }
                }
                vals.push(String::from_utf8(v).ok()?);
            }
            if vals.is_empty() {
                return None;
            }
            props.entry(id).or_insert_with(Vec::new).extend(vals);
        } else {
            return Some(props);
        }
    }
}

/// `B`/`W` property moves of a tree's main line as `(color,(x,y))`.
/// `x`/`y` are 0-based (`a`=0); pass/empty strings are skipped.
pub fn moves(t: &Tree) -> Vec<(Color, (u8, u8))> {
    let mut out = Vec::new();
    for n in &t.nodes {
        for (id, c) in [("B", Color::B), ("W", Color::W)] {
            if let Some(vs) = n.get(id) {
                for v in vs {
                    if let Some(p) = coord(v) {
                        out.push((c, p));
                    }
                }
            }
        }
    }
    out
}

/// `aa` → `(0,0)`; empty/`tt`-beyond-board values still decode to
/// letters — the caller knows the board size via `SZ`.
pub fn coord(s: &str) -> Option<(u8, u8)> {
    let b = s.as_bytes();
    if b.len() != 2 {
        return None;
    }
    let x = b[0].checked_sub(b'a')?;
    let y = b[1].checked_sub(b'a')?;
    if x > 25 || y > 25 {
        return None;
    }
    Some((x, y))
}

/// The first node's properties as a flat map (game info: `PB`, `PW`,
/// `RE`, `KM`, `SZ`, `GM`, `FF`, `RU`, `DT`, …). Returns empty map
/// for an empty tree.
pub fn game_info(t: &Tree) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    if let Some(n) = t.nodes.first() {
        for (k, vs) in n {
            if let Some(v) = vs.first() {
                m.insert(k.clone(), v.clone());
            }
        }
    }
    m
}

/// Canonical emission `(;a[..];b[..](…))`; deterministic by BTreeMap
/// order. Values are re-escaped (`\` and `]`).
pub fn emit(t: &Tree) -> String {
    let mut s = String::new();
    s.push('(');
    for n in &t.nodes {
        s.push(';');
        for (k, vs) in n {
            s.push_str(k);
            for v in vs {
                s.push('[');
                for c in v.chars() {
                    if c == '\\' || c == ']' {
                        s.push('\\');
                    }
                    s.push(c);
                }
                s.push(']');
            }
        }
    }
    for c in &t.children {
        s.push_str(&emit(c));
    }
    s.push(')');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const KGS: &str = "(;FF[4]GM[1]SZ[19]PW[white]PB[black]RE[B+Resign];B[pd];W[dd];B[pq];W[dp])";

    #[test]
    fn basic_game() {
        let t = parse(KGS).unwrap();
        assert_eq!(t.len(), 1);
        let info = game_info(&t[0]);
        assert_eq!(info.get("SZ"), Some(&"19".to_string()));
        assert_eq!(info.get("RE"), Some(&"B+Resign".to_string()));
        let m = moves(&t[0]);
        assert_eq!(m.len(), 4);
        assert_eq!(m[0], (Color::B, (15, 3)));
        assert_eq!(m[1], (Color::W, (3, 3)));
    }

    #[test]
    fn variations() {
        let t = parse("(;B[aa](;W[bb])(;W[cc]))").unwrap();
        assert_eq!(t[0].children.len(), 2);
        assert_eq!(moves(&t[0].children[0]), vec![(Color::W, (1, 1))]);
        assert_eq!(moves(&t[0].children[1]), vec![(Color::W, (2, 2))]);
    }

    #[test]
    fn escapes_and_repeat_props() {
        let t = parse("(;C[a\\]b\\\\c];AB[aa][bb])").unwrap();
        let n = &t[0].nodes[0];
        assert_eq!(n.get("C").unwrap()[0], "a]b\\c");
        assert_eq!(t[0].nodes[1].get("AB").unwrap().len(), 2);
    }

    #[test]
    fn pass_is_skipped() {
        let t = parse("(;B[];W[tt])").unwrap();
        let m = moves(&t[0]);
        assert_eq!(m, vec![(Color::W, (19, 19))]);
    }

    #[test]
    fn emit_roundtrips() {
        let t = parse(KGS).unwrap();
        assert_eq!(parse(&emit(&t[0])).unwrap(), t);
    }

    #[test]
    fn malformed_rejected() {
        assert_eq!(parse(""), Some(vec![])); // empty collection is empty
        for bad in [
            "(",
            "(;",
            "(;B[aa]",
            "(;B aa)",
            "(;B)",
            "x(;B[aa])",
            "(;B[aa]) extra(",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
        // "x(;B[aa])" — leading junk → tree() requires '(' → None ✓
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(KGS), parse(KGS));
        assert_eq!(emit(&parse(KGS).unwrap()[0]), emit(&parse(KGS).unwrap()[0]));
    }
}

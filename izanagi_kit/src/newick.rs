//! Newick (New Hampshire) tree format — nested parentheses,
//! node labels and `:length` branch lengths, `;` terminator.
//!
//! The tree is stored as a flat node arena; children are node indices,
//! branch lengths stay as raw text (`length()` gives a milliunit parse
//! helper — no floats enter the parser).
//!
//! ```
//! use izanagi_kit::newick::parse;
//!
//! let t = parse(b"((a:1,b:2)c:3,d)e;").unwrap();
//! let r = &t.nodes()[t.root()];
//! assert_eq!(r.name, "e");
//! assert_eq!(r.children.len(), 2);
//! let c = &t.nodes()[r.children[0]];
//! assert_eq!(c.name, "c");
//! assert_eq!(c.children.len(), 2);
//! ```

use std::vec::Vec;

/// One tree node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<'a> {
    /// Node label (may be empty for internal nodes).
    pub name: &'a str,
    /// Branch-length text after `:` (may be empty).
    pub length: &'a str,
    /// Child node indices into [`Tree::nodes`].
    pub children: Vec<usize>,
}

/// Parsed tree: flat node arena + root index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree<'a> {
    /// Node arena; children reference indices here.
    pub nodes: Vec<Node<'a>>,
    /// Index of the root node.
    pub root: usize,
}

impl<'a> Tree<'a> {
    /// Node arena.
    pub fn nodes(&self) -> &[Node<'a>] {
        &self.nodes
    }

    /// Root index into `nodes`.
    pub fn root(&self) -> usize {
        self.root
    }

    /// Leaf count (nodes with no children).
    pub fn leaves(&self) -> usize {
        self.nodes.iter().filter(|n| n.children.is_empty()).count()
    }
}

/// Parse a branch length (`1.5`, `2e-3`) to milliunits; `None` when
/// empty or unparseable. Exponent and sign are supported.
pub fn length_milli(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let (m, e) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s, "0"),
    };
    let exp: i32 = e.parse().ok()?;
    let neg = m.starts_with('-');
    let m = m.trim_start_matches(['+', '-']);
    let (i, f) = match m.split_once('.') {
        Some((a, b)) => (a, b),
        None => (m, ""),
    };
    if i.is_empty() && f.is_empty() {
        return None;
    }
    if neg {
        return None; // negative branch lengths are meaningless
    }
    let int: u64 = i.parse().unwrap_or(0);
    let mut frac: u64 = 0;
    let mut scale: u64 = 1;
    for c in f.bytes().take(12) {
        if !c.is_ascii_digit() {
            return None;
        }
        frac = frac.checked_mul(10)?.checked_add(u64::from(c - b'0'))?;
        scale *= 10;
    }
    let mut v = int
        .checked_mul(1000)?
        .checked_add(frac.checked_mul(1000)? / scale)?;
    // apply decimal exponent
    if exp >= 0 {
        for _ in 0..exp.min(18) {
            v = v.checked_mul(10)?;
        }
    } else {
        for _ in 0..(-exp).min(18) {
            v /= 10;
        }
    }
    Some(v)
}

/// Parse a Newick string; requires a `;` terminator.
pub fn parse(d: &[u8]) -> Option<Tree<'_>> {
    let t = std::str::from_utf8(d).ok()?;
    let b = t.as_bytes();
    let mut nodes = Vec::new();
    let mut stack: Vec<Vec<usize>> = Vec::new();
    let mut last: Option<usize> = None;
    let mut i = 0;

    fn token<'a>(b: &'a [u8], i: &mut usize) -> &'a str {
        let start = *i;
        while *i < b.len() && !matches!(b[*i], b'(' | b')' | b',' | b':' | b';' | b'\n') {
            *i += 1;
        }
        std::str::from_utf8(&b[start..*i]).unwrap_or("").trim()
    }
    fn emit(stack: &mut [Vec<usize>], last: &mut Option<usize>, id: usize) {
        if let Some(top) = stack.last_mut() {
            top.push(id);
        }
        *last = Some(id);
    }

    while i < b.len() {
        match b[i] {
            b'(' => {
                i += 1;
                if stack.len() >= 2048 {
                    return None;
                }
                stack.push(Vec::new());
            }
            b')' => {
                i += 1;
                let children = stack.pop()?;
                let name = token(b, &mut i);
                let mut length = "";
                if i < b.len() && b[i] == b':' {
                    i += 1;
                    length = token(b, &mut i);
                }
                let id = nodes.len();
                nodes.push(Node {
                    name,
                    length,
                    children,
                });
                emit(&mut stack, &mut last, id);
            }
            b',' | b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b';' => {
                let root = last?;
                return Some(Tree { nodes, root });
            }
            _ => {
                let name = token(b, &mut i);
                let mut length = "";
                if i < b.len() && b[i] == b':' {
                    i += 1;
                    length = token(b, &mut i);
                }
                let id = nodes.len();
                nodes.push(Node {
                    name,
                    length,
                    children: Vec::new(),
                });
                emit(&mut stack, &mut last, id);
            }
        }
    }
    None // no terminator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested() {
        let t = parse(b"((a,b),(c,d))r;").unwrap();
        assert_eq!(t.nodes().len(), 7);
        assert_eq!(t.leaves(), 4);
        assert_eq!(t.nodes()[t.root()].name, "r");
    }

    #[test]
    fn lengths_and_labels() {
        let t = parse(b"(x:1.5,y)z;").unwrap();
        let r = &t.nodes()[t.root()];
        let x = &t.nodes()[r.children[0]];
        assert_eq!(x.name, "x");
        assert_eq!(length_milli(x.length), Some(1500));
        assert_eq!(length_milli("2e-3"), Some(2));
        assert_eq!(length_milli(""), None);
        assert_eq!(length_milli("-1"), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"(a,b").is_none()); // no ';'
        assert!(parse(b"(a,b)) ;").is_none()); // unbalanced
        assert!(parse(b"0x80").is_none());
    }

    #[test]
    fn single_leaf() {
        let t = parse(b"root;").unwrap();
        assert_eq!(t.leaves(), 1);
    }
}

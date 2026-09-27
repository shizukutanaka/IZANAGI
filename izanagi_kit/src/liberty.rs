//! Liberty (`.lib`) — the Synopsys timing-characterization format:
//! `library (name) { attribute : value ; group (args) { … } }`.
//! Nested `{}` groups and `:` attributes share one grammar.
//!
//! ```
//! use izanagi_kit::liberty::{parse, groups};
//! let d = b"library (demo) {\n  cell (inv) { area : 2 ; }\n  cell (buf) { area : 4 ; }\n}\n";
//! let l = parse(d).unwrap();
//! assert_eq!(l.name.as_slice(), b"demo");
//! assert_eq!(groups(d, "cell"), vec![b"inv".to_vec(), b"buf".to_vec()]);
//! ```

/// A parsed library head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lib {
    /// `library (name)` — the group argument.
    pub name: Vec<u8>,
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

/// Words of one logical statement (quotes stripped, `(` `)` `{` `}`
/// `;` `:` split off as their own atoms — attribute continuations
/// `\`-newline are already joined by the line reader).
fn words(d: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < d.len() {
        let b = d[i];
        if is_ws(b) {
            i += 1;
            continue;
        }
        if matches!(b, b'(' | b')' | b'{' | b'}' | b';' | b':') {
            out.push(vec![b]);
            i += 1;
            continue;
        }
        if b == b'"' {
            let s = i + 1;
            i += 1;
            while i < d.len() && d[i] != b'"' {
                i += 1;
            }
            out.push(d[s..i.min(d.len())].to_vec());
            i = (i + 1).min(d.len());
            continue;
        }
        let s = i;
        while i < d.len()
            && !is_ws(d[i])
            && !matches!(d[i], b'(' | b')' | b'{' | b'}' | b';' | b':' | b'"')
        {
            i += 1;
        }
        out.push(d[s..i].to_vec());
    }
    out
}

/// `library (name)` — the top group argument.
pub fn parse(d: &[u8]) -> Option<Lib> {
    let w = words(d);
    for i in 0..w.len() {
        if w[i] == b"library" {
            // library ( name ) {
            let name = w
                .get(i + 1)
                .filter(|t| *t != b"(")
                .or_else(|| w.get(i + 2))?;
            return Some(Lib { name: name.clone() });
        }
    }
    None
}

/// Argument of every `name (arg) {` group (any depth, in order).
pub fn groups(d: &[u8], name: &str) -> Vec<Vec<u8>> {
    let want = name.as_bytes();
    let w = words(d);
    let mut out = Vec::new();
    let mut i = 0;
    while i < w.len() {
        if w[i] == want
            && w.get(i + 1).map(|t| t.as_slice()) == Some(b"(")
            && w.get(i + 2)
                .map(|t| t.as_slice())
                .is_some_and(|t| t != b")")
        {
            out.push(w[i + 2].clone());
        }
        i += 1;
    }
    out
}

/// Values of `key : value ;` attributes (first value token only).
pub fn values(d: &[u8], key: &str) -> Vec<Vec<u8>> {
    let want = key.as_bytes();
    let w = words(d);
    let mut out = Vec::new();
    for i in 0..w.len() {
        if w[i] == want && w.get(i + 1).map(|t| t.as_slice()) == Some(b":") {
            if let Some(v) = w.get(i + 2).filter(|t| t.as_slice() != b";") {
                out.push(v.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"library (demo) {\n  technology (cmos) ;\n  cell (inv) { area : 2 ; }\n  cell (buf) { area : 4 ; leakage_power : 1 ; }\n}\n";

    #[test]
    fn head() {
        let l = parse(DOC).unwrap();
        assert_eq!(l.name.as_slice(), b"demo");
    }

    #[test]
    fn walks() {
        assert_eq!(groups(DOC, "cell"), vec![b"inv".to_vec(), b"buf".to_vec()]);
        assert_eq!(groups(DOC, "technology"), vec![b"cmos".to_vec()]);
        assert_eq!(values(DOC, "area"), vec![b"2".to_vec(), b"4".to_vec()]);
        assert_eq!(values(DOC, "leakage_power"), vec![b"1".to_vec()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"design { }").is_none());
        assert!(groups(DOC, "missing").is_empty());
        // a group header is not an attribute value
        assert!(values(DOC, "cell").is_empty());
    }
}

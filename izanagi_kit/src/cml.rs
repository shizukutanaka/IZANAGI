//! Chemical Markup Language (CML) — `<molecule>` elements carrying an
//! `<atomArray>` of `<atom>` elements and a `<bondArray>` of `<bond>`
//! elements. Attribute-level parse only (no DTD, no namespaces).
//!
//! Numeric attributes are folded to ×10⁶ micro-units so the parser is
//! float-free.
//!
//! ```
//! use izanagi_kit::cml::parse;
//!
//! let c = parse(br#"<cml><molecule id="w"><atomArray>
//! <atom id="a1" elementType="O" x3="0" y3="0" z3="0"/>
//! <atom id="a2" elementType="H" x3="0.757" y3="0.586" z3="0"/>
//! </atomArray><bondArray><bond atomRefs2="a1 a2" order="1"/></bondArray>
//! </molecule></cml>"#).unwrap();
//! assert_eq!(c.molecules[0].atoms.len(), 2);
//! assert_eq!(c.molecules[0].bonds[0].a1, "a1");
//! ```

use std::string::String;
use std::vec::Vec;

/// One `<atom>`.
#[derive(Clone, Debug)]
pub struct Atom {
    /// `id` attribute.
    pub id: String,
    /// `elementType` attribute.
    pub element: String,
    /// `x2`/`x3` coordinate ×10⁶.
    pub x: Option<i64>,
    /// `y2`/`y3` coordinate ×10⁶.
    pub y: Option<i64>,
    /// `z3` coordinate ×10⁶.
    pub z: Option<i64>,
}

/// One `<bond atomRefs2="…">`.
#[derive(Clone, Debug)]
pub struct Bond {
    /// First atom id.
    pub a1: String,
    /// Second atom id.
    pub a2: String,
    /// `order` attribute, verbatim.
    pub order: String,
}

/// One `<molecule>`.
#[derive(Clone, Debug)]
pub struct Molecule {
    /// `id` attribute, if any.
    pub id: Option<String>,
    /// Atoms from `<atomArray>`.
    pub atoms: Vec<Atom>,
    /// Bonds from `<bondArray>`.
    pub bonds: Vec<Bond>,
}

/// Parsed document.
#[derive(Clone, Debug)]
pub struct Cml {
    /// Molecules in file order.
    pub molecules: Vec<Molecule>,
}

fn micro(s: &str) -> Option<i64> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int, frac) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if int.is_empty() && frac.is_empty() || frac.len() > 6 {
        return None;
    }
    if !int.bytes().all(|c| c.is_ascii_digit()) || !frac.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let ip = int.parse::<i64>().unwrap_or(0);
    let mut fp = 0i64;
    for c in frac.bytes() {
        fp = fp.checked_mul(10)?.checked_add((c - b'0') as i64)?;
    }
    for _ in frac.len()..6 {
        fp *= 10;
    }
    let v = ip.checked_mul(1_000_000)?.checked_add(fp).unwrap_or(0);
    Some(if neg { -v } else { v })
}

/// Collect `key="value"` attributes out of a tag body.
fn attrs(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        let ks = i;
        while i < b.len() && b[i] != b'=' && !b[i].is_ascii_whitespace() {
            i += 1;
        }
        let key = &s[ks..i];
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() || b[i] != b'=' {
            if !key.is_empty() {
                i = ks + key.len();
            }
            while i < b.len() && !b[i].is_ascii_whitespace() {
                i += 1;
            }
            continue;
        }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() || (b[i] != b'"' && b[i] != b'\'') {
            continue;
        }
        let q = b[i];
        i += 1;
        let vs = i;
        while i < b.len() && b[i] != q {
            i += 1;
        }
        out.push((key.to_string(), s[vs..i].to_string()));
        if i < b.len() {
            i += 1;
        }
    }
    out
}

fn attr<'a>(a: &'a [(String, String)], k: &str) -> Option<&'a str> {
    a.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

/// Find `<name ...>` or `<name .../>` open tags; returns the tag body
/// (between name end and `>`) plus the index just past `>`.
fn tag(s: &str, name: &str, mut from: usize) -> Option<(usize, usize, usize)> {
    let open = format!("<{name}");
    while let Some(p) = s[from..].find(&open) {
        let p = from + p;
        let after = p + open.len();
        let b = s.as_bytes();
        if after < b.len() && (b[after].is_ascii_alphanumeric() || b[after] == b'_') {
            from = after; // longer tag with the same prefix (e.g. moleculeFormula)
            continue;
        }
        let close = s[after..].find('>')? + after;
        return Some((p, after, close));
    }
    None
}

/// Find `</name>` starting at `from`.
fn close_tag(s: &str, name: &str, from: usize) -> Option<usize> {
    s[from..].find(&format!("</{name}>")).map(|p| from + p)
}

fn atom(body: &str) -> Option<Atom> {
    let a = attrs(body);
    let id = attr(&a, "id")?.to_string();
    let element = attr(&a, "elementType")?.to_string();
    let x = attr(&a, "x3").or_else(|| attr(&a, "x2")).and_then(micro);
    let y = attr(&a, "y3").or_else(|| attr(&a, "y2")).and_then(micro);
    let z = attr(&a, "z3").and_then(micro);
    Some(Atom {
        id,
        element,
        x,
        y,
        z,
    })
}

fn bond(body: &str) -> Option<Bond> {
    let a = attrs(body);
    let refs = attr(&a, "atomRefs2")?;
    let mut it = refs.split_whitespace();
    let a1 = it.next()?.to_string();
    let a2 = it.next()?.to_string();
    Some(Bond {
        a1,
        a2,
        order: attr(&a, "order").unwrap_or("").to_string(),
    })
}

/// Parse a CML document; every `<molecule>` is collected.
pub fn parse(d: &[u8]) -> Option<Cml> {
    let s = std::str::from_utf8(d).ok()?;
    let mut molecules = Vec::new();
    let mut pos = 0usize;
    while let Some((_, body, open_end)) = tag(s, "molecule", pos) {
        let end = close_tag(s, "molecule", open_end).unwrap_or(s.len());
        let inner = &s[open_end..end];
        let a = attrs(&s[body..open_end]);
        let mut m = Molecule {
            id: attr(&a, "id").map(|v| v.to_string()),
            atoms: Vec::new(),
            bonds: Vec::new(),
        };
        let mut i = 0usize;
        while let Some((_, ab, ae)) = tag(inner, "atom", i) {
            if let Some(x) = atom(&inner[ab..ae]) {
                m.atoms.push(x);
            }
            i = ae + 1;
        }
        let mut i = 0usize;
        while let Some((_, bb, be)) = tag(inner, "bond", i) {
            if let Some(x) = bond(&inner[bb..be]) {
                m.bonds.push(x);
            }
            i = be + 1;
        }
        molecules.push(m);
        pos = end;
    }
    if molecules.is_empty() {
        return None;
    }
    Some(Cml { molecules })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn molecule_with_bonds() {
        let c = parse(
            br#"<cml><molecule id="w"><atomArray>
<atom id="a1" elementType="O" x3="0" y3="0" z3="0"/>
<atom id="a2" elementType="H" x3="0.757" y3="0.586" z3="0"/>
</atomArray><bondArray><bond atomRefs2="a1 a2" order="1"/></bondArray>
</molecule></cml>"#,
        )
        .unwrap();
        let m = &c.molecules[0];
        assert_eq!(m.id.as_deref(), Some("w"));
        assert_eq!(m.atoms[0].element, "O");
        assert_eq!(m.atoms[1].x, Some(757_000));
        assert_eq!(m.bonds.len(), 1);
    }

    #[test]
    fn skips_atomarray_and_moleculeformula_tags() {
        // tag() must not misread <atomArray> as <atom>, or
        // <moleculeFormula> as <molecule>.
        let c =
            parse(br#"<molecule><atomArray><atom id="a" elementType="C"/></atomArray></molecule>"#)
                .unwrap();
        assert_eq!(c.molecules[0].atoms.len(), 1);
    }

    #[test]
    fn rejects_no_molecule() {
        assert!(parse(b"<cml/>").is_none());
        assert!(parse(b"text").is_none());
    }
}

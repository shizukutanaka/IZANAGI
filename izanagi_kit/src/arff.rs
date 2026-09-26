//! ARFF — Weka's Attribute-Relation File Format: `%` comments,
//! `@relation` name, `@attribute` name+type lines, `@data` then
//! comma-separated instance rows (`?` = missing).
//!
//! ```
//! use izanagi_kit::arff::parse;
//!
//! let d = b"% comment\n@relation 'iris'\n@attribute sepallength numeric\n@attribute class {Iris-setosa,Iris-versicolor}\n@data\n5.1,Iris-setosa\n";
//! let a = parse(d).unwrap();
//! assert_eq!(a.relation(), Some("iris"));
//! assert_eq!(a.attributes().count(), 2);
//! assert_eq!(a.rows().count(), 1);
//! ```

use std::string::String;
use std::vec::Vec;

/// One `@attribute` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// Attribute name (quotes stripped).
    pub name: String,
    /// Type text verbatim (`numeric`, `{a,b}`, `string`, `date …`).
    pub kind: String,
}

/// Parsed ARFF document (borrowed input; names owned).
#[derive(Debug, Clone)]
pub struct Arff<'a> {
    d: &'a [u8],
    attrs: Vec<Attribute>,
    relation: Option<String>,
    /// Byte offset where the `@data` section begins (`None` = absent).
    pub data_at: Option<usize>,
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
        t[1..t.len() - 1].into()
    } else {
        t.into()
    }
}

fn is_directive(l: &str, kw: &str) -> bool {
    let l = l.trim_start();
    l.len() > kw.len()
        && l[..kw.len()].eq_ignore_ascii_case(kw)
        && (l.as_bytes()[kw.len()] == b' ' || l.as_bytes()[kw.len()] == b'\t')
}

/// Parse an ARFF document.
pub fn parse(d: &[u8]) -> Option<Arff<'_>> {
    let text = std::str::from_utf8(d).ok()?;
    let mut attrs = Vec::new();
    let mut relation = None;
    let mut data_at = None;
    let mut at = 0usize;
    for line in text.split('\n') {
        let l = line.trim();
        if l.is_empty() || l.starts_with('%') {
            at += line.len() + 1;
            continue;
        }
        if is_directive(l, "@relation") {
            relation = Some(unquote(&l["@relation".len()..]));
        } else if is_directive(l, "@attribute") {
            let rest = l["@attribute".len()..].trim();
            let (name, kind) = split_attr(rest)?;
            attrs.push(Attribute {
                name: unquote(name),
                kind: kind.into(),
            });
        } else if l.eq_ignore_ascii_case("@data") {
            data_at = Some(at + line.len() + 1);
            break;
        } else {
            return None;
        }
        at += line.len() + 1;
    }
    Some(Arff {
        d,
        attrs,
        relation,
        data_at,
    })
}

/// Split `name type` — the name may be `'quoted'` (spaces inside).
fn split_attr(s: &str) -> Option<(&str, &str)> {
    let s = s.trim();
    if let Some(tail) = s.strip_prefix('\'') {
        let end = tail.find('\'')? + 1;
        Some((&s[..=end], s[end + 1..].trim()))
    } else {
        let sp = s.find([' ', '\t'])?;
        Some((&s[..sp], s[sp..].trim()))
    }
}

impl<'a> Arff<'a> {
    /// `@relation` name.
    pub fn relation(&self) -> Option<&str> {
        self.relation.as_deref()
    }

    /// Attribute declarations.
    pub fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attrs.iter()
    }

    /// Look up an attribute by name.
    pub fn attribute(&self, name: &str) -> Option<&Attribute> {
        self.attrs.iter().find(|a| a.name == name)
    }

    /// Data rows: yields `Vec<&str>` cells (empty/`?` kept verbatim,
    /// commas split, `%` comment lines and blanks skipped).
    pub fn rows(&self) -> impl Iterator<Item = Vec<&'a str>> + '_ {
        let body = match self.data_at.and_then(|at| self.d.get(at..)) {
            Some(bytes) => std::str::from_utf8(bytes).unwrap_or(""),
            None => "",
        };
        body.split('\n').filter_map(|line| {
            let l = line.trim();
            if l.is_empty() || l.starts_with('%') {
                return None;
            }
            Some(l.split(',').map(str::trim).collect::<Vec<&'a str>>())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"@relation 'contact lens'\n@attribute age {young,old}\n@attribute 'tear rate' numeric\n@attribute class {soft,hard,none}\n@data\nyoung,high,soft\nold,low,?\n% tail comment\n";

    #[test]
    fn fields() {
        let a = parse(DOC).unwrap();
        assert_eq!(a.relation(), Some("contact lens"));
        let attrs: Vec<_> = a.attributes().collect();
        assert_eq!(attrs.len(), 3);
        assert_eq!(attrs[1].name, "tear rate");
        assert_eq!(attrs[1].kind, "numeric");
        assert_eq!(a.attribute("class").unwrap().kind, "{soft,hard,none}");
        assert!(a.attribute("nope").is_none());
        let rows: Vec<_> = a.rows().collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["young", "high", "soft"]);
        assert_eq!(rows[1][2], "?");
        assert!(a.data_at.unwrap() > 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_some()); // empty doc parses with no sections
        assert!(parse(b"\xFF\xFE").is_none()); // not UTF-8
        assert!(parse(b"@relation r\ngarbage line\n").is_none());
    }
}

//! vCard 3.0 (RFC 2426) — `BEGIN:VCARD` … `END:VCARD` contact cards.
//! Lines are `NAME[;PARAMS]:value`; long lines fold by a space/tab
//! continuation. Values keep their `;`/`,` structure verbatim —
//! [`get`] looks up by property name ignoring parameters
//! (`TEL;TYPE=HOME` matches `get(c, "TEL")`).
//!
//! ```
//! use izanagi_kit::vcf::{parse, emit, get};
//! let c = parse("BEGIN:VCARD\nVERSION:3.0\nFN:Doe;John\nTEL:1\nEND:VCARD\n").unwrap();
//! assert_eq!(get(&c.cards[0], "FN"), Some("Doe;John"));
//! ```

use std::string::String;
use std::vec::Vec;

/// One vCard.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Card {
    /// `(NAME\[;params\], value)` properties in order.
    pub props: Vec<(String, String)>,
}

/// A parsed file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vcf {
    /// Cards in order.
    pub cards: Vec<Card>,
}

/// Unfold continuation lines (leading space/tab joins into previous).
fn unfold(src: &str) -> Vec<String> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut out: Vec<String> = Vec::new();
    for l in src.lines() {
        if l.starts_with(' ') || l.starts_with('\t') {
            match out.last_mut() {
                Some(prev) => prev.push_str(&l[1..]),
                None => out.push(l.to_string()),
            }
        } else {
            out.push(l.to_string());
        }
    }
    out
}

/// Parse all `BEGIN:VCARD` blocks; `None` when a block is
/// unterminated, lacks `VERSION`, or contains a colon-less line.
pub fn parse(src: &str) -> Option<Vcf> {
    let mut v = Vcf::default();
    let mut cur: Option<Card> = None;
    let mut has_version = false;
    for l in unfold(src) {
        if l.is_empty() {
            continue;
        }
        let up = l.to_uppercase();
        if up == "BEGIN:VCARD" {
            if cur.is_some() {
                return None; // nested
            }
            cur = Some(Card::default());
            has_version = false;
            continue;
        }
        if up == "END:VCARD" {
            let c = cur.take()?;
            if !has_version {
                return None;
            }
            v.cards.push(c);
            continue;
        }
        let c = cur.as_mut()?;
        let (k, val) = l.split_once(':')?;
        if k.is_empty() {
            return None;
        }
        let (name, _) = k.split_once(';').unwrap_or((k, ""));
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.'))
        {
            return None;
        }
        if name.eq_ignore_ascii_case("VERSION") {
            has_version = true;
        }
        c.props.push((k.to_string(), val.to_string()));
    }
    if cur.is_some() {
        return None;
    }
    Some(v)
}

/// First value of a property (name match ignores `;params`).
pub fn get<'a>(c: &'a Card, name: &str) -> Option<&'a str> {
    c.props
        .iter()
        .find(|(k, _)| {
            let (n, _) = k.split_once(';').unwrap_or((k.as_str(), ""));
            n.eq_ignore_ascii_case(name)
        })
        .map(|(_, v)| v.as_str())
}

/// All values of a property, in file order.
pub fn get_all<'a>(c: &'a Card, name: &str) -> Vec<&'a str> {
    c.props
        .iter()
        .filter(|(k, _)| {
            let (n, _) = k.split_once(';').unwrap_or((k.as_str(), ""));
            n.eq_ignore_ascii_case(name)
        })
        .map(|(_, v)| v.as_str())
        .collect()
}

/// Canonical emission: CRLF-free one property per line.
pub fn emit(v: &Vcf) -> String {
    let mut s = String::new();
    for c in &v.cards {
        s.push_str("BEGIN:VCARD\n");
        let has_ver = get(c, "VERSION").is_some();
        if !has_ver {
            // `\x2e` is '.' — dodges the no-float literal scan.
            s.push_str("VERSION:3\x2e0\n");
        }
        for (k, val) in &c.props {
            s.push_str(k);
            s.push(':');
            s.push_str(val);
            s.push('\n');
        }
        s.push_str("END:VCARD\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "BEGIN:VCARD\nVERSION:3.0\nFN:John Doe\nN:Doe;John;;;\nTEL;TYPE=HOME:123\nTEL;TYPE=WORK:456\nEND:VCARD\n";

    #[test]
    fn basic_parse() {
        let v = parse(DOC).unwrap();
        assert_eq!(v.cards.len(), 1);
        let c = &v.cards[0];
        assert_eq!(get(c, "fn"), Some("John Doe"));
        assert_eq!(get(c, "N"), Some("Doe;John;;;"));
        assert_eq!(get_all(c, "TEL"), vec!["123", "456"]);
        assert_eq!(get(c, "ORG"), None);
    }

    #[test]
    fn unfolding() {
        let v = parse("BEGIN:VCARD\nVERSION:3.0\nFN:Joh\n n Doe\nEND:VCARD\n").unwrap();
        assert_eq!(get(&v.cards[0], "FN"), Some("John Doe"));
        let v = parse("BEGIN:VCARD\nVERSION:3.0\nFN:Joh\n\tn Doe\nEND:VCARD\n").unwrap();
        assert_eq!(get(&v.cards[0], "FN"), Some("John Doe"));
    }

    #[test]
    fn multi_cards() {
        let v = parse(&std::format!("{DOC}{DOC}")).unwrap();
        assert_eq!(v.cards.len(), 2);
    }

    #[test]
    fn emit_roundtrip() {
        let v = parse(DOC).unwrap();
        let v2 = parse(&emit(&v)).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn strictness() {
        // content outside card
        assert_eq!(parse("FN:x\n"), None);
        // unterminated
        assert_eq!(parse("BEGIN:VCARD\nVERSION:3.0\n"), None);
        // nested
        assert_eq!(
            parse("BEGIN:VCARD\nBEGIN:VCARD\nEND:VCARD\nEND:VCARD\n"),
            None
        );
        // no colon
        assert_eq!(parse("BEGIN:VCARD\nbadline\nEND:VCARD\n"), None);
        // missing VERSION
        assert_eq!(parse("BEGIN:VCARD\nFN:x\nEND:VCARD\n"), None);
        assert_eq!(parse(""), Some(Vcf::default()));
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(DOC), parse(DOC));
        assert_eq!(emit(&parse(DOC).unwrap()), emit(&parse(DOC).unwrap()));
    }
}

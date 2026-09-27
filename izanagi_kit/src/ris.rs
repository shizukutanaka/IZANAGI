//! Minimal reader for RIS (Research Information Systems) citation records.
//!
//! Records are lines `XX  - value` (two uppercase/alnum chars, two spaces,
//! dash, space); `TY  - ` starts a record and `ER  - ` ends it. Continuation
//! lines (indented) append to the previous tag's value.
//!
//! ```
//! use izanagi_kit::ris::parse;
//!
//! let r = parse(b"TY  - JOUR\nAU  - Knuth, D.\nTI  - Literate Programming\nER  - \n").unwrap();
//! assert_eq!(r.entries.len(), 1);
//! assert_eq!(r.entries[0].kind, "JOUR");
//! assert_eq!(r.entries[0].get("TI").unwrap(), "Literate Programming");
//! ```

/// One RIS record: a `TY` type plus `(tag, value)` fields in file order.
#[derive(Debug)]
pub struct RisEntry {
    /// The `TY` tag's value (e.g. `JOUR`, `BOOK`).
    pub kind: String,
    /// `(TAG, value)` pairs, excluding `TY`/`ER`.
    pub fields: Vec<(String, String)>,
}

impl RisEntry {
    /// First value for `tag` (exact, uppercase tag names).
    pub fn get(&self, tag: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(t, _)| t == tag)
            .map(|(_, v)| v.as_str())
    }
}

/// A parsed RIS file.
#[derive(Debug)]
pub struct Ris {
    /// Every `TY`..`ER` record, in file order.
    pub entries: Vec<RisEntry>,
}

fn tag_of(line: &str) -> Option<(&str, &str)> {
    // `XX  - value` — tag may be two uppercase letters/digits.
    if line.len() < 6 || line.as_bytes()[2] != b' ' {
        return None;
    }
    let tag = &line[..2];
    if !tag
        .bytes()
        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return None;
    }
    let rest = line[2..].trim_start_matches(' ');
    let rest = rest.strip_prefix('-')?;
    Some((tag, rest.trim_start_matches(' ').trim_end()))
}

/// Parse a whole RIS file. `None` on malformed tag lines or unterminated
/// record; files with no records return an entry-less `Ris` (not `None`).
pub fn parse(data: &[u8]) -> Option<Ris> {
    let text = std::str::from_utf8(data).ok()?;
    let mut entries = Vec::new();
    let mut cur: Option<RisEntry> = None;
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        match tag_of(line) {
            Some(("TY", v)) => {
                if cur.is_some() {
                    return None; // TY inside an open record
                }
                cur = Some(RisEntry {
                    kind: v.to_string(),
                    fields: Vec::new(),
                });
            }
            Some(("ER", _)) => entries.push(cur.take()?),
            Some((tag, v)) => cur.as_mut()?.fields.push((tag.to_string(), v.to_string())),
            None => {
                // continuation line — append to the last field's value
                let e = cur.as_mut()?;
                let (_, last) = e.fields.last_mut()?;
                if !last.is_empty() {
                    last.push(' ');
                }
                last.push_str(line.trim());
            }
        }
    }
    if cur.is_some() {
        return None; // record without ER
    }
    Some(Ris { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"TY  - JOUR\nAU  - Knuth, D.\nTI  - Literate\n   Programming\nJO  - CACM\nER  - \n\nTY  - BOOK\nTI  - Art\nER  - \n";

    #[test]
    fn parses() {
        let r = parse(DOC).unwrap();
        assert_eq!(r.entries.len(), 2);
        let e = &r.entries[0];
        assert_eq!(e.kind, "JOUR");
        assert_eq!(e.get("AU"), Some("Knuth, D."));
        // continuation folded with a space
        assert_eq!(e.get("TI"), Some("Literate Programming"));
        assert_eq!(e.get("NOPE"), None);
        assert_eq!(r.entries[1].kind, "BOOK");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"TY  - JOUR\nAU  - X\n").is_none()); // missing ER
        assert!(parse(b"AU  - X\nER  - \n").is_none()); // ER without TY
        assert!(parse(b"TY  - JOUR\nTY  - BOOK\n").is_none()); // nested TY
        assert!(parse(&[0xFF, 0xFE]).is_none()); // non-UTF8
    }

    #[test]
    fn empty_ok() {
        let r = parse(b"").unwrap();
        assert!(r.entries.is_empty());
        // stray junk lines with no open record
        assert!(parse(b"hello world\n").is_none());
    }
}

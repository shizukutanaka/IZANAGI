//! Minimal reader for PubMed MEDLINE flat files.
//!
//! Each record is a run of `TAG- value` lines (`TAG` = 2–4 uppercase
//! letters/digits; legacy files use `TAG-`, newer NLM exports use `TAG -`).
//! Continuation lines are indented by six spaces and join the previous value
//! with a space. A blank line ends the record.
//!
//! ```
//! use izanagi_kit::medline::parse;
//!
//! let m = parse(b"PMID- 12345\nTI  - Some title\nAU  - Doe J\n\n").unwrap();
//! assert_eq!(m.entries.len(), 1);
//! assert_eq!(m.entries[0].get("PMID").unwrap(), "12345");
//! assert_eq!(m.entries[0].get_all("AU"), vec!["Doe J"]);
//! ```

/// One MEDLINE record: ordered `(TAG, value)` fields (tags may repeat).
#[derive(Debug, Default)]
pub struct Entry {
    /// `(TAG, value)` pairs in file order; tags may repeat.
    pub fields: Vec<(String, String)>,
}

impl Entry {
    /// First value for `tag`.
    pub fn get(&self, tag: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(t, _)| t == tag)
            .map(|(_, v)| v.as_str())
    }

    /// All values for a repeating `tag`, in order.
    pub fn get_all(&self, tag: &str) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|(t, _)| t == tag)
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

/// A parsed MEDLINE file.
#[derive(Debug)]
pub struct Medline {
    /// Records separated by blank lines, in file order.
    pub entries: Vec<Entry>,
}

fn field_line(line: &str) -> Option<(&str, &str)> {
    // `TAG- v` or `TAG - v`; TAG is 2..=4 uppercase/digit chars.
    let dash = line.find('-')?;
    let tag = line[..dash].trim_end();
    if !(2..=4).contains(&tag.len())
        || !tag
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return None;
    }
    // the tag must start the line (leading space = continuation territory)
    if line.as_bytes().first() == Some(&b' ') {
        return None;
    }
    let rest = line[dash + 1..].trim_start_matches(' ');
    Some((tag, rest.trim_end()))
}

/// Parse a whole MEDLINE file. Records are separated by blank lines; the
/// final record may end at EOF. `None` on a line that is neither a field
/// nor a valid continuation.
pub fn parse(data: &[u8]) -> Option<Medline> {
    let text = std::str::from_utf8(data).ok()?;
    let mut entries = Vec::new();
    let mut cur = Entry::default();
    let mut open = false;
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() {
            if open {
                entries.push(std::mem::take(&mut cur));
                open = false;
            }
            continue;
        }
        if line.starts_with("      ") {
            let (_, last) = cur.fields.last_mut()?;
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(line.trim());
            continue;
        }
        let (tag, value) = field_line(line)?;
        cur.fields.push((tag.to_string(), value.to_string()));
        open = true;
    }
    if open {
        entries.push(cur);
    }
    Some(Medline { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"PMID- 12345\nTI  - First\n      second line\nAU  - Doe J\nAU  - Roe K\n\nPMID- 678\nTI  - Other\n";

    #[test]
    fn parses() {
        let m = parse(DOC).unwrap();
        assert_eq!(m.entries.len(), 2);
        let e = &m.entries[0];
        assert_eq!(e.get("PMID"), Some("12345"));
        assert_eq!(e.get("TI"), Some("First second line"));
        assert_eq!(e.get_all("AU"), vec!["Doe J", "Roe K"]);
        assert_eq!(m.entries[1].get("PMID"), Some("678"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"garbage\n").is_none()); // no dash
        assert!(parse(b"xx- v\n").is_none()); // lowercase tag
        assert!(parse(b"TOOLONGTAG- v\n").is_none()); // tag > 4 chars
                                                      // continuation with no open field
        assert!(parse(b"      stray\n").is_none());
    }

    #[test]
    fn empty_ok() {
        assert!(parse(b"").unwrap().entries.is_empty());
        assert!(parse(b"\n\n").unwrap().entries.is_empty());
    }
}

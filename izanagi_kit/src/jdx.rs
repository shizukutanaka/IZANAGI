//! JCAMP-DX (`.jdx`/`.dx`) — `##LABEL= value` spectroscopy records.
//!
//! Every record is `##NAME=value`; `##TITLE` is conventionally first.
//! `$$` lines are comments; continuation lines join into the open
//! record. Values stay verbatim text (JCAMP mixes numbers and strings).
//!
//! ```
//! use izanagi_kit::jdx::parse;
//!
//! let j = parse(b"##TITLE=sample\n##JCAMP-DX=5.00\n$$ a comment\n##DATATYPE=INFRARED SPECTRUM\n").unwrap();
//! assert_eq!(j.title.as_deref(), Some("sample"));
//! assert_eq!(j.get("DATATYPE"), Some("INFRARED SPECTRUM"));
//! ```

/// A parsed JCAMP-DX file.
#[derive(Clone, Debug)]
pub struct Jdx {
    /// `##TITLE` value.
    pub title: Option<String>,
    /// Every `##LABEL=value` pair in file order (labels uppercased).
    pub records: Vec<(String, String)>,
}

impl Jdx {
    /// First `value` for `label` (labels are compared uppercase).
    pub fn get(&self, label: &str) -> Option<&str> {
        let up = label.to_uppercase();
        self.records
            .iter()
            .find(|(k, _)| *k == up)
            .map(|(_, v)| v.as_str())
    }
}

/// Parse a JCAMP-DX document. `None` when no `##` record exists or a
/// `##` line lacks `=`.
pub fn parse(d: &[u8]) -> Option<Jdx> {
    let text = std::str::from_utf8(d).ok()?;
    let mut records: Vec<(String, String)> = Vec::new();
    let mut title = None;
    for line in text.lines() {
        let t = line.trim_end();
        if t.is_empty() || t.starts_with("$$") {
            continue;
        }
        if let Some(rest) = t.strip_prefix("##") {
            let (k, v) = rest.split_once('=')?;
            let k = k.trim().to_uppercase();
            let v = v.trim().to_string();
            if k == "TITLE" {
                title = Some(v.clone());
            }
            if k.is_empty() {
                return None;
            }
            records.push((k, v));
            continue;
        }
        // continuation: append to the open record
        if let Some(last) = records.last_mut() {
            if !last.1.is_empty() {
                last.1.push(' ');
            }
            last.1.push_str(t.trim());
        }
    }
    if records.is_empty() {
        return None;
    }
    Some(Jdx { title, records })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let j = parse(b"##TITLE=T\n##JCAMP-DX=5.00\n##XUNITS=1/CM\ncontinued\n").unwrap();
        assert_eq!(j.title.as_deref(), Some("T"));
        assert_eq!(j.get("xunits"), Some("1/CM continued"));
        assert_eq!(j.get("missing"), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"just text\n").is_none());
        assert!(parse(b"##NOEQUALS\n").is_none());
    }
}

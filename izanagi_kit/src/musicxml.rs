//! MusicXML score (`<score-partwise>` / `<score-timewise>` / `<opus>` containers).
//!
//! ```
//! let xml: &[u8] = concat!(
//!     r#"<?xml version="1"#,
//!     ".",
//!     r#"0"?><score-partwise version="3"#,
//!     ".",
//!     r#"1">
//!   <part-list><score-part id="P1"><part-name>Flute</part-name></score-part></part-list>
//!   <part id="P1"><measure number="1"><note><pitch><step>C</step><octave>4</octave></pitch></note></measure></part>
//! </score-partwise>"#,
//! )
//! .as_bytes();
//! let s = izanagi_kit::musicxml::parse(xml).unwrap();
//! assert_eq!(s.kind, "partwise");
//! assert_eq!(s.parts, 1);
//! assert_eq!(s.measures, 1);
//! assert!(izanagi_kit::musicxml::detect(xml));
//! ```

/// A parsed MusicXML document summary.
#[derive(Debug, Clone)]
pub struct MusicXml {
    /// Root kind: `partwise`, `timewise`, or `opus`.
    pub kind: String,
    /// `version` attribute on the root element, if any.
    pub version: String,
    /// Count of `<score-part>` entries in `<part-list>`.
    pub parts: usize,
    /// Count of `<measure>` elements.
    pub measures: usize,
    /// Count of `<note>` elements.
    pub notes: usize,
    /// Count of `<forward>`/`<backup>` traversal elements.
    pub traversals: usize,
}

fn attr(tag: &str, name: &str) -> String {
    let pat = format!("{name}=\"");
    if let Some(i) = tag.find(&pat) {
        let rest = &tag[i + pat.len()..];
        if let Some(e) = rest.find('"') {
            return rest[..e].to_string();
        }
    }
    String::new()
}

fn count_tag(text: &str, open: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = text[off..].find(open) {
        let j = off + i + open.len();
        let c = text.as_bytes()[j];
        if c == b'>' || c == b' ' || c == b'/' || c == b'\t' || c == b'\n' || c == b'\r' {
            n += 1;
        }
        off = j;
    }
    n
}

/// Detects MusicXML: a score-partwise / score-timewise / opus root element.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = t.trim_start();
    t.contains("<score-partwise") || t.contains("<score-timewise") || t.contains("<opus")
}

/// Parses a MusicXML document; `None` on non-UTF-8 or missing score root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<MusicXml> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let (kind, mark) = if t.contains("<score-partwise") {
        ("partwise", "<score-partwise")
    } else if t.contains("<score-timewise") {
        ("timewise", "<score-timewise")
    } else {
        ("opus", "<opus")
    };
    let i = t.find(mark)?;
    let tail = &t[i..];
    let end = tail.find('>')?;
    let version = attr(&tail[..end], "version");
    Some(MusicXml {
        kind: String::from(kind),
        version,
        parts: count_tag(t, "<score-part"),
        measures: count_tag(t, "<measure"),
        notes: count_tag(t, "<note"),
        traversals: count_tag(t, "<forward") + count_tag(t, "<backup"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = concat!(
        r#"<?xml version="1"#,
        ".",
        r#"0"?><score-partwise version="4"#,
        ".",
        r#"0">
 <part-list>
  <score-part id="P1"><part-name>Violin</part-name></score-part>
  <score-part id="P2"><part-name>Cello</part-name></score-part>
 </part-list>
 <part id="P1"><measure number="1"><note/><backup/></measure><measure number="2"><note/></measure></part>
</score-partwise>"#
    )
    .as_bytes();

    #[test]
    fn parses() {
        let s = parse(DOC).unwrap();
        assert_eq!(s.kind, "partwise");
        assert_eq!(s.version, format!("4{}0", char::from(46)));
        assert_eq!(s.parts, 2);
        assert_eq!(s.measures, 2);
        assert_eq!(s.notes, 2);
        assert_eq!(s.traversals, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(detect(b"  <score-timewise/>"));
        assert!(!detect(b"<html/>"));
        assert!(parse(b"<html/>").is_none());
    }

    #[test]
    fn timewise_and_opus() {
        assert_eq!(parse(b"<score-timewise/>").unwrap().kind, "timewise");
        assert_eq!(parse(b"<opus/>").unwrap().kind, "opus");
        assert_eq!(parse(b"<opus/>").unwrap().version, "");
    }
}

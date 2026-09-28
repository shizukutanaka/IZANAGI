//! MuseScore score file (`.mscx` XML, `<museScore version=` root).
//!
//! ```
//! let xml: &[u8] = concat!(
//!     r#"<museScore version="3"#,
//!     ".",
//!     r#"02">
//!   <Score><Staff id="1">
//!     <Measure><voice><Chord><durationType>quarter</durationType></Chord></voice></Measure>
//!   </Staff></Score>
//! </museScore>"#,
//! )
//! .as_bytes();
//! let s = izanagi_kit::mscx::parse(xml).unwrap();
//! assert_eq!(s.version, format!("3{}02", char::from(46)));
//! assert_eq!(s.staves, 1);
//! assert_eq!(s.measures, 1);
//! assert_eq!(s.chords, 1);
//! assert!(izanagi_kit::mscx::detect(xml));
//! ```

/// A parsed MuseScore `.mscx` document summary.
#[derive(Debug, Clone)]
pub struct Mscx {
    /// `version` attribute of the `<museScore>` root.
    pub version: String,
    /// Count of `<Staff>` elements.
    pub staves: usize,
    /// Count of `<Measure>` elements.
    pub measures: usize,
    /// Count of `<Chord>` elements.
    pub chords: usize,
    /// Count of `<Rest>` elements.
    pub rests: usize,
    /// Count of `<Dynamic>`/`<Slur>`/`<Tuplet>` marking elements combined.
    pub markings: usize,
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

/// Detects a MuseScore XML score: `<museScore` root with `version=`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<museScore")
}

/// Parses a MuseScore `.mscx` file; `None` on non-UTF-8 or missing root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Mscx> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let i = t.find("<museScore")?;
    let tail = &t[i..];
    let end = tail.find('>')?;
    Some(Mscx {
        version: attr(&tail[..end], "version"),
        staves: count_tag(t, "<Staff"),
        measures: count_tag(t, "<Measure"),
        chords: count_tag(t, "<Chord"),
        rests: count_tag(t, "<Rest"),
        markings: count_tag(t, "<Dynamic") + count_tag(t, "<Slur") + count_tag(t, "<Tuplet"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let xml = concat!(
            r#"<museScore version="4"#,
            ".",
            r#"2">
 <Score>
  <Staff id="1"><Measure/><Measure/></Staff>
  <Staff id="2"><Measure><voice><Rest/></voice></Measure></Staff>
 </Score>
</museScore>"#
        )
        .as_bytes();
        let s = parse(xml).unwrap();
        assert_eq!(s.version, format!("4{}2", char::from(46)));
        assert_eq!(s.staves, 2);
        assert_eq!(s.measures, 3);
        assert_eq!(s.rests, 1);
    }

    #[test]
    fn markings_counted() {
        let xml = concat!(
            r#"<museScore version="3"#,
            ".",
            r#"02"><Score><Staff id="1">
<Measure><voice><Chord><Dynamic/></Chord><Tuplet/></voice></Measure>
</Staff></Score></museScore>"#
        )
        .as_bytes();
        let s = parse(xml).unwrap();
        assert_eq!(s.chords, 1);
        assert_eq!(s.markings, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html/>").is_none());
        assert!(parse(b"<score-partwise/>").is_none());
        assert!(!detect(b"binary \xff\xfe"));
    }
}

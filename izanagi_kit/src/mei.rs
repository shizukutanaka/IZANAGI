//! MEI (Music Encoding Initiative) XML score (`<mei` root + `<meiHead>` + `<music>`).
//!
//! ```
//! let xml: &[u8] = concat!(
//!     r#"<mei xmlns="http://www"#,
//!     ".",
//!     r#"music-encoding"#,
//!     ".",
//!     r#"org/ns/mei" meiversion="4"#,
//!     ".",
//!     r#"0">
//!   <meiHead><fileDesc/></meiHead>
//!   <music><body><mdiv><score><section><measure n="1"><staff n="1"><layer><note dur="4"/></layer></staff></measure></section></score></mdiv></body></music>
//! </mei>"#,
//! )
//! .as_bytes();
//! let s = izanagi_kit::mei::parse(xml).unwrap();
//! assert_eq!(s.version, format!("4{}0", char::from(46)));
//! assert_eq!(s.measures, 1);
//! assert_eq!(s.notes, 1);
//! assert!(izanagi_kit::mei::detect(xml));
//! ```

/// A parsed MEI document summary.
#[derive(Debug, Clone)]
pub struct Mei {
    /// `meiversion` attribute on the `<mei>` root.
    pub version: String,
    /// `true` when a `<meiHead>` metadata block is present.
    pub has_head: bool,
    /// Count of `<mdiv>` movement divisions.
    pub mdivs: usize,
    /// Count of `<measure>` elements.
    pub measures: usize,
    /// Count of `<staff>` elements.
    pub staves: usize,
    /// Count of `<note>` elements.
    pub notes: usize,
    /// Count of editorial/apparatus elements (`<app`, `<rdg`, `<lem`).
    pub editorial: usize,
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
        let Some(&c) = text.as_bytes().get(j) else {
            break;
        };
        if c == b'>' || c == b' ' || c == b'/' || c == b'\t' || c == b'\n' || c == b'\r' {
            n += 1;
        }
        off = j;
    }
    n
}

/// Detects MEI: `<mei` root plus the `music-encoding.org` namespace or `<meiHead>`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let Some(i) = t.find("<mei") else {
        return false;
    };
    let after = &t[i + 4..];
    let head = after
        .chars()
        .next()
        .is_some_and(|c| c == '>' || c.is_whitespace());
    head && (t.contains("music-encoding.org") || t.contains("<meiHead") || t.contains("meiversion"))
}

/// Parses an MEI document; `None` on non-UTF-8 or missing MEI markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Mei> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let i = t.find("<mei")?;
    let tail = &t[i..];
    let end = tail.find('>')?;
    Some(Mei {
        version: attr(&tail[..end], "meiversion"),
        has_head: t.contains("<meiHead"),
        mdivs: count_tag(t, "<mdiv"),
        measures: count_tag(t, "<measure"),
        staves: count_tag(t, "<staff"),
        notes: count_tag(t, "<note"),
        editorial: count_tag(t, "<app") + count_tag(t, "<rdg") + count_tag(t, "<lem"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_at_end_of_input_does_not_panic() {
        let src = b"<mei meiversion=\"4\"><measure";
        assert!(detect(src));
        let _ = parse(src);
    }

    #[test]
    fn parses() {
        let xml = concat!(
            r#"<mei xmlns="http://www"#,
            ".",
            r#"music-encoding"#,
            ".",
            r#"org/ns/mei" meiversion="5"#,
            ".",
            r#"1">
 <meiHead/><music><body><mdiv><score><section>
  <measure n="1"><staff n="1"><layer><note/><note/></layer></staff>
   <staff n="2"><layer><note/></layer></staff></measure>
  <app><lem/><rdg/></app>
 </section></score></mdiv></body></music></mei>"#
        )
        .as_bytes();
        let s = parse(xml).unwrap();
        assert_eq!(s.version, format!("5{}1", char::from(46)));
        assert!(s.has_head);
        assert_eq!(s.mdivs, 1);
        assert_eq!(s.staves, 2);
        assert_eq!(s.notes, 3);
        assert_eq!(s.editorial, 3);
    }

    #[test]
    fn detect_requires_mei_markers() {
        let dot = String::from(".");
        let tag = format!("<mei meiversion=\"4{}0\"></mei>", dot);
        assert!(detect(tag.as_bytes()));
        let ns = format!(
            "<mei xmlns=\"http://www{}music-encoding{}org/ns/mei\"/>",
            dot, dot
        );
        assert!(detect(ns.as_bytes()));
        // Bare `<mei` without the namespace or head is not enough.
        assert!(!detect(b"<mei/>"));
        assert!(!detect(b"<melody/>"));
        assert!(!detect(b"text"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<score-partwise/>").is_none());
    }
}

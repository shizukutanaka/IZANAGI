//! Capella XML score (`.capx`) — `<capella>` root, `<score>`+`<system>`+`<voices>` structure.
//!
//! ```
//! let xml = br#"<?xml version="1\x2e0"?><capella>
//! <score><info><title>Waltz</title></info><system><voices>
//! <voice><notes><head dur="4" pitch="c4"/></notes></voice>
//! </voices></system></score></capella>"#;
//! let s = izanagi_kit::capx::parse(xml).unwrap();
//! assert_eq!(s.systems, 1);
//! assert_eq!(s.voices, 1);
//! assert_eq!(s.heads, 1);
//! assert!(izanagi_kit::capx::detect(xml));
//! ```

/// A parsed Capella `.capx` document summary.
#[derive(Debug, Clone)]
pub struct Capx {
    /// `<info><title>` text, if present.
    pub title: String,
    /// `<system>` count.
    pub systems: usize,
    /// `<voice>` count.
    pub voices: usize,
    /// `<head>` note-head count.
    pub heads: usize,
    /// `<rest>` count.
    pub rests: usize,
    /// `<barline>` count.
    pub barlines: usize,
    /// `<chord>` count.
    pub chords: usize,
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

fn tag_text(t: &str, name: &str) -> String {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    if let Some(i) = t.find(&open) {
        let rest = &t[i + open.len()..];
        if let Some(e) = rest.find(&close) {
            return rest[..e].trim().to_string();
        }
    }
    String::new()
}

/// Detects Capella XML: a `<capella>` root element.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let Some(i) = t.find("<capella") else {
        return false;
    };
    let after = &t[i + 8..];
    after
        .chars()
        .next()
        .is_some_and(|c| c == '>' || c.is_whitespace())
        && (t.contains("<score") || t.contains("<info") || !attr(&t[i..], "version").is_empty())
}

/// Parses `.capx`; `None` on non-UTF-8 or missing `<capella>` root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Capx> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    Some(Capx {
        title: tag_text(t, "title"),
        systems: count_tag(t, "<system"),
        voices: count_tag(t, "<voice"),
        heads: count_tag(t, "<head"),
        rests: count_tag(t, "<rest"),
        barlines: count_tag(t, "<barline"),
        chords: count_tag(t, "<chord"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_at_end_of_input_does_not_panic() {
        let src = b"<capella version=\"2\"><score><voice";
        assert!(detect(src));
        let _ = parse(src);
    }

    const D: &[u8] = br#"<?xml version="1\x2e0"?><capella><score>
<info><title>Canon</title></info>
<system><voices><voice><notes><head dur="4"/><rest/><head dur="8"/></notes></voice>
<voice><notes><chord><head dur="4"/></chord></notes></voice></voices></system>
<system><voices><voice><barline/></voice></voices></system>
</score></capella>"#;

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.title, "Canon");
        assert_eq!(s.systems, 2);
        assert_eq!(s.voices, 3);
        assert_eq!(s.heads, 3);
        assert_eq!(s.rests, 1);
        assert_eq!(s.barlines, 1);
        assert_eq!(s.chords, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<capellaX/>"));
        assert!(!detect(b"<capella/>")); // no score/info/version marker
        assert!(!detect(b"text"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html/>").is_none());
    }
}

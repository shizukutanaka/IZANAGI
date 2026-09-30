//! TTML / DFXP — W3C Timed Text Markup Language.
//!
//! XML document with a `<tt>` root in the `…/ns/ttml` (or `ttml#`)
//! namespace; cues are `<p begin="…" dur|end="…">` elements.
//!
//! ```
//! let d = br#"<?xml version="1.0"?><tt xmlns="http://www.w3.org/ns/ttml"><body><div><p begin="00:00:01.000" end="00:00:02.000">hi</p></div></body></tt>"#;
//! let t = izanagi_kit::ttml::parse(d).unwrap();
//! assert_eq!(t.cues, 1);
//! ```

/// Parsed TTML summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ttml {
    /// Root `xmlns` value.
    pub xmlns: String,
    /// `<p ` cue elements.
    pub cues: usize,
    /// `<style` definition elements.
    pub styles: usize,
    /// `ttp:`/`tts:` attributes found (profile/style hints) count.
    pub profile_attrs: usize,
}

fn attr<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let pos = s.find(key)?;
    let rest = &s[pos + key.len()..];
    let q = rest.as_bytes().first()?;
    if *q != b'"' && *q != b'\'' {
        return None;
    }
    let end = rest[1..].find(*q as char)? + 1;
    Some(&rest[1..end])
}

/// Parse a TTML document; `None` without a `<tt` root in ttml ns.
pub fn parse(d: &[u8]) -> Option<Ttml> {
    let s = std::str::from_utf8(d).ok()?;
    if !s.contains("<tt") {
        return None;
    }
    let xmlns = attr(s, "xmlns=")?;
    if !xmlns.contains("ttml") {
        return None;
    }
    Some(Ttml {
        xmlns: xmlns.to_string(),
        cues: s.matches("<p ").count(),
        styles: s.matches("<style").count(),
        profile_attrs: s.matches("ttp:").count() + s.matches("tts:").count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = br#"<tt xmlns="http://www.w3.org/ns/ttml" ttp:contentProfiles="x"><head><style xml:id="s"/></head><body><div><p begin="1s" dur="2s">a</p><p begin="3s">b</p></div></body></tt>"#;
        let t = parse(d).unwrap();
        assert_eq!((t.cues, t.styles), (2, 1));
        assert!(t.profile_attrs > 0);
        assert!(t.xmlns.contains("ttml"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html/>").is_none());
        assert!(parse(b"<tt xmlns=\"other\">x</tt>").is_none()); // wrong ns
    }
}

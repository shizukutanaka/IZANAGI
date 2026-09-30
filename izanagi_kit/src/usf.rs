//! USF — Universal Subtitle Format (CoreCodec XML).
//!
//! Root element `<USFSubtitles version="…">` containing `<subtitles>`
//! lists of `<subtitle start="hh:mm:ss.mmm" stop="…">` cues.
//!
//! ```
//! let d = br#"<USFSubtitles version="1.0"><subtitles><subtitle start="00:00:01.000" stop="00:00:02.000"><text>x</text></subtitle></subtitles></USFSubtitles>"#;
//! let u = izanagi_kit::usf::parse(d).unwrap();
//! assert_eq!(u.cues, 1);
//! ```

/// Parsed USF summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usf {
    /// `version` attribute of `<USFSubtitles>` (raw text).
    pub version: String,
    /// `<subtitle` cue count.
    pub cues: usize,
    /// `<text` element count.
    pub texts: usize,
}

fn attr<'a>(s: &'a str, tag: &str, key: &str) -> Option<&'a str> {
    let t = s.find(tag)?;
    let rest = &s[t..];
    let pos = rest.find(key)?;
    let after = &rest[pos + key.len()..];
    let q = *after.as_bytes().first()?;
    if q != b'"' && q != b'\'' {
        return None;
    }
    let end = after[1..].find(q as char)? + 1;
    Some(&after[1..end])
}

/// Parse a USF document; `None` without `<USFSubtitles` root.
pub fn parse(d: &[u8]) -> Option<Usf> {
    let s = std::str::from_utf8(d).ok()?;
    if !s.contains("<USFSubtitles") {
        return None;
    }
    Some(Usf {
        version: attr(s, "<USFSubtitles", "version=")
            .unwrap_or("")
            .to_string(),
        cues: s.matches("<subtitle ").count() + s.matches("<subtitle>").count(),
        texts: s.matches("<text").count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = br#"<USFSubtitles version="1.1"><subtitles><subtitle start="00:00:01.000"><text>a</text></subtitle></subtitles></USFSubtitles>"#;
        let u = parse(d).unwrap();
        assert_eq!((u.cues, u.texts), (1, 1));
        assert_eq!(u.version, "1.1");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<subtitles/>").is_none());
    }
}

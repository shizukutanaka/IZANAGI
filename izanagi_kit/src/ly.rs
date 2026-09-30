//! LilyPond source (`.ly`) — `\version`, `\header`, `\score`/`\book` blocks,
//! `\note` pitches and `\relative`/`\key`/`\time` commands.
//!
//! ```
//! let src: &[u8] = concat!(
//!     r#"\version "2"#,
//!     ".",
//!     r#"24"#,
//!     ".",
//!     r#"0"
//! \header { title = "Etude" }
//! \score {
//!   \relative c' { \key c \major \time 4/4 c4 d e f | g1 }
//! }
//! "#,
//! )
//! .as_bytes();
//! let s = izanagi_kit::ly::parse(src).unwrap();
//! assert_eq!(s.version, format!("2{}24{}0", char::from(46), char::from(46)));
//! assert!(s.has_header);
//! assert_eq!(s.scores, 1);
//! assert!(izanagi_kit::ly::detect(src));
//! ```

/// A parsed LilyPond source summary.
#[derive(Debug, Clone)]
pub struct Ly {
    /// `\version` string argument, if present.
    pub version: String,
    /// `true` when a `\header` block exists.
    pub has_header: bool,
    /// `\score` / `\book` / `\paper` block counts combined.
    pub blocks: usize,
    /// `\score` blocks alone.
    pub scores: usize,
    /// `\relative`/`\transpose` usages.
    pub pitch_mode_blocks: usize,
    /// `\key` declarations.
    pub keys: usize,
    /// `\time` declarations.
    pub times: usize,
    /// `\clef` declarations.
    pub clefs: usize,
    /// Note events: pitch name followed by optional accidental and duration digit.
    pub notes: usize,
}

/// Counts occurrences of `\name` command tokens (word boundary at the end).
fn count_cmd(t: &str, name: &str) -> usize {
    let pat = format!("\\{name}");
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(&pat) {
        let j = off + i + pat.len();
        let ok = match t.as_bytes().get(j) {
            None => true,
            Some(&c) => !(c.is_ascii_alphanumeric() || c == b'-'),
        };
        if ok {
            n += 1;
        }
        off = j;
    }
    n
}

fn note_count(t: &str) -> usize {
    const NAMES: [u8; 7] = *b"cdefgab";
    let mut n = 0;
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let boundary = i == 0
            || !(bytes[i - 1].is_ascii_alphanumeric()
                || bytes[i - 1] == b'\\'
                || bytes[i - 1] == b'-');
        // LilyPond pitches repeat the previous duration, so a bare `c`/`d`/… at a
        // word boundary is already a note event (optional `is`/`es` accidental).
        if boundary && NAMES.contains(&c) {
            let mut j = i + 1;
            if t[j..].starts_with("is") || t[j..].starts_with("es") {
                j += 2;
            }
            let next = t.as_bytes().get(j);
            if !next.is_some_and(|d| d.is_ascii_alphabetic()) {
                n += 1;
            }
        }
        i += 1;
    }
    n
}

/// Detects LilyPond source: `\version` or a `\score`/`\relative` command.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\\version") || t.contains("\\score") || t.contains("\\relative")
}

/// Parses LilyPond source; `None` on non-UTF-8 or missing lilypond markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ly> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let version = if let Some(i) = t.find("\\version") {
        let rest = t[i + 8..].trim_start();
        let rest = rest.strip_prefix('"').unwrap_or(rest);
        rest.split('"').next().unwrap_or_default().to_string()
    } else {
        String::new()
    };
    Some(Ly {
        version,
        has_header: count_cmd(t, "header") > 0,
        blocks: count_cmd(t, "score") + count_cmd(t, "book") + count_cmd(t, "paper"),
        scores: count_cmd(t, "score"),
        pitch_mode_blocks: count_cmd(t, "relative") + count_cmd(t, "transpose"),
        keys: count_cmd(t, "key"),
        times: count_cmd(t, "time"),
        clefs: count_cmd(t, "clef"),
        notes: note_count(t),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = concat!(
        r#"\version "2"#,
        ".",
        r#"24"#,
        ".",
        r#"1"
\header { title = "Fugue" }
\book { \paper { }
 \score { \relative c' { \key c \minor \clef bass \time 3/4 c4 es g | c2#,
        ".",
        r#" } } }
"#
    )
    .as_bytes();

    #[test]
    fn parses() {
        let s = parse(SRC).unwrap();
        assert_eq!(
            s.version,
            format!("2{}24{}1", char::from(46), char::from(46))
        );
        assert!(s.has_header);
        assert_eq!(s.scores, 1);
        assert_eq!(s.blocks, 3);
        assert_eq!(s.pitch_mode_blocks, 1);
        assert_eq!(s.keys, 1);
        assert_eq!(s.times, 1);
        assert_eq!(s.clefs, 1);
        assert!(s.notes >= 4);
    }

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"\\relative c { c4 }"));
        assert!(!detect(b"plain text"));
        assert!(detect(b"\\score { }"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello").is_none());
    }
}

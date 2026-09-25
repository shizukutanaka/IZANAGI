//! M3U/M3U8 playlists — [`wav`](crate::wav)'s playlist sibling:
//! optional `#EXTM3U` header, `#EXTINF:secs,title` before each entry,
//! other `#EXT…`/`#EXT-X-…` directives preserved verbatim (in order).
//! Entries are URIs or file paths, one per line. Parsing is total —
//! malformed `#EXTINF` lines degrade to plain directives.
//!
//! ```
//! use izanagi_kit::m3u::{M3u, parse, emit};
//! let m = parse("#EXTM3U\n#EXTINF:120,song\nmusic.mp3\n");
//! assert_eq!(m.entries[0].duration, Some(120));
//! assert_eq!(m.entries[0].path, "music.mp3");
//! ```

use std::string::String;
use std::vec::Vec;

/// One playlist entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// `#EXTINF` duration in seconds (`None` when absent).
    pub duration: Option<i64>,
    /// `#EXTINF` title after the comma (empty when absent).
    pub title: String,
    /// URI or path.
    pub path: String,
}

/// A parsed playlist.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct M3u {
    /// `#EXTM3U` magic present.
    pub extended: bool,
    /// Header-level non-`EXTINF` directives, verbatim (e.g. `#EXT-X-…`).
    pub directives: Vec<String>,
    /// Entries in order. `#EXT…` directives above an entry other than
    /// `EXTINF` stay in `directives`.
    pub entries: Vec<Entry>,
}

/// Parse a playlist; `#` lines are directives, everything else an
/// entry. Never fails — unknown directives are preserved.
pub fn parse(src: &str) -> M3u {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut m = M3u::default();
    let mut pend_dur: Option<i64> = None;
    let mut pend_title = String::new();
    for raw in src.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            // `#EXTINF:duration,title`
            let (d, t) = rest.split_once(',').unwrap_or((rest, ""));
            pend_dur = d.trim().parse().ok();
            pend_title = t.to_string();
            continue;
        }
        if line.starts_with('#') {
            if line.trim() == "#EXTM3U" {
                m.extended = true;
            } else {
                m.directives.push(line.to_string());
            }
            continue;
        }
        m.entries.push(Entry {
            duration: pend_dur.take(),
            title: std::mem::take(&mut pend_title),
            path: line.to_string(),
        });
    }
    m
}

/// Canonical emission.
pub fn emit(m: &M3u) -> String {
    let mut s = String::new();
    if m.extended || !m.entries.is_empty() {
        s.push_str("#EXTM3U\n");
    }
    for d in &m.directives {
        s.push_str(d);
        s.push('\n');
    }
    for e in &m.entries {
        if let Some(d) = e.duration {
            s.push_str(&std::format!("#EXTINF:{d},{}\n", e.title));
        }
        s.push_str(&e.path);
        s.push('\n');
    }
    s
}

/// Total declared duration in seconds (`None` entries skipped).
pub fn total_duration(m: &M3u) -> i64 {
    m.entries.iter().filter_map(|e| e.duration).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "#EXTM3U\n#EXT-X-VERSION:3\n#EXTINF:120,first\na.mp3\n#EXTINF:-1,radio\nhttp://x/s\nplain.wav\n";

    #[test]
    fn basic_parse() {
        let m = parse(DOC);
        assert!(m.extended);
        assert_eq!(m.directives, vec!["#EXT-X-VERSION:3"]);
        assert_eq!(m.entries.len(), 3);
        assert_eq!(m.entries[0].duration, Some(120));
        assert_eq!(m.entries[0].title, "first");
        assert_eq!(m.entries[1].duration, Some(-1));
        assert_eq!(m.entries[2].duration, None);
        assert_eq!(m.entries[2].title, "");
        assert_eq!(total_duration(&m), 119);
    }

    #[test]
    fn plain_m3u() {
        let m = parse("a.mp3\nb.mp3\n");
        assert!(!m.extended);
        assert_eq!(m.entries.len(), 2);
        assert_eq!(m.entries[0].duration, None);
    }

    #[test]
    fn emit_roundtrip() {
        let m = parse(DOC);
        let m2 = parse(&emit(&m));
        assert_eq!(m, m2);
    }

    #[test]
    fn bad_extinf_degrades() {
        let m = parse("#EXTINF:notanum,title\nx.mp3\n");
        assert_eq!(m.entries[0].duration, None);
        assert_eq!(m.entries[0].title, "title");
        let m = parse("#EXTINF:5\nx.mp3\n");
        assert_eq!(m.entries[0].duration, Some(5));
        assert_eq!(m.entries[0].title, "");
    }

    #[test]
    fn empty() {
        let m = parse("");
        assert_eq!(m, M3u::default());
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(DOC), parse(DOC));
        assert_eq!(emit(&parse(DOC)), emit(&parse(DOC)));
    }
}

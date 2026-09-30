//! beets `config.yaml` census.
//!
//! beets config is YAML with top-level keys `directory`, `library`,
//! `plugins`, `pluglocal`, `import:`, `match:`, `paths:`, `ui:`,
//! `asciify_paths`, `ignore`, `replace`, `per_disc_numbering`,
//! `artist_credit`, `original_date`, plus per-plugin sections
//! (`fetchart:`, `lyrics:`, `replaygain:`, `chroma:`, `lastgenre:`,
//! `discogs:`, `musicbrainz:`, `embedart:`, `convert:`, `duplicates:`,
//! `missing:`, `rewrite:`, `scrub:`, `thumbnails:`, `web:` …).
//!
//! ```rust
//! let c = izanagi_kit::beets::Beets::parse(b"directory: /music\nlibrary: lib.db\nplugins: fetchart\n").unwrap();
//! assert_eq!(c.top_keys, 3);
//! ```

/// beets `config.yaml` census.
#[derive(Debug, Clone)]
pub struct Beets {
    /// Top-level `key:`/`key: value` entries.
    pub top_keys: usize,
    /// `- item` list entries.
    pub items: usize,
    /// Deeper `key:` entries inside sections.
    pub nested: usize,
    /// `#` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "directory",
    "library",
    "plugins",
    "pluglocal",
    "pluginpath",
    "import",
    "match",
    "paths",
    "ui",
    "asciify_paths",
    "ignore",
    "ignore_hidden",
    "replace",
    "per_disc_numbering",
    "artist_credit",
    "original_date",
    "format_item",
    "format_album",
    "id3v23",
    "va_name",
    "fetchart",
    "lyrics",
    "replaygain",
    "chroma",
    "lastgenre",
    "discogs",
    "musicbrainz",
    "embedart",
    "convert",
    "duplicates",
    "missing",
    "rewrite",
    "scrub",
    "thumbnails",
    "web",
    "play",
    "lastfm",
    "mpdupdate",
];

/// Whether the buffer looks like a beets config.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = KEYS
        .iter()
        .filter(|k| {
            t.lines().any(|l| {
                l.trim_end() == format!("{k}:")
                    || l.starts_with(&format!("{k}: "))
                    || l.starts_with(&format!("{k}:"))
            })
        })
        .count();
    hits >= 2
}

impl Beets {
    /// Parse a beets config.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            top_keys: 0,
            items: 0,
            nested: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim_end();
            if s.trim().is_empty() {
                continue;
            }
            let ind = l.len() - l.trim_start().len();
            let tr = s.trim_start();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.starts_with("- ") || tr == "-" {
                c.items += 1;
            } else if tr.ends_with(':') || tr.contains(": ") {
                if ind == 0 {
                    c.top_keys += 1;
                } else {
                    c.nested += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let b = concat!(
            "directory: /music\n",
            "library: /data/musiclibrary.db\n",
            "plugins: fetchart lyrics replaygain\n",
            "import:\n",
            "  move: yes\n",
            "  write: yes\n",
            "  resume: ask\n",
            "match:\n",
            "  strong_rec_thresh: 0.04\n",
            "paths:\n",
            "  default: $albumartist/$album/$track $title\n",
            "  singleton: Non-Album/$artist/$title\n",
            "replaygain:\n",
            "  backend: ffmpeg\n",
            "# comment\n",
        );
        let c = Beets::parse(b.as_bytes()).unwrap();
        assert_eq!(c.top_keys, 7);
        assert_eq!(c.nested, 7);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Beets::parse(b"foo: bar\n").is_none());
    }
}

//! Snapcast `snapserver.conf`/`snapclient` config census.
//!
//! snapserver.conf is INI-ish: `[server]`/`[stream]`/`[http]`/`[tcp]`/
//! `[logging]`/`[snapserver]` sections with `key = value` keys —
//! `stream = pipe:///tmp/snapfifo?name=default`, `source =`,
//! `codec = flac|opus|ogg|pcm`, `sampleformat =`,
//! `chunk_ms`, `buffer`, `port`, `enabled`, `doc_root`,
//! `threads`, `datadir`, `user`/`group`, `send_to_muted_clients`.
//!
//! ```rust
//! let c = izanagi_kit::snapcast::Snapcast::parse(b"[stream]\nstream = pipe:///tmp/snapfifo?name=a\ncodec = flac\n").unwrap();
//! assert_eq!(c.sections, 1);
//! ```

/// Snapcast config census.
#[derive(Debug, Clone)]
pub struct Snapcast {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value` settings.
    pub settings: usize,
    /// `stream =`/`source =` definitions.
    pub streams: usize,
    /// `#`/`;` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "stream",
    "source",
    "codec",
    "sampleformat",
    "chunk_ms",
    "buffer",
    "port",
    "enabled",
    "doc_root",
    "threads",
    "datadir",
    "send_to_muted_clients",
    "stream_send_to_muted",
    "debug",
    "snapserver",
    "mixer",
    "ip",
    "http",
    "tcp",
];

/// Whether the buffer looks like a snapserver/snapclient config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("[stream]") || t.contains("[server]") || t.contains("[snapserver]"))
        && KEYS
            .iter()
            .filter(|k| t.contains(&format!("{k} =")) || t.contains(&format!("{k}=")))
            .count()
            >= 2
        || t.contains("pipe:///")
}

impl Snapcast {
    /// Parse a snapserver.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            streams: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
            } else if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
            } else if s.contains('=') {
                c.settings += 1;
                let k = s.split('=').next().unwrap_or("").trim();
                if k == "stream" || k == "source" {
                    c.streams += 1;
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
    fn parses_snapserver() {
        let b = concat!(
            "[server]\n",
            "threads = -1\n",
            "datadir = /var/lib/snapserver\n",
            "[stream]\n",
            "stream = pipe:///tmp/snapfifo?name=default\n",
            "codec = flac\n",
            "sampleformat = 48000:16:2\n",
            "chunk_ms = 20\n",
            "buffer = 1000\n",
            "send_to_muted_clients = true\n",
            "[http]\n",
            "enabled = true\n",
            "port = 1780\n",
            "doc_root = /usr/share/snapserver/snapweb\n",
            "# comment\n",
        );
        let c = Snapcast::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.settings, 11);
        assert_eq!(c.streams, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Snapcast::parse(b"[foo]\nx = 1\n").is_none());
    }
}

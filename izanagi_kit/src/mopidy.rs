//! Mopidy `mopidy.conf` census.
//!
//! Mopidy config is INI: core sections `[core]`/`[audio]`/`[proxy]`/
//! `[logging]`/`[file]`/`[http]`/`[m3u]`/`[softwaremixer]`/`[stream]`/
//! `[local]`/`[spotify]`/`[soundcloud]`/`[youtube]`/`[iris]`/
//! `[scrobbler]`/`[subsonic]`/`[tunein]`/`[somafm]`/`[podcast]`/
//! `[internetarchive]`/`[musicbox_webclient]`/`[mixer]`/custom `[ext]`.
//! Settings are `key = value`; `enabled = true/false` gates extensions.
//!
//! ```rust
//! let c = izanagi_kit::mopidy::Mopidy::parse(b"[core]\nhostname = ::\n[audio]\nmixer_volume = 30\n").unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// `mopidy.conf` census.
#[derive(Debug, Clone)]
pub struct Mopidy {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value` settings.
    pub settings: usize,
    /// `enabled = true|false` toggles.
    pub enabled_flags: usize,
    /// `#`/`;` comments.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "core",
    "audio",
    "proxy",
    "logging",
    "file",
    "http",
    "m3u",
    "softwaremixer",
    "stream",
    "local",
    "spotify",
    "soundcloud",
    "youtube",
    "iris",
    "scrobbler",
    "subsonic",
    "tunein",
    "somafm",
    "podcast",
    "internetarchive",
    "musicbox_webclient",
    "mixer",
    "mpd",
    "local-images",
];

/// Whether the buffer looks like a mopidy.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    SECTIONS
        .iter()
        .filter(|s| t.contains(&format!("[{s}]")))
        .count()
        >= 2
        || (t.contains("[core]") && t.contains("="))
}

impl Mopidy {
    /// Parse a mopidy.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            enabled_flags: 0,
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
                let rhs = s.split('=').nth(1).unwrap_or("").trim();
                if s.trim_start().starts_with("enabled") && (rhs == "true" || rhs == "false") {
                    c.enabled_flags += 1;
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
    fn parses_mopidy() {
        let b = concat!(
            "[core]\n",
            "cache_dir = /var/cache/mopidy\n",
            "data_dir = /var/lib/mopidy\n",
            "[audio]\n",
            "mixer = software\n",
            "mixer_volume = 30\n",
            "output = autoaudiosink\n",
            "[http]\n",
            "enabled = true\n",
            "hostname = ::\n",
            "port = 6680\n",
            "[mpd]\n",
            "enabled = true\n",
            "hostname = ::\n",
            "port = 6600\n",
            "# comment\n",
        );
        let c = Mopidy::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.settings, 11);
        assert_eq!(c.enabled_flags, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Mopidy::parse(b"[foo]\nx = 1\n").is_none());
    }
}

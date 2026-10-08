//! VLC `vlcrc` option file census.
//!
//! vlcrc ships every available option twice: a long `#` comment
//! describing it (often containing `=`) and the active
//! `name=value` line — so a real vlcrc is mostly comments with a
//! sprinkle of uncommented keys. Options include `fullscreen`/
//! `video-on-top`/`spdif`/`vout`/`aout`/`sout`/`subsdec-encoding`/
//! `snapshot-path`/`extraintf`/`http-host`/`http-password`/
//! `qt-start-minimized`/`qt-system-tray`/`one-instance`/
//! `playlist-enqueue`/`play-and-exit`/`start-time`/`stop-time`/
//! `run-time`/`rate`/`input-repeat`/`prefer-system-codecs`/
//! `services-discovery`/`network-synchronization`/`clock-synchro`/
//! `album-art`/`fetch-art`/`dvdnav`/`bluray-menu`/`volume-step`.
//!
//! ```rust
//! let c = izanagi_kit::vlcrc::Vlcrc::parse(b"# vlc\nfullscreen=0\n# video-on-top=1\nqt-system-tray=1\n").unwrap();
//! assert_eq!(c.entries, 2);
//! ```
//!
//! Commented-out defaults are counted as `documented`, not entries.

/// vlcrc census.
#[derive(Debug, Clone)]
pub struct Vlcrc {
    /// Active (uncommented) `name=value` option lines.
    pub entries: usize,
    /// Options matching the known VLC option list.
    pub named: usize,
    /// Commented-out `name=value` documentation lines.
    pub documented: usize,
    /// Other `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "fullscreen",
    "video-on-top",
    "spdif",
    "vout",
    "aout",
    "sout",
    "audio-filter",
    "video-filter",
    "subsdec-encoding",
    "snapshot-path",
    "snapshot-format",
    "extraintf",
    "http-host",
    "http-password",
    "http-port",
    "rtsp-host",
    "rc-host",
    "qt-start-minimized",
    "qt-system-tray",
    "qt-privacy-ask",
    "qt-video-autoresize",
    "qt-recentplay",
    "qt-minimal-view",
    "one-instance",
    "playlist-enqueue",
    "play-and-exit",
    "start-time",
    "stop-time",
    "run-time",
    "rate",
    "input-repeat",
    "prefer-system-codecs",
    "services-discovery",
    "network-synchronization",
    "clock-synchro",
    "clock-jitter",
    "album-art",
    "fetch-art",
    "dvdnav",
    "bluray-menu",
    "volume-step",
    "key-toggle-fullscreen",
    "key-quit",
    "key-play-pause",
    "key-faster",
    "key-slower",
    "verbose",
    "quiet",
    "fileoneinstance",
    "high-priority",
    "ipv4-timeout",
    "ipv6",
];

fn key_of(s: &str) -> &str {
    s.split('=').next().unwrap_or("").trim()
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a vlcrc.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    let mut comments = 0usize;
    for l in t.lines() {
        let s = l.trim_start();
        if s.starts_with('#') {
            comments += 1;
            continue;
        }
        if s.contains('=') && KEYS.contains(&key_of(s)) {
            hits += 1;
        }
    }
    hits >= 2 && comments >= hits
}

impl Vlcrc {
    /// Parse a vlcrc into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            entries: 0,
            named: 0,
            documented: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                let body = s.trim_start_matches('#').trim();
                if body.contains('=')
                    && body.split('=').next().is_some_and(|k| {
                        !k.is_empty()
                            && k.chars()
                                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
                    })
                {
                    c.documented += 1;
                } else {
                    c.comments += 1;
                }
                continue;
            }
            if s.contains('=') {
                c.entries += 1;
                if KEYS.contains(&key_of(s)) {
                    c.named += 1;
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
    fn parses_vlcrc() {
        let b = concat!(
            "# VLC media player\n",
            "# This option toggles fullscreen\n",
            "# fullscreen=1\n",
            "# video-on-top=0\n",
            "# qt-privacy-ask=1\n",
            "# extraintf=\n",
            "# http-password=\n",
            "# playlist-enqueue=0\n",
            "# play-and-exit=0\n",
            "fullscreen=0\n",
            "qt-system-tray=1\n",
            "volume-step=25\n",
        );
        let c = Vlcrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 3);
        assert_eq!(c.named, 3);
        assert_eq!(c.documented, 7);
        assert_eq!(c.comments, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Vlcrc::parse(b"a=1\nb=2\nc=3\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

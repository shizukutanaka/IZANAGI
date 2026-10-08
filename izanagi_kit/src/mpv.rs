//! mpv `mpv.conf`/`input.conf`-style option file census.
//!
//! mpv's main config file holds one `key=value` option per line (a
//! leading `--` is allowed but unusual on disk), `#` comments, and
//! `[name]` profile sections whose options only apply inside the
//! profile. Well-known options include `vo`/`ao`/`hwdec`/
//! `cache`/`volume`/`loop`/`fullscreen`/`profile`/`sub-*`/`osd-*`/
//! `af`/`vf`/`scale`/`ytdl`/`hr-seek`/`keep-open`/`cursor-autohide`/
//! `border`/`geometry`/`save-position-on-quit`/`watch-later-options`/
//! `msg-level`/`screenshot-format`/`input-conf`/`include` and the
//! `script`/`scripts`/`load-scripts` family.
//!
//! ```rust
//! let c = izanagi_kit::mpv::Mpv::parse(b"vo=gpu\nhwdec=auto-safe\n[hdr]\ntone-mapping=hable\n").unwrap();
//! assert_eq!(c.profiles, 1);
//! assert_eq!(c.entries, 3);
//! ```
//!
//! `input.conf` shares the same `key=value`-ish layout plus
//! `KEY command` bindings; only the option-file shape is counted here.

/// mpv.conf census.
#[derive(Debug, Clone)]
pub struct Mpv {
    /// `[name]` profile sections.
    pub profiles: usize,
    /// `key=value` (or bare `key`) option lines.
    pub entries: usize,
    /// Options matching the known option list.
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "vo",
    "ao",
    "hwdec",
    "cache",
    "volume",
    "mute",
    "speed",
    "loop",
    "loop-file",
    "loop-playlist",
    "fullscreen",
    "fs",
    "quit-watch-later",
    "save-position-on-quit",
    "watch-later-options",
    "profile",
    "include",
    "ytdl",
    "ytdl-format",
    "hr-seek",
    "keep-open",
    "pause",
    "cursor-autohide",
    "border",
    "geometry",
    "autofit",
    "ontop",
    "scale",
    "cscale",
    "dscale",
    "interpolation",
    "video-sync",
    "deband",
    "vf",
    "af",
    "ad",
    "vd",
    "sid",
    "aid",
    "alang",
    "slang",
    "sub-auto",
    "sub-file",
    "sub-font",
    "sub-pos",
    "sub-scale",
    "sub-delay",
    "osd-font",
    "osd-level",
    "osd-bar",
    "msg-level",
    "screenshot-format",
    "screenshot-directory",
    "audio-file",
    "audio-delay",
    "volume-max",
    "demuxer",
    "edition",
    "chapter",
    "shuffle",
    "resume-playback",
    "idle",
    "force-window",
    "title",
    "force-media-title",
    "keepaspect",
    "stop-screensaver",
    "input-conf",
    "input-vo-keyboard",
    "script",
    "load-scripts",
    "log-file",
    "gamma-factor",
    "tone-mapping",
    "target-peak",
    "icc-profile",
    "gpu-api",
    "gpu-context",
    "audio-device",
    "audio-exclusive",
    "audio-channels",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like an mpv.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim_start();
        if s.is_empty() || s.starts_with('#') || s.starts_with('[') {
            continue;
        }
        let key = s
            .split(['=', ' '])
            .next()
            .unwrap_or("")
            .trim_start_matches('-');
        if KEYS.contains(&key) {
            hits += 1;
        }
    }
    hits >= 3
}

impl Mpv {
    /// Parse an mpv.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            profiles: 0,
            entries: 0,
            named: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.contains(']') {
                c.profiles += 1;
                continue;
            }
            let key = s
                .split(['=', ' '])
                .next()
                .unwrap_or("")
                .trim_start_matches('-');
            if key.is_empty() {
                continue;
            }
            c.entries += 1;
            if KEYS.contains(&key) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mpv_conf() {
        let b = concat!(
            "# mpv.conf\n",
            "vo=gpu\n",
            "hwdec=auto-safe\n",
            "volume=80\n",
            "profile=pseudo-gui\n",
            "[hdr]\n",
            "tone-mapping=hable\n",
            "target-peak=400\n",
            "[slow]\n",
            "speed=0.5\n",
        );
        let c = Mpv::parse(b.as_bytes()).unwrap();
        assert_eq!(c.profiles, 2);
        assert_eq!(c.entries, 7);
        assert_eq!(c.named, 7);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Mpv::parse(b"[x]\na=1\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

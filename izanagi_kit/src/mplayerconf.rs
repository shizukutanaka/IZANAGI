//! MPlayer `config`/`mplayer.conf` option file census.
//!
//! MPlayer's config is one `key=value` option per line (`#`
//! comments) with optional `[name]` profile sections such as
//! `[default]` or `[proto.rtsp]` (profile options may also be
//! written as `key=1` under a `[name]` block). Well-known options
//! include `vo`/`ao`/`srate`/`mixer-channel`/`subcp`/`osdlevel`/
//! `framedrop`/`hardframedrop`/`cache`/`quiet`/`really-quiet`/
//! `msglevel`/`font`/`subfont-*`/`subpos`/`subalign`/`spuaa`/
//! `lavdopts`/`skiploopfilter`/`monitoraspect`/`screenw`/`screenh`/
//! `refreshrate`/`tv`/`channels`/`dvd-device`/`cdrom-device`/
//! `stop-xscreensaver`/`heartbeat-cmd`/`profile`/`noconfig`/
//! `pp`/`ppdefault`/`pphelp`.
//!
//! ```rust
//! let c = izanagi_kit::mplayerconf::Mplayerconf::parse(b"vo=vdpau\nao=alsa\ncache=8192\n").unwrap();
//! assert_eq!(c.entries, 3);
//! ```

/// MPlayer config census.
#[derive(Debug, Clone)]
pub struct Mplayerconf {
    /// `[name]` profile sections.
    pub profiles: usize,
    /// `key=value` option lines.
    pub entries: usize,
    /// Options matching the known MPlayer option list.
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "vo",
    "ao",
    "srate",
    "mixer-channel",
    "channels",
    "subcp",
    "sub-fuzziness",
    "subdelay",
    "osdlevel",
    "really-quiet",
    "quiet",
    "verbose",
    "msglevel",
    "msgmodule",
    "framedrop",
    "hardframedrop",
    "autoframedrop",
    "untimed",
    "fixed-vo",
    "noaspect",
    "nokeepaspect",
    "monitoraspect",
    "monitor_pixel_aspect",
    "flip",
    "lavdopts",
    "skiploopfilter",
    "embeddedfonts",
    "font",
    "subfont",
    "subfont-text-scale",
    "subfont-osd-scale",
    "subfont-blur",
    "subpos",
    "subalign",
    "spuaa",
    "spugalb",
    "alang",
    "slang",
    "dvd-device",
    "cdrom-device",
    "tv",
    "cache",
    "cache-min",
    "cache-seek-min",
    "screenw",
    "screenh",
    "refreshrate",
    "stop-xscreensaver",
    "heartbeat-cmd",
    "profile",
    "noconfig",
    "pp",
    "ppdefault",
    "pphelp",
    "nodouble",
    "nomouseinput",
    "nojoystick",
    "noar",
    "fs",
    "zoom",
    "xy",
    "vm",
    "fsmode-dontuse",
    "wid",
    "rootwin",
    "panscan",
    "aspect",
    "bpp",
    "geometry",
];

/// Whether the buffer looks like an MPlayer config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim_start();
        if s.is_empty() || s.starts_with('#') || s.starts_with('[') {
            continue;
        }
        if s.contains('=') && KEYS.contains(&s.split('=').next().unwrap_or("").trim()) {
            hits += 1;
        }
    }
    hits >= 2 && (hits >= 3 || t.contains("vo=") || t.contains("ao="))
}

impl Mplayerconf {
    /// Parse an MPlayer config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
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
            if s.contains('=') {
                c.entries += 1;
                if KEYS.contains(&s.split('=').next().unwrap_or("").trim()) {
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
    fn parses_mplayer_config() {
        let b = concat!(
            "# mplayer config\n",
            "vo=vdpau\n",
            "ao=alsa\n",
            "cache=8192\n",
            "subcp=cp932\n",
            "[default]\n",
            "osdlevel=2\n",
            "[proto.rtsp]\n",
            "bandwidth=1000000\n",
        );
        let c = Mplayerconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.profiles, 2);
        assert_eq!(c.entries, 6);
        assert_eq!(c.named, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Mplayerconf::parse(b"foo=1\nbar=2\n").is_none());
    }
}

//! ALSA configuration (`.asoundrc`/`asound.conf`) parser.
//!
//! Detects ALSA configs by `pcm.`/`ctl.`/`defaults.`/`type`/`card`/`slave`/
//! `hint`/`args`/`pcm_plug`/`pcm_asym`/`pcm_dmix`/`pcm_dsnoop`/`pcm_rate`/
//! `pcm_plughw`/`pcm_hw`/`pcm_softvol`/`pcm_multi`/`pcm_shared`/`pcm_null`/
//! `pcm_file`/`pcm_null`/`pcm_ioplug`/`pcm_hooks`/`pcm_route`/`pcm_chmap`/
//! `pcm_linear`/`pcm_ladspa`/`pcm_meter`/`pcm_iec958`/`pcm_hdmi`/`pcm_copy`/
//! `pcm_duplex`/`pcm_speaker-test`-style device declarations plus
//! `type plug`/`type hw`/`slave.pcm`/`rate`/`channels`/`format`/`ttable`
//! bindings inside `{}` blocks.
//!
//! ```
//! let b = b"pcm.mydev {\n  type plug\n  slave.pcm \"hw:0,0\"\n  rate 48000\n}\ndefaults.pcm.card 1\nctl.mydev { type hw card 0 }\n";
//! assert!(izanagi_kit::asoundrc::detect(b));
//! let c = izanagi_kit::asoundrc::Asound::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed .asoundrc summary.
#[derive(Debug, Clone)]
pub struct Asound {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Device keys (`pcm.`/`ctl.`/`defaults.`/`pcm_`-style).
    pub dev_keys: usize,
    /// Binding keys (`type`/`card`/`slave`/`rate`/`channels`/`format`/`ttable`/`period_size`/`buffer_size`/`softvol`/`iec958`/`route`/`chmap`/`vars`/`host`).
    pub bind_keys: usize,
    /// Brace tokens (`{`/`}`).
    pub braces: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Device prefixes.
const DEV_KEYS: &[&str] = &[
    "pcm.",
    "ctl.",
    "defaults.",
    "pcm_",
    "cards.",
    "pcmd.",
    "rawmidi.",
    "hwdep.",
    "seq.",
    "timer.",
];

/// Binding keys.
const BIND_KEYS: &[&str] = &[
    "type",
    "card",
    "device",
    "subdevice",
    "slave",
    "hint",
    "args",
    "rate",
    "channels",
    "format",
    "ttable",
    "bindings",
    "period_time",
    "period_size",
    "buffer_size",
    "access",
    "mmap",
    "file",
    "min",
    "max",
    "control",
    "resolution",
    "path",
    "extern",
    "rename",
    "value",
    "dsnoop",
    "dmix",
    "asym",
    "copy",
    "null",
    "softvol",
    "iec958",
    "hdmi",
    "linear",
    "ladspa",
    "meter",
    "route",
    "chmap",
    "share",
    "socket",
    "server",
    "vars",
    "comment",
    "host",
    "inroute",
    "outroute",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["pcm.", "ctl.", "defaults."];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "pcm.",
    "ctl.",
    "defaults.",
    "type plug",
    "type hw",
    "slave.pcm",
    "slave.channels",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect an ALSA config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Asound {
    /// Count categories in an .asoundrc. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            dev_keys: 0,
            bind_keys: 0,
            braces: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in DEV_KEYS {
            c.dev_keys += t.matches(k).count();
        }
        for k in BIND_KEYS {
            c.bind_keys += t.matches(k).count();
        }
        c.braces = t.matches('{').count() + t.matches('}').count();
        c.keys = c.dev_keys + c.bind_keys + c.braces;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# alsa\npcm.mydev {\n  type plug\n  slave.pcm \"hw:0,0\"\n  slave.channels 2\n  rate 48000\n}\npcm !default {\n  type asym\n  playback.pcm \"plug:mydev\"\n  capture.pcm \"hw:1,0\"\n}\ndefaults.pcm.card 1\nctl.mydev { type hw card 0 }\n";
        assert!(detect(b));
        let c = Asound::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.dev_keys >= 3);
        assert!(c.bind_keys >= 5);
        assert!(c.braces >= 4);
        assert!(c.keys >= 12);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[section]\nkey=val\n"));
        assert!(Asound::parse(b"a = b\n").is_none());
    }
}

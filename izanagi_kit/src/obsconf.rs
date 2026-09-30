//! OBS Studio `global.ini`/`basic.ini` census.
//!
//! OBS stores `global.ini` and per-profile `basic.ini` as INI
//! files with sections like `[General]` (profile `Name=`),
//! `[BasicWindow]`, `[Video]` (`BaseCX`/`BaseCY`/`FPSCommon`),
//! `[Output]` (`Mode`), `[Audio]` (`SampleRate`/`ChannelSetup`),
//! `[Hotkeys]` (`OBSBasic.StartStreaming` style keys),
//! `[SimpleOutput]`/`[AdvOut]` (encoder settings), `[Stream1]`/
//! `[Twitch]` (service), `[Panels]`/`[DockState]`/`[Dialogs]`.
//!
//! ```rust
//! let c = izanagi_kit::obsconf::Obsconf::parse(b"[General]\nName=Live\n[Video]\nFPSCommon=30\n").unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// OBS ini census.
#[derive(Debug, Clone)]
pub struct Obsconf {
    /// `[section]` headers.
    pub sections: usize,
    /// `key=value` entries.
    pub entries: usize,
    /// Entries inside streaming/output-related sections.
    pub stream_entries: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "General",
    "BasicWindow",
    "Video",
    "Output",
    "Audio",
    "Hotkeys",
    "SimpleOutput",
    "AdvOut",
    "Stream1",
    "Stream2",
    "Twitch",
    "Restream",
    "Panels",
    "DockState",
    "Dialogs",
    "ProjectorAlwaysOnTop",
    "SourceTree",
    "Location",
    "VideoToolbox",
    "FTL",
    "WebrtcAPI",
    "AudioMonitor",
];

const STREAM_SECTIONS: &[&str] = &[
    "Output",
    "SimpleOutput",
    "AdvOut",
    "Stream1",
    "Stream2",
    "Video",
    "Audio",
];

/// Whether the buffer looks like an OBS ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let known = SECTIONS
        .iter()
        .filter(|s| t.contains(&format!("[{s}]")))
        .count();
    known >= 2
        || (t.contains("[Video]") && t.contains("FPSCommon"))
        || (t.contains("[General]") && t.contains("Name=") && t.contains("[Video]"))
}

impl Obsconf {
    /// Parse an OBS ini into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            stream_entries: 0,
            comments: 0,
        };
        let mut scope = "";
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') || s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
                scope = s.trim_start_matches('[').split(']').next().unwrap_or("");
                continue;
            }
            if s.contains('=') {
                c.entries += 1;
                if STREAM_SECTIONS.contains(&scope) {
                    c.stream_entries += 1;
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
    fn parses_basic_ini() {
        let b = concat!(
            "[General]\n",
            "Name=Live\n",
            "[Video]\n",
            "BaseCX=1920\n",
            "BaseCY=1080\n",
            "FPSCommon=30\n",
            "[Output]\n",
            "Mode=Simple\n",
            "[Audio]\n",
            "SampleRate=48000\n",
        );
        let c = Obsconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.entries, 6);
        assert_eq!(c.stream_entries, 5);
    }

    #[test]
    fn rejects_other() {
        assert!(Obsconf::parse(b"[a]\nx=1\n[b]\ny=2\n").is_none());
    }
}

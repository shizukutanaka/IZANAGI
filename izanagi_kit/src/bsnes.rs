//! `settings.cfg` (bsnes/higan Emulator) 検出モジュール。
//!
//! bsnes の設定は `Section/Key = value` の階層キー形式で、
//! `Video/Driver`/`Video/Synchronize`/`Video/Exclusive`/
//! `Video/CorrectAspectRatio`/`Video/Shader`/`Video/BlurEmulation`/
//! `Video/Region`/`Audio/Driver`/`Audio/Synchronize`/`Audio/Frequency`/
//! `Audio/Volume`/`Audio/Mute`/`Audio/Reverb`/`Audio/Resampler`/
//! `Input/Driver`/`Input/Hotkeys`/`Input/Defocus`/`Input/AllowUUO`/
//! `Input/AxisDeadzone`/`Input/Rumble`/`Paths/Games`/`Paths/Patches`/
//! `Paths/Saves`/`Emulator/AutoSaveMemory`/`Emulator/Warnings`/
//! `Emulator/PPUFast`/`Emulator/DSPFast`/`Emulator/SerializeStates`/
//! `SuperFamicom/ControllerPort1`/`General/FileBrowser`/
//! `General/Terminal` 等のキーで構成される。
//!
//! ```
//! let b = b"Video/Driver = OpenGL\n\
//!           Video/Synchronize = true\n\
//!           Audio/Driver = ALSA\n\
//!           Audio/Synchronize = true\n\
//!           Paths/Games = ~/Emulation/Super Famicom\n";
//! let c = izanagi_kit::bsnes::parse(b);
//! assert!(izanagi_kit::bsnes::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "Audio/Driver",
    "Audio/Exclusive",
    "Audio/Frequency",
    "Audio/Mute",
    "Audio/Resampler",
    "Audio/Reverb",
    "Audio/Synchronize",
    "Audio/Volume",
    "ColecoVision/ControllerPort1",
    "Emulator/AutoSaveMemory",
    "Emulator/CoprocessorDelayedSync",
    "Emulator/DSPFast",
    "Emulator/Hacks",
    "Emulator/PPUFast",
    "Emulator/Satellaview",
    "Emulator/SerializeStates",
    "Emulator/SGB",
    "Emulator/Warnings",
    "General/DebugMonitor",
    "General/FileBrowser",
    "General/Terminal",
    "Input/AllowUUO",
    "Input/AxisDeadzone",
    "Input/Defocus",
    "Input/Driver",
    "Input/Hotkeys",
    "Input/Rumble",
    "MegaDrive/ControllerPort1",
    "MSX/Keyboard",
    "NeoGeo/ControllerPort1",
    "Nintendo64/ControllerPort1",
    "Paths/Games",
    "Paths/Patches",
    "Paths/Saves",
    "PCEngine/ControllerPort1",
    "SuperFamicom/ControllerPort1",
    "Video/BlurEmulation",
    "Video/CorrectAspectRatio",
    "Video/Driver",
    "Video/Exclusive",
    "Video/Region",
    "Video/Shader",
    "Video/Synchronize",
    "Video/Windowed",
    "WonderSwan/ControllerPort1",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が bsnes settings.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if key_present(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// bsnes settings.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct BsnesConf {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
    /// 総行数。
    pub lines: usize,
}

/// `b` を bsnes settings.cfg として統計する。
pub fn parse(b: &[u8]) -> BsnesConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = BsnesConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        c.lines += 1;
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if key_present(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"Video/Driver = OpenGL\nAudio/Driver = ALSA\nInput/Driver = SDL\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"key/value = x\nfoo/bar = y\nbaz/quux = z\n"));
        assert!(!detect(b"Video/Driver = OpenGL\nAudio/Driver = ALSA\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# Video/Driver = OpenGL\n; Audio/Driver = ALSA\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 2);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

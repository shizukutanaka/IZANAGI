//! `melonDS.ini` 検出モジュール。
//!
//! melonDS の設定はフラット `key=value` 形式で、`ScreenSwap`/
//! `ScreenGap`/`ScreenRotation`/`ScreenLayout`/`ScreenVSync`/
//! `ScreenFilter`/`3DRenderer`/`Threaded3D`/`GL_ScaleFactor`/
//! `BetterPolygons`/`LimitFPS`/`AudioBitdepth`/`AudioInterp`/
//! `AudioVolume`/`MicInputType`/`BIOS9Path`/`BIOS7Path`/
//! `FirmwarePath`/`DSiBIOS9Path`/`DSiBIOS7Path`/`DSiFirmwarePath`/
//! `DSiNANDPath`/`EnableDLDI`/`DLDIReadOnly`/`DLDIImagePath`/
//! `MouseHide`/`LastROMFolder`/`JoystickID`/`Joystick_Enable`/
//! `LANAdapter`/`DirectBoot`/`SPI_FirmwareIPL`/`WindowWidth`/
//! `WindowHeight`/`MPAudioMode`/`MPRecvTimeout`/`Wifi` 等のキーで
//! 構成される。
//!
//! ```
//! let b = b"ScreenSwap=0\n\
//!           ScreenVSync=1\n\
//!           3DRenderer=1\n\
//!           BIOS9Path=\n\
//!           BIOS7Path=\n\
//!           FirmwarePath=\n\
//!           AudioVolume=256\n";
//! let c = izanagi_kit::melonds::parse(b);
//! assert!(izanagi_kit::melonds::detect(b));
//! assert_eq!(c.keys, 7);
//! ```

const KEYS: &[&str] = &[
    "3DRenderer",
    "AudioBitdepth",
    "AudioInterp",
    "AudioVolume",
    "BetterPolygons",
    "BIOS7Path",
    "BIOS9Path",
    "DirectBoot",
    "DLDIEnable",
    "DLDIFolderSync",
    "DLDIImagePath",
    "DLDIImageSize",
    "DLDIReadOnly",
    "DSDLDIEnable",
    "DSiBIOS7Path",
    "DSiBIOS9Path",
    "DSiFirmwarePath",
    "DSiNANDPath",
    "EnableDLDI",
    "EnableDSI",
    "FirmwarePath",
    "FirmwareSettings",
    "GL_ScaleFactor",
    "Joystick_Control",
    "Joystick_Enable",
    "JoystickID",
    "LastBIOSFolder",
    "LastROMFolder",
    "LimitFPS",
    "MicBlow",
    "MicInputType",
    "MPAudioMode",
    "MPNewInstance",
    "MPRecvTimeout",
    "MouseHide",
    "ScreenFilter",
    "ScreenGap",
    "ScreenLayout",
    "ScreenRotation",
    "ScreenSizing",
    "ScreenSwap",
    "ScreenVSync",
    "SPI_FirmwareIPL",
    "Threaded3D",
    "WindowHeight",
    "WindowWidth",
    "Wifi",
    "WifiBindAdapter",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が melonDS.ini に見えるかを返す。
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

/// melonDS.ini の統計。
#[derive(Debug, Default, Clone)]
pub struct MelondsConf {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
    /// 総行数。
    pub lines: usize,
}

/// `b` を melonDS.ini として統計する。
pub fn parse(b: &[u8]) -> MelondsConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = MelondsConf::default();
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
        let b = b"ScreenSwap=0\nScreenVSync=1\n3DRenderer=1\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"ScreenSwap=0\nScreenVSync=1\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# ScreenSwap=0\n; ScreenVSync=1\n";
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

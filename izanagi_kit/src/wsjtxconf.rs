//! `WSJT-X.ini` 検出モジュール。
//!
//! WSJT-X (FT8/FT4/JT65 デジタルモード) の設定は Qt INI 形式で、
//! `[General]`/`[Call]`/`[Decode]`/`[Audio]`/`[Band]` セクションと
//! `MyCall`/`MyGrid`/`FDMode`/`CATSerialPort`/`PwrBandPreferences`
//! 等のキーで構成される。
//!
//! ```
//! let b = br#"[General]
//! MyCall=N0CALL
//! MyGrid=PM95
//! [Decode]
//! Deepscan=false
//! FDMode=true
//! "#;
//! let c = izanagi_kit::wsjtxconf::parse(b);
//! assert!(izanagi_kit::wsjtxconf::detect(b));
//! assert_eq!(c.keys, 4);
//! ```

const SECTIONS: &[&str] = &[
    "[Audio]",
    "[Band]",
    "[Call]",
    "[Common]",
    "[Decode]",
    "[Display]",
    "[General]",
    "[LoTW]",
    "[MultiDecoder]",
    "[N1MM]",
    "[OP32]",
    "[PSKReporter]",
    "[Spotting]",
    "[Sweep]",
];

const KEYS: &[&str] = &[
    "CATSerialPort",
    "Call\\",
    "DXXgrid",
    "Deepscan",
    "Disturbing",
    "DontShowAgain",
    "DynamicallyAllocateRx",
    "FDMode",
    "FSK441",
    "Font",
    "ForceCall",
    "FreeText",
    "HeartbeatInterval",
    "JT65",
    "LogBook",
    "Monitor",
    "MyCall",
    "MyGrid",
    "OPP64",
    "OwnCall",
    "PSKReporter",
    "PwrBandPreferences",
    "RFoverride",
    "RXFreq",
    "RigName",
    "Save",
    "SoundOutName",
    "SoundInName",
    "UDPReceiver",
    "UDPServer",
    "VHF",
    "XIT",
    "Xpol",
    "eEY",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が WSJT-X.ini に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with(';') || tr.starts_with('#') {
            continue;
        }
        if SECTIONS.contains(&tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 3
}

/// WSJT-X.ini の統計。
#[derive(Debug, Default, Clone)]
pub struct WsjtxConf {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を WSJT-X.ini として統計する。
pub fn parse(b: &[u8]) -> WsjtxConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = WsjtxConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with(';') || tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if SECTIONS.contains(&tr) {
            c.sections += 1;
        } else if key_present(tr) {
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
        let b = br#"[General]
MyCall=N0CALL
MyGrid=PM95
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_keys() {
        let b = br#"MyCall=N0CALL
FDMode=true
CATSerialPort=/dev/ttyUSB0
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[General]\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"MyCall=x\nMyGrid=y\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}

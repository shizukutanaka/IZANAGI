//! `direwolf.conf` 検出モジュール。
//!
//! Dire Wolf (APRS ソフトウェアTNC) の設定は大文字ディレクティブ
//! (`ADEVICE`/`ACHANNELS`/`MYCALL`/`MODEM`/`AGWPORT`/`KISSPORT`/
//! `TBEACON`/`PBEACON`/`IGSERVER`/`DIGIPEATER`/`PTT` 等) の
//! `KEYWORD 引数` 行で構成される。
//!
//! ```
//! let b = br#"ADEVICE null null
//! ACHANNELS 1
//! MYCALL N0CALL-9
//! MODEM 300 2130:2230 D+ /4
//! AGWPORT 8000
//! KISSPORT 8001
//! "#;
//! let c = izanagi_kit::direwolfconf::parse(b);
//! assert!(izanagi_kit::direwolfconf::detect(b));
//! assert_eq!(c.directives, 6);
//! ```

const KEYWORDS: &[&str] = &[
    "ACHANNELS",
    "ADEVICE",
    "ADEVICEPL",
    "AGWPORT",
    "AMAM",
    "BEACON",
    "CBEACON",
    "CHANNEL",
    "COMMENT",
    "DCD",
    "DIGIPEATER",
    "DWCHANNELS",
    "DWAIT",
    "FILTER",
    "FIX_BITS",
    "GPSD",
    "IGLOGIN",
    "IGSERVER",
    "IGTXVIA",
    "KISSPORT",
    "LATITUDE",
    "LONGITUDE",
    "MAXFRAME",
    "MAXNOV",
    "MODEM",
    "MYALT",
    "MYCALL",
    "NEWQUEL",
    "OBJECT",
    "PBEACON",
    "PERSIST",
    "PITPOINT",
    "PTT",
    "SLOTTIME",
    "SPEAK",
    "TTPOINT",
    "TBEACON",
    "TXDELAY",
    "TXFREQ",
    "TXTAIL",
    "UTELEM",
    "VIBEACON",
];

fn directive(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    KEYWORDS.contains(&head)
}

/// `b` が direwolf.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut dirs = 0usize;
    let mut anchor = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if directive(tr) {
            dirs += 1;
            if tr.starts_with("MYCALL") || tr.starts_with("ADEVICE") || tr.starts_with("ACHANNELS")
            {
                anchor += 1;
            }
        }
    }
    (anchor >= 1 && dirs >= 3) || dirs >= 5
}

/// direwolf.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct DirewolfConf {
    /// 既知ディレクティブ行数。
    pub directives: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を direwolf.conf として統計する。
pub fn parse(b: &[u8]) -> DirewolfConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = DirewolfConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if directive(tr) {
            c.directives += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"ADEVICE null null
ACHANNELS 1
MYCALL N0CALL
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 3);
    }

    #[test]
    fn detects_beacon_config() {
        let b = br#"PBEACON sendto=IG delay=0:30 every=10:00
TBEACON sendto=0 delay=0:30 every=10:00
IGSERVER t2.aprs.net
IGLOGIN N0CALL 12345
DIGIPEATER 0
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"MYCALL N0CALL\n"));
        assert!(!detect(b"foo bar\nbaz qux\nquux 1\n"));
        assert!(!detect(b"# MYCALL N0CALL\nADEVICE x\nMYCALL x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}

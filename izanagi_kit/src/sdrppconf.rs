//! `config.json` (SDR++) 検出モジュール。
//!
//! SDR++ の設定は JSON で、トップレベルに `"modules"` /
//! `"moduleInstances"` / `"menuElements"` / `"streams"` /
//! `"frequency"` / `"vfo"` / `"bandwidth"` / `"servers"` 等の
//! キーを持つ。
//!
//! ```
//! let b = br#"{
//!     "frequency": 144800000.0,
//!     "modules": ["radio", "recorder"],
//!     "moduleInstances": {"Radio": "radio"},
//!     "menuElements": ["radio"],
//!     "streams": {}
//! }
//! "#;
//! let c = izanagi_kit::sdrppconf::parse(b);
//! assert!(izanagi_kit::sdrppconf::detect(b));
//! assert_eq!(c.sdrpp_keys, 5);
//! ```

const KEYS: &[&str] = &[
    "bandplan",
    "bandwidth",
    "centerFrequency",
    "dataDirectory",
    "fftHold",
    "fftSmoothing",
    "fftWindow",
    "frequency",
    "max",
    "menuElements",
    "min",
    "moduleInstances",
    "modules",
    "offset",
    "recordingDirectory",
    "servers",
    "streams",
    "vfo",
    "waterfallUpdateRate",
    "zoom",
];

/// 行内の最初の `"key":` を返す。
fn jkey(line: &str) -> Option<&str> {
    let s = line.trim_start();
    let s = s.strip_prefix('"')?;
    let e = s.find('"')?;
    let key = &s[..e];
    if s[e + 1..].trim_start().starts_with(':') {
        Some(key)
    } else {
        None
    }
}

fn sdrpp_key(line: &str) -> bool {
    jkey(line).is_some_and(|k| KEYS.contains(&k))
}

/// `b` が SDR++ config.json に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        if sdrpp_key(l) {
            keys += 1;
        }
    }
    keys >= 2
}

/// SDR++ config.json の統計。
#[derive(Debug, Default, Clone)]
pub struct SdrppConf {
    /// 既知 SDR++ キー行数。
    pub sdrpp_keys: usize,
}

/// `b` を SDR++ config.json として統計する。
pub fn parse(b: &[u8]) -> SdrppConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SdrppConf::default();
    for l in t.lines() {
        if sdrpp_key(l) {
            c.sdrpp_keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"{
    "frequency": 144800000.0,
    "modules": []
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sdrpp_keys, 2);
    }

    #[test]
    fn detects_variant() {
        let b = br#""menuElements": ["a"],
"vfo": {}}"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\"name\": \"x\", \"version\": 1}"));
        assert!(!detect(b"frequency = 100\nmodules = a\n"));
        assert!(!detect(b"{\"modules\": [\"x\"]}"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sdrpp_keys, 0);
    }
}

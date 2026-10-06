//! `config.json` (V2Ray/Xray) 検出モジュール。
//!
//! V2Ray/Xray-core の設定は JSON で、`"inbounds"`/`"outbounds"`/
//! `"routing"`/`"dns"`/`"log"`/`"policy"`/`"stats"`/`"api"`/
//! `"transport"`/`"reverse"`/`"fakedns"`/`"observatory"`/
//! `"burstObservatory"`/`"mux"`/`"sniffing"` 等のキーを持つ。
//!
//! ```
//! let b = br#"{
//!     "log": {"loglevel": "warning"},
//!     "inbounds": [{"port": 1080, "protocol": "socks"}],
//!     "outbounds": [{"protocol": "freedom"}],
//!     "routing": {"domainStrategy": "AsIs"}
//! }
//! "#;
//! let c = izanagi_kit::v2rayconf::parse(b);
//! assert!(izanagi_kit::v2rayconf::detect(b));
//! assert_eq!(c.keys, 4);
//! ```

const KEYS: &[&str] = &[
    "api",
    "burstObservatory",
    "dns",
    "fakedns",
    "inbounds",
    "log",
    "observatory",
    "outbounds",
    "policy",
    "reverse",
    "routing",
    "stats",
    "transport",
];

const HINTS: &[&str] = &["inbounds", "outbounds"];

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

/// `b` が V2Ray/Xray config.json に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut hints = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                keys += 1;
            }
            if HINTS.contains(&k) {
                hints += 1;
            }
        }
    }
    (hints >= 2 && keys >= 2) || keys >= 3 || hints >= 1 && keys >= 3
}

/// V2Ray/Xray config.json の統計。
#[derive(Debug, Default, Clone)]
pub struct V2rayConf {
    /// 既知トップレベルキー行数。
    pub keys: usize,
    /// inbounds/outbounds 行数。
    pub inout: usize,
}

/// `b` を V2Ray/Xray config.json として統計する。
pub fn parse(b: &[u8]) -> V2rayConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = V2rayConf::default();
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                c.keys += 1;
            }
            if HINTS.contains(&k) {
                c.inout += 1;
            }
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
    "inbounds": [],
    "outbounds": []
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
        assert_eq!(c.inout, 2);
    }

    #[test]
    fn detects_routing() {
        let b = br#"{
    "log": {},
    "routing": {},
    "dns": {}
}
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\"inbounds\": []}"));
        assert!(!detect(b"{\"log\": {}, \"stats\": true}"));
        assert!(!detect(b"inbounds=[]\noutbounds=[]\nrouting={}\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

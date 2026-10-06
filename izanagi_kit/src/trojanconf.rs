//! `config.json` (trojan) 検出モジュール。
//!
//! trojan プロキシの設定は JSON で、`"run_type"`/`"local_addr"`/
//! `"local_port"`/`"remote_addr"`/`"remote_port"`/`"password"`/
//! `"ssl"`/`"alpn"`/`"sni"`/`"mux"`/`"log_level"`/`"verify"`/
//! `"verify_hostname"`/`"cert"`/`"key"`/`"fallback_addr"` 等の
//! キーを持つ。
//!
//! ```
//! let b = br#"{
//!     "run_type": "client",
//!     "local_addr": "127.0.0.1",
//!     "local_port": 1080,
//!     "remote_addr": "example.com",
//!     "remote_port": 443,
//!     "password": ["password1"],
//!     "ssl": {"verify": true, "sni": "example.com"}
//! }
//! "#;
//! let c = izanagi_kit::trojanconf::parse(b);
//! assert!(izanagi_kit::trojanconf::detect(b));
//! assert_eq!(c.keys, 7);
//! ```

const KEYS: &[&str] = &[
    "alpn",
    "cert",
    "curves",
    "dhparam",
    "fallback_addr",
    "fallback_port",
    "key",
    "key_password",
    "local_addr",
    "local_port",
    "log_level",
    "logfile",
    "mux",
    "mysql",
    "password",
    "prefer_ipv4",
    "private_key_password",
    "remote_addr",
    "remote_port",
    "reuse_port",
    "run_type",
    "session_ticket",
    "sigalgs",
    "sni",
    "ssl",
    "tcp",
    "verify",
    "verify_hostname",
    "websocket",
];

const HINTS: &[&str] = &[
    "run_type",
    "local_addr",
    "remote_addr",
    "verify_hostname",
    "fallback_addr",
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

/// `b` が trojan config.json に見えるかを返す。
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
    (hints >= 2 && keys >= 4) || keys >= 6
}

/// trojan config.json の統計。
#[derive(Debug, Default, Clone)]
pub struct TrojanConf {
    /// 既知キー行数。
    pub keys: usize,
    /// trojan 特有キー行数。
    pub hint_keys: usize,
}

/// `b` を trojan config.json として統計する。
pub fn parse(b: &[u8]) -> TrojanConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = TrojanConf::default();
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                c.keys += 1;
            }
            if HINTS.contains(&k) {
                c.hint_keys += 1;
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
    "run_type": "client",
    "local_addr": "127.0.0.1",
    "local_port": 1080,
    "remote_addr": "x.com",
    "remote_port": 443,
    "password": ["p"]
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 6);
    }

    #[test]
    fn detects_many() {
        let b = br#"{
    "ssl": {},
    "sni": "x",
    "mux": {},
    "alpn": ["h2"],
    "tcp": {},
    "cert": "/a"
}
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\"run_type\": \"x\", \"local_addr\": \"a\"}"));
        assert!(!detect(
            b"{\"local_addr\": \"a\", \"local_port\": 1, \"password\": []}"
        ));
        assert!(!detect(b"run_type=client\nlocal_addr=a\nremote_addr=b\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

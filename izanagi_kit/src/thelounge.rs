//! The Lounge `config.js` 検出モジュール。
//!
//! The Lounge (IRC クライアント) の設定は `module.exports = { ... }`
//! の JavaScript オブジェクト形式で、`public`/`host`/`port`/`bind`/
//! `theme`/`prefetch`/`defaults`/`leaveMessage` 等のキーが特徴。
//!
//! ```
//! let b = br#""use strict";
//! module.exports = {
//!     public: false,
//!     host: "0.0.0.0",
//!     port: 9000,
//!     theme: "morning",
//!     prefetch: true,
//!     defaults: {
//!         name: "Libera.Chat",
//!     },
//! };
//! "#;
//! let c = izanagi_kit::thelounge::parse(b);
//! assert!(izanagi_kit::thelounge::detect(b));
//! assert!(c.keys >= 5);
//! ```

const KEYS: &[&str] = &[
    "awayMessage",
    "bind",
    "defaults",
    "displayNetwork",
    "fetchMaxBytes",
    "fileUpload",
    "host",
    "leaveMessage",
    "lockNetwork",
    "log",
    "motd",
    "name",
    "nick",
    "port",
    "prefetch",
    "public",
    "reverseProxy",
    "theme",
    "tls",
    "webirc",
];

fn is_comment(t: &str) -> bool {
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

fn key_hit(t: &str) -> bool {
    // `key: value,` JS object literal key
    let Some(colon) = t.find(':') else {
        return false;
    };
    let key = t[..colon].trim();
    if key.starts_with('"') && key.ends_with('"') {
        let key = &key[1..key.len() - 1];
        return KEYS.contains(&key);
    }
    KEYS.contains(&key)
}

/// `b` が The Lounge config.js に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut exports = false;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if tr.contains("module.exports") {
            exports = true;
            continue;
        }
        if key_hit(tr) {
            keys += 1;
        }
    }
    exports && keys >= 3
}

/// The Lounge config.js の統計。
#[derive(Debug, Default, Clone)]
pub struct TheLoungeConf {
    /// `module.exports` 行があったか。
    pub has_exports: bool,
    /// 既知キー行数。
    pub keys: usize,
    /// ネストした `{`/`}` 行数。
    pub braces: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を The Lounge config.js として統計する。
pub fn parse(b: &[u8]) -> TheLoungeConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = TheLoungeConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr.contains("module.exports") {
            c.has_exports = true;
            continue;
        }
        if key_hit(tr) {
            c.keys += 1;
        }
        if tr.starts_with('{') || tr.starts_with('}') {
            c.braces += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#""use strict";
module.exports = {
    public: true,
    host: "0.0.0.0",
    port: 9000,
    theme: "morning",
};
"#;
        assert!(detect(b));
        let c = parse(b);
        assert!(c.has_exports);
        assert_eq!(c.keys, 4);
    }

    #[test]
    fn detects_quoted_keys() {
        let b = br#"module.exports = {
    "public": false,
    "theme": "senpai",
    "prefetch": true,
};
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"public: true\nhost: x\nport: 1\n"));
        assert!(!detect(b"module.exports = {};\n"));
        assert!(!detect(b"const x = { public: true };\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

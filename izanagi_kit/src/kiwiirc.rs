//! `config.conf` (Kiwi IRC) 検出モジュール。
//!
//! Kiwi IRC の `config.conf` はフラットな `key = value` 形式で、
//! `conf.*`、`plugins`、`upstream`、`hostname`、`port`、`webirc`、
//! `logLevel` 等のキーが使われる。
//!
//! ```
//! let b = br#"logLevel = 3
//! conf.outputFormat = compact
//! upstream.0 = unix:/tmp/kiwi.sock
//! webirc.foo.example.com = password
//! plugins = [ "/usr/share/kiwiirc/plugins" ]
//! hostname = kiwi.example.com
//! port = 8000
//! "#;
//! let c = izanagi_kit::kiwiirc::parse(b);
//! assert!(izanagi_kit::kiwiirc::detect(b));
//! assert_eq!(c.keys, 7);
//! ```

const KEY_PREFIXES: &[&str] = &[
    "conf.",
    "plugins",
    "upstream",
    "webirc",
    "hostname",
    "port",
    "logLevel",
    "tls",
    "cert",
    "key",
    "serverkey",
    "reverseProxies",
    "staticHttp",
    "transports",
    "identd",
    "gateway",
    "public_html",
    "getKiwiirc",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with(';')
}

fn key_hit(t: &str) -> bool {
    // key = value where key begins with a known prefix
    let Some(eq) = t.find('=') else {
        return false;
    };
    let key = t[..eq].trim();
    if key.is_empty() {
        return false;
    }
    KEY_PREFIXES.iter().any(|p| {
        if let Some(head) = p.strip_suffix('.') {
            key.starts_with(p) || key == head
        } else {
            key == *p || key.starts_with(&format!("{p}."))
        }
    })
}

/// `b` が Kiwi IRC config.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if key_hit(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// Kiwi IRC 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct KiwiircConf {
    /// 既知キー行数。
    pub keys: usize,
    /// `upstream.*`/`webirc.*` キー行数。
    pub upstream_keys: usize,
    /// `conf.*` キー行数。
    pub conf_keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Kiwi IRC 設定として統計する。
pub fn parse(b: &[u8]) -> KiwiircConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = KiwiircConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if key_hit(tr) {
            c.keys += 1;
            let key = tr[..tr.find('=').unwrap_or(0)].trim();
            if key.starts_with("upstream") || key.starts_with("webirc") {
                c.upstream_keys += 1;
            } else if key.starts_with("conf.") {
                c.conf_keys += 1;
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
        let b = br#"logLevel = 3
conf.outputFormat = compact
upstream.0 = unix:/tmp/kiwi.sock
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
        assert_eq!(c.conf_keys, 1);
        assert_eq!(c.upstream_keys, 1);
    }

    #[test]
    fn detects_minimal() {
        let b = br#"hostname = kiwi.example.com
port = 8000
plugins = [ "/plugins" ]
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"logLevel = 3\n"));
        assert!(!detect(b"foo = 1\nbar = 2\nbaz = 3\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

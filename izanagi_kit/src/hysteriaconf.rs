//! `config.yaml` (Hysteria) 検出モジュール。
//!
//! Hysteria プロキシの設定は YAML で、`"server"`/`"listen"`/
//! `"up"`/`"down"`/`"obfs"`/`"auth"`/`"tls"`/`"bandwidth"`/
//! `"recv_window_conn"`/`"recv_window"`/`"protocol"`/
//! `"server_name"`/`"insecure"`/`"quic"`/`"masquerade"` 等の
//! キーを持つ。
//!
//! ```
//! let b = br#"server: example.com:443
//! auth: password123
//! bandwidth:
//!   up: 20 mbps
//!   down: 100 mbps
//! masquerade: https://news.ycombinator.com/
//! "#;
//! let c = izanagi_kit::hysteriaconf::parse(b);
//! assert!(izanagi_kit::hysteriaconf::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEYS: &[&str] = &[
    "acme",
    "alpn",
    "auth",
    "bandwidth",
    "cert",
    "disable_mtu_discovery",
    "down",
    "fast_open",
    "fingerprint",
    "ignore_client_bandwidth",
    "insecure",
    "key",
    "listen",
    "masquerade",
    "masqueradeListenHTTP",
    "masqueradeListenHTTPS",
    "obfs",
    "protocol",
    "proxy",
    "quic",
    "recv_window",
    "recv_window_client",
    "recv_window_conn",
    "resolver",
    "server",
    "server_name",
    "sniff",
    "socks5",
    "tls",
    "up",
    "up_mbps",
    "down_mbps",
    "userpass",
];

const HINTS: &[&str] = &[
    "obfs",
    "masquerade",
    "recv_window_conn",
    "recv_window",
    "up_mbps",
    "down_mbps",
];

/// 行頭 `key:` (YAMLのキー) を返す。
fn yaml_key(t: &str) -> Option<&str> {
    if t.starts_with('-') || t.starts_with('#') {
        return None;
    }
    let e = t.find(':')?;
    let key = &t[..e];
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(key)
}

/// `b` が Hysteria config.yaml に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut hints = 0usize;
    for l in t.lines() {
        if let Some(k) = yaml_key(l.trim_start()) {
            if KEYS.contains(&k) {
                keys += 1;
            }
            if HINTS.contains(&k) {
                hints += 1;
            }
        }
    }
    (hints >= 1 && keys >= 2) || keys >= 4
}

/// Hysteria config.yaml の統計。
#[derive(Debug, Default, Clone)]
pub struct HysteriaConf {
    /// 既知キー行数。
    pub keys: usize,
    /// Hysteria 特有キー行数。
    pub hint_keys: usize,
}

/// `b` を Hysteria config.yaml として統計する。
pub fn parse(b: &[u8]) -> HysteriaConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = HysteriaConf::default();
    for l in t.lines() {
        if let Some(k) = yaml_key(l.trim_start()) {
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
        let b = br#"server: x.com:443
obfs: abc123
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
        assert_eq!(c.hint_keys, 1);
    }

    #[test]
    fn detects_many() {
        let b = br#"listen: :443
tls: {}
quic: {}
auth: pw
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"server: x\nlisten: :443\n"));
        assert!(!detect(b"name: a\nversion: 1\nport: 80\nhost: b\n"));
        assert!(!detect(b"server=x\nobfs=y\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

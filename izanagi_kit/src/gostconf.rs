//! `gost.yml` / `gost.json` (GOST v3) 検出モジュール。
//!
//! GOST トンネルツールの設定は YAML で、`"services"`/`"chains"`/
//! `"hops"`/`"admission"`/`"bypasses"`/`"resolvers"`/`"hosts"`/
//! `"limits"`/`"authers"`/`"observer"`/`"router"`/`"selectors"`/
//! `"metrics"`/`"profiling"`/`"record"`/`"redirect"` 等の
//! トップレベルキーを持つ。
//!
//! ```
//! let b = br#"services:
//! - name: service-0
//!   addr: ":8080"
//!   handler:
//!     type: http
//! chains:
//! - name: chain-0
//!   hops:
//!   - name: hop-0
//! "#;
//! let c = izanagi_kit::gostconf::parse(b);
//! assert!(izanagi_kit::gostconf::detect(b));
//! assert_eq!(c.keys, 2);
//! ```

const KEYS: &[&str] = &[
    "admission",
    "authers",
    "bypasses",
    "chains",
    "config",
    "controllers",
    "flibustier",
    "forwarders",
    "hosts",
    "hubs",
    "ingress",
    "limits",
    "loggers",
    "metrics",
    "observer",
    "profiler",
    "profiling",
    "recorders",
    "redirect",
    "registries",
    "reversers",
    "resolvers",
    "routers",
    "selectors",
    "services",
    "tun",
];

const HINTS: &[&str] = &["services", "chains", "hops", "bypasses", "authers"];

/// 行頭 `key:` (YAMLのキー、トップレベル=非インデントのみ) を返す。
fn top_key(l: &str) -> Option<&str> {
    if l.starts_with(' ') || l.starts_with('\t') || l.starts_with('-') || l.starts_with('#') {
        return None;
    }
    let e = l.find(':')?;
    let key = &l[..e];
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(key)
}

/// `b` が GOST 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut hints = 0usize;
    for l in t.lines() {
        if let Some(k) = top_key(l) {
            if KEYS.contains(&k) {
                keys += 1;
            }
            if HINTS.contains(&k) {
                hints += 1;
            }
        }
    }
    (hints >= 1 && keys >= 2) || keys >= 3
}

/// GOST 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct GostConf {
    /// 既知トップレベルキー行数。
    pub keys: usize,
    /// GOST 特有キー行数。
    pub hint_keys: usize,
}

/// `b` を GOST 設定として統計する。
pub fn parse(b: &[u8]) -> GostConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = GostConf::default();
    for l in t.lines() {
        if let Some(k) = top_key(l) {
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
        let b = br#"services:
- name: s0
chains:
- name: c0
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn detects_extra() {
        let b = br#"services:
- addr: ":8080"
bypasses:
- matchers: []
resolvers:
- nameservers: []
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"services:\n- name: s\n"));
        assert!(!detect(b"apiVersion: v1\nkind: ConfigMap\nmetadata:\n"));
        assert!(!detect(b"services=x\nchains=y\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

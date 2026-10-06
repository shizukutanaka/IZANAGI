//! `config.yaml` (CrowdSec) 検出モジュール。
//!
//! CrowdSec の設定は YAML で、トップレベルに
//! `crowdsec_service`/`cscli`/`api`/`prometheus`/`common`/
//! `db_config`/`acquisition`/`profiles`/`plugin_config`/
//! `simulation`/`lapi_credentials`/`patterns`/`whitelist` 等の
//! キーを持つ。
//!
//! ```
//! let b = br#"common:
//!   daemonize: true
//! crowdsec_service:
//!   acquisition_path: /etc/crowdsec/acquis.yaml
//!   profiles_path: /etc/crowdsec/profiles.yaml
//! api:
//!   server:
//!     listen_uri: 127.0.0.1:8080
//! "#;
//! let c = izanagi_kit::crowdsec::parse(b);
//! assert!(izanagi_kit::crowdsec::detect(b));
//! assert_eq!(c.keys, 3);
//! ```

const KEYS: &[&str] = &[
    "acquisition",
    "api",
    "api_client",
    "bouncers",
    "common",
    "config_paths",
    "crowdsec_service",
    "cscli",
    "cti",
    "db_config",
    "disable",
    "engine",
    "hub",
    "lapi_credentials",
    "load_decisions",
    "patterns",
    "plugin_config",
    "prometheus",
    "profiles",
    "profiling",
    "papi",
    "simulation",
    "whitelist",
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

fn crowdsec_key(l: &str) -> bool {
    yaml_key(l.trim_start()).is_some_and(|k| KEYS.contains(&k))
}

/// `b` が CrowdSec config.yaml に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        if crowdsec_key(l) {
            keys += 1;
        }
    }
    keys >= 2
}

/// CrowdSec config.yaml の統計。
#[derive(Debug, Default, Clone)]
pub struct CrowdsecConf {
    /// 既知 CrowdSec キー行数。
    pub keys: usize,
}

/// `b` を CrowdSec config.yaml として統計する。
pub fn parse(b: &[u8]) -> CrowdsecConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = CrowdsecConf::default();
    for l in t.lines() {
        if crowdsec_key(l) {
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
        let b = br#"crowdsec_service:
  enable: true
api:
  server:
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn detects_variant() {
        let b = br#"common: {}
db_config: {}
prometheus:
  enabled: true
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"api: v1\nkind: Config\n"));
        assert!(!detect(b"crowdsec_service: {}\n"));
        assert!(!detect(b"name: x\nversion: 1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

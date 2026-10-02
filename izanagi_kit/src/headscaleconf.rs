//! Headscale `config.yaml`/`config.yml` の検出と構造カウント。
//!
//! `server_url`/`listen_addr`/`metrics_listen_addr`/`grpc_listen_addr`/
//! `grpc_allow_insecure`/`private_key_path`/`noise`/`prefixes`/`derp`/`database`/
//! `acme_email`/`tls_*`/`unix_socket`/`oidc`/`logtail`/`dns`/`policy`/
//! `randomize_client_port`/`cli`/`ip_allocation` 等の既知トップレベルキーを識別する。
//!
//! ```
//! let c = izanagi_kit::headscaleconf::parse(
//!     b"server_url: http://127.0.0.1:8080\nlisten_addr: 0.0.0.0:8080\nnoise:\n  private_key_path: /var/lib/headscale/noise.key\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert!(izanagi_kit::headscaleconf::detect(
//!     b"server_url: https://hs.example.com\nlisten_addr: 0.0.0.0:8080\n"));
//! ```

/// 既知トップレベルキー(スネークケース)。
const TOP_KEYS: &[&str] = &[
    "acme_email",
    "acme_url",
    "base_domain",
    "cli",
    "database",
    "derp",
    "disable_check_updates",
    "dns",
    "ephemeral_node_inactivity_timeout",
    "grpc_allow_insecure",
    "grpc_listen_addr",
    "ip_allocation",
    "ispaused",
    "letsencrypt",
    "listen_addr",
    "log",
    "logtail",
    "metrics_listen_addr",
    "noise",
    "oidc",
    "policy",
    "prefixes",
    "private_key_path",
    "prometheus_listen_addr",
    "randomize_client_port",
    "server_url",
    "tls",
    "tls_cert_path",
    "tls_key_path",
    "unix_socket",
    "unix_socket_permission",
];
/// ネスト既知サブキー。
const SUBKEYS: &[&str] = &[
    "access_key",
    "allowed_domains",
    "allowed_groups",
    "allowed_users",
    "auto_update",
    "base_domain",
    "ca",
    "cert",
    "cli",
    "client_id",
    "client_secret",
    "controller",
    "custom_css_path",
    "debug",
    "derp",
    "disabled",
    "distribution",
    "enabled",
    "extra_params",
    "format",
    "grpc_allow_insecure",
    "grpc_listen_addr",
    "hostname",
    "ip_allocation",
    "issuer",
    "key",
    "map",
    "metrics",
    "mode",
    "name",
    "omit_regions",
    "path",
    "paths",
    "pkce",
    "policy",
    "postgres_pass",
    "postgres_schema",
    "postgres_ssl",
    "postgres_user",
    "prefix",
    "private_key_path",
    "profile",
    "region",
    "regions",
    "refresh_interval",
    "scope",
    "secret",
    "sqlite_path",
    "sqlite_write_ahead_log",
    "tls",
    "type",
    "verify",
    "vhost",
];

/// Headscale 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルキー(非インデント)。
    pub sections: usize,
    /// ネスト `key: value` 既知代入。
    pub options: usize,
    /// `- ` リスト項目。
    pub items: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// YAML の `key:` 行のキーを返す。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        None
    } else {
        Some(k)
    }
}

/// b が headscale config かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        let indent = line.len() - line.trim_start().len();
        if let Some(k) = yaml_key(t) {
            if indent == 0 && TOP_KEYS.contains(&k) || indent > 0 && SUBKEYS.contains(&k) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        items: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('-') {
            c.items += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if yaml_key(t).is_some() {
            if indent == 0 {
                c.sections += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# headscale\nserver_url: http://127.0.0.1:8080\nlisten_addr: 0.0.0.0:8080\nmetrics_listen_addr: 127.0.0.1:9090\nnoise:\n  private_key_path: /var/lib/headscale/noise_private.key\nprefixes:\n  v6: fd7a:115c:a1e0::/48\n  v4: 100.64.0.0/10\ndatabase:\n  type: sqlite3\n  sqlite:\n    path: /var/lib/headscale/db.sqlite\n";

    #[test]
    fn headscaleconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_headscale() {
        assert!(!detect(b"foo:\n  bar: baz\n"));
        assert!(!detect(b"hello\n"));
    }
}

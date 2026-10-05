//! Headscale `config.yaml` (self-hosted Tailscale control server).
//!
//! ```
//! let b = b"server_url: https://headscale.example.com\nlisten_addr: 0.0.0.0:8080\nmetrics_listen_addr: 127.0.0.1:9090\ngrpc_listen_addr: 127.0.0.1:50443\nprivate_key_path: /var/lib/headscale/private.key\nnoise:\n  private_key_path: /var/lib/headscale/noise_private.key\nprefixes:\n  v6: fd7a:115c:a1e0::/48\n  v4: 100.64.0.0/10\n  allocation: sequential\nderp:\n  server:\n    enabled: false\n    region_id: 999\n    region_code: headscale\n  urls:\n    - https://controlplane.tailscale.com/derpmap/default\ndatabase:\n  type: sqlite\n  sqlite:\n    path: /var/lib/headscale/db.sqlite\npolicy:\n  mode: file\n  path: \"\"\ndns:\n  magic_dns: true\n  base_domain: example.com\n  nameservers:\n    global:\n      - 1.1.1.1\n";
//! assert!(izanagi_kit::headscaleconf::detect(b));
//! let c = izanagi_kit::headscaleconf::Headscale::parse(b).unwrap();
//! assert!(c.sections >= 5);
//! ```
const SECTION_KEYS: &[&str] = &[
    "noise:",
    "prefixes:",
    "derp:",
    "disable_check_updates:",
    "ephemeral_node_inactivity_timeout:",
    "database:",
    "acme_url:",
    "acme_email:",
    "tls_letsencrypt_hostname:",
    "tls_letsencrypt_cache_dir:",
    "tls_letsencrypt_challenge_type:",
    "tls_letsencrypt_listen:",
    "tls_cert_path:",
    "tls_key_path:",
    "tls_client_auth_mode:",
    "logtail:",
    "randomize_client_port:",
    "oidc:",
    "policy:",
    "dns:",
    "unix_socket:",
    "unix_socket_permission:",
    "grpc_listen_addr:",
    "grpc_allow_insecure:",
    "server_url:",
    "listen_addr:",
    "metrics_listen_addr:",
    "private_key_path:",
    "log:",
    "node_update_check_interval:",
    "prefixv6:",
    "prefixv4:",
];
const KEYS: &[&str] = &[
    "v6",
    "v4",
    "allocation",
    "sequential",
    "random",
    "enabled",
    "region_id",
    "region_code",
    "region_name",
    "stun_listen_addr",
    "private_key_path",
    "automatically_add_embedded_derp_region",
    "paths",
    "urls",
    "update_frequency",
    "type",
    "sqlite",
    "sqlite_path",
    "postgres",
    "host",
    "port",
    "name",
    "user",
    "pass",
    "ssl",
    "max_open_connections",
    "max_idle_connections",
    "conn_max_idle_time_secs",
    "debug",
    "gorm",
    "slow_threshold",
    "strip_email_domain",
    "only_start_if_oidc_is_available",
    "issuer",
    "client_id",
    "client_secret",
    "client_secret_path",
    "scope",
    "extra_params",
    "allowed_domains",
    "allowed_groups",
    "pkce",
    "use_expiry_from_token",
    "expiry",
    "map_legacy_users",
    "magic_dns",
    "base_domain",
    "override_local_dns",
    "nameservers",
    "global",
    "split",
    "search_domains",
    "extra_records",
    "resolvconf",
    "mode",
    "path",
    "acl",
    "policy_path",
    "level",
    "format",
    "enabled",
    "control_url",
    "insecure",
    "tailnet",
    "initialize",
    "authkey",
    "preauthkeys",
    "reusable",
    "ephemeral",
    "expiration",
    "acl_tags",
    "tsnet",
    "hostname",
    "derp",
    "verify",
    "region",
    "namespace",
    "node",
    "route",
    "advertise",
    "exit",
    "approval",
];

/// Detect a Headscale config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut sec = 0usize;
    for k in SECTION_KEYS {
        if t.contains(k) {
            sec += 1;
        }
    }
    sec >= 2 || t.contains("server_url:") && sec >= 1
}

/// Structural counts for a Headscale config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Headscale {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `name:` section keys (any depth).
    pub sections: usize,
    /// `name: value` leaf lines.
    pub leaves: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Headscale {
    /// Parse structural counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            leaves: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.ends_with(':') && !tr.starts_with('-') {
                c.sections += 1;
            } else if tr.contains(':') && !tr.starts_with('-') {
                c.leaves += 1;
            }
        }
        for k in SECTION_KEYS {
            c.keys += t.matches(k).count();
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"server_url: https://headscale.example.com\nlisten_addr: 0.0.0.0:8080\nmetrics_listen_addr: 127.0.0.1:9090\nnoise:\n  private_key_path: /var/lib/headscale/noise_private.key\nprefixes:\n  v6: fd7a:115c:a1e0::/48\n  v4: 100.64.0.0/10\nderp:\n  urls:\n    - https://controlplane.tailscale.com/derpmap/default\ndatabase:\n  type: sqlite\ndns:\n  magic_dns: true\n";
        assert!(detect(b));
        let c = Headscale::parse(b).unwrap();
        assert!(c.sections >= 5);
        assert!(c.leaves >= 7);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_yaml() {
        let b = b"foo:\n  a: 1\nbar:\n  b: 2\n";
        assert!(!detect(b));
        assert!(Headscale::parse(b).is_none());
    }
}

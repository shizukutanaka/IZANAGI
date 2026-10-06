//! Kong declarative configuration (decK / `_format_version` YAML) detection
//! and census.
//!
//! Detects `_format_version:` and counts meta keys (`_format_version`/
//! `_transform`/`_workspace`/`_comment`/`_select_tags`/`_ignore`/`konnect`/
//! `info`), top-level entity sections (`services:`/`routes:`/`consumers:`/
//! `plugins:`/`upstreams:`/`targets:`/`certificates:`/`ca_certificates:`/
//! `snis:`/`key_sets:`/`keys:`/`vaults:`/`partials:`/`filter_chains:`/
//! `groups:`/`rbac_roles:`/`basic_auth_credentials:`/`key_auth_credentials:`/
//! `jwt_secrets:`/`hmac_auth_credentials:`/`oauth2_credentials:`/
//! `mtls_auth_credentials:`/`acls:`/`consumer_groups:`), `- name:` entity
//! entries, entity keys (`host`/`port`/`protocol`/`path`/`paths`/`hosts`/
//! `methods`/`headers`/`uris`/`snis`/`strip_path`/`preserve_host`/
//! `regex_priority`/`retries`/`connect_timeout`/`read_timeout`/`write_timeout`/
//! `client_certificate`/`tls_verify`/`enabled`/`route`/`service`/`config`/
//! `tags`/`username`/`custom_id`/`key`/`credentials`/`algorithm`/`secret`/
//! `weight`/`target`/`healthchecks`/`checks`/`active`/`passive`/`threshold`/
//! `interval`/`timeout`/`type`/`slots`/`hash_on`/`hash_fallback`/
//! `loadbalancing`/`consumer`/`instance_name`/`ca_certificate`/`cert`/`key`/
//! `cert_alt`/`key_alt`/`public_key`/`private_key`/`kid`/`jwk`/`pem`/
//! `ssl_certificate`/`ssl_certificate_key`/`prefix`/`description`/`keys`/
//! `set`/`env`), and `#` comment lines.
//!
//! ```
//! let b = b"_format_version: \"3\"\n_transform: true\nservices:\n- name: api\n  host: backend\n  port: 8080\n  routes:\n  - name: api-route\n    paths:\n    - /api\nplugins:\n- name: rate-limiting\n  config:\n    minute: 100\n";
//! assert!(izanagi_kit::kong::detect(b));
//! let c = izanagi_kit::kong::Kong::parse(b).unwrap();
//! assert_eq!(c.version_keys, 2);
//! assert_eq!(c.sections, 3);
//! ```

/// Parsed Kong declarative config summary.
#[derive(Debug, Clone)]
pub struct Kong {
    /// `_format_version`/`_transform`/`_workspace`/`_comment`/`konnect` meta keys.
    pub version_keys: usize,
    /// top-level entity sections (`services:`/`routes:`/`plugins:`/…).
    pub sections: usize,
    /// `- name:` entity entries.
    pub entries: usize,
    /// entity keys (`host`/`port`/`paths`/`config`/`healthchecks`/…).
    pub entity_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const VERSION_KEYS: &[&str] = &[
    "_format_version:",
    "_transform:",
    "_workspace:",
    "_comment:",
    "_select_tags:",
    "_ignore:",
    "konnect:",
    "info:",
];

const SECTIONS: &[&str] = &[
    "services:",
    "routes:",
    "consumers:",
    "plugins:",
    "upstreams:",
    "targets:",
    "certificates:",
    "ca_certificates:",
    "snis:",
    "key_sets:",
    "keys:",
    "vaults:",
    "partials:",
    "filter_chains:",
    "groups:",
    "rbac_roles:",
    "basic_auth_credentials:",
    "key_auth_credentials:",
    "jwt_secrets:",
    "hmac_auth_credentials:",
    "oauth2_credentials:",
    "mtls_auth_credentials:",
    "acls:",
    "consumer_groups:",
];

const ENTITY_KEYS: &[&str] = &[
    "host:",
    "port:",
    "protocol:",
    "path:",
    "paths:",
    "hosts:",
    "methods:",
    "headers:",
    "uris:",
    "snis:",
    "strip_path:",
    "preserve_host:",
    "regex_priority:",
    "retries:",
    "connect_timeout:",
    "read_timeout:",
    "write_timeout:",
    "client_certificate:",
    "tls_verify:",
    "enabled:",
    "route:",
    "service:",
    "config:",
    "tags:",
    "username:",
    "custom_id:",
    "key:",
    "credentials:",
    "algorithm:",
    "secret:",
    "weight:",
    "target:",
    "healthchecks:",
    "checks:",
    "active:",
    "passive:",
    "threshold:",
    "interval:",
    "timeout:",
    "type:",
    "slots:",
    "hash_on:",
    "hash_fallback:",
    "loadbalancing:",
    "consumer:",
    "instance_name:",
    "ca_certificate:",
    "cert:",
    "cert_alt:",
    "key_alt:",
    "public_key:",
    "private_key:",
    "kid:",
    "jwk:",
    "pem:",
    "ssl_certificate:",
    "ssl_certificate_key:",
    "prefix:",
    "description:",
    "set:",
    "env:",
];
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim_start(), key))
}

/// Detects Kong declarative config files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    has_key(&t, "_format_version")
}

impl Kong {
    /// Parses a Kong declarative config, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            version_keys: 0,
            sections: 0,
            entries: 0,
            entity_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let key = tr.trim_start_matches("- ").trim_start();
            if VERSION_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.version_keys += 1;
            }
            if SECTIONS.iter().any(|k| tr.starts_with(k)) {
                c.sections += 1;
            }
            if tr.starts_with("- name:") {
                c.entries += 1;
            }
            if ENTITY_KEYS.iter().any(|k| key.starts_with(k)) {
                c.entity_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# _format_version: \"3.0\"\n"));
    }

    #[test]
    fn detects_and_counts() {
        let b = b"_format_version: \"3\"\n_transform: true\nservices:\n- name: api\n  host: backend\n  port: 8080\n  routes:\n  - name: api-route\n    paths:\n    - /api\nplugins:\n- name: rate-limiting\n  config:\n    minute: 100\n";
        let c = Kong::parse(b).unwrap();
        assert_eq!(c.version_keys, 2);
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 3);
        assert!(c.entity_keys >= 3);
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(Kong::parse(b"key: value\n").is_none());
        assert!(!detect(b"services:\n  app:\n    image: x\n"));
    }
}

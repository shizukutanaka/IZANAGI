//! Parser for New Relic agent configuration (`newrelic.yml` for the Java /
//! Ruby / Python / Node agents, and `newrelic-infra.yml` for the infrastructure
//! agent).
//!
//! Counts top-level keys (`license_key`, `app_name`, `log_level`, `enabled`,
//! `audit_mode`, `high_security`), environment sections (`common:`,
//! `production:`, `staging:`, `development:`, `test:`), feature blocks
//! (`transaction_tracer:`, `error_collector:`, `browser_monitoring:`,
//! `distributed_tracing:`, `application_logging:`, `span_events:`,
//! `transaction_events:`, `slow_sql:`, `cross_application_tracer:`,
//! `thread_profiler:`, `jfr:`, `custom_instrumentation_editor:`, `ai_monitoring:`),
//! and `#` comments.
//!
//! ```
//! let b = b"common:\n  license_key: abc\n  app_name: svc\n";
//! assert!(izanagi_kit::newrelic::detect(b));
//! let c = izanagi_kit::newrelic::Newrelic::parse(b).unwrap();
//! assert_eq!(c.env_sections, 1);
//! assert_eq!(c.keys, 3);
//! ```

/// Parsed newrelic conf summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Newrelic {
    /// Total `key: value` entries.
    pub keys: usize,
    /// Environment sections (`common:`/`production:`/`staging:`/`development:`/`test:`).
    pub env_sections: usize,
    /// Feature blocks (`transaction_tracer:`/`error_collector:`/... any non-env `key:` block).
    pub feature_blocks: usize,
    /// `enabled:`/`audit_mode:`/`high_security:` toggle-style keys.
    pub toggles: usize,
    /// Identity keys (`license_key`, `app_name`, `application_name`, `proxy_host`, `proxy_port`, `proxy_user`, `proxy_password`, `host`, `port`, `labels`, `log_file_path`, `apdex_t`).
    pub identity: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const ENV_SECTIONS: &[&str] = &[
    "common",
    "production",
    "staging",
    "development",
    "test",
    "default",
];

const IDENTITY_KEYS: &[&str] = &[
    "license_key",
    "app_name",
    "application_name",
    "insert_key",
    "proxy_host",
    "proxy_port",
    "proxy_user",
    "proxy_password",
    "proxy_scheme",
    "proxy_workstation",
    "proxy_domain",
    "host",
    "port",
    "labels",
    "log_file_path",
    "log_file_name",
    "apdex_t",
    "ca_bundle_path",
    "enable_auto_app_naming",
    "enable_auto_transaction_naming",
    "custom_hostnames",
    "display_name",
    "entity_guid",
];

fn key_of(s: &str) -> &str {
    match s.find(':') {
        Some(c) => s[..c]
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .next()
            .unwrap_or(""),
        None => "",
    }
}

/// Detects newrelic yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("license_key") && s.contains(':') {
            hits += 2;
        }
        if s.starts_with("app_name") && s.contains(':') {
            hits += 2;
        }
        if s.contains("transaction_tracer")
            || s.contains("error_collector")
            || s.contains("distributed_tracing")
            || s.contains("browser_monitoring")
        {
            hits += 1;
        }
        if s.contains("newrelic") {
            hits += 1;
        }
    }
    hits >= 3
}

impl Newrelic {
    /// Parse newrelic yaml; `None` when no `key:` entries found.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let Ok(t) = core::str::from_utf8(b) else {
            return None;
        };
        let mut c = Self {
            keys: 0,
            env_sections: 0,
            feature_blocks: 0,
            toggles: 0,
            identity: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let key = key_of(s);
            if key.is_empty() {
                continue;
            }
            c.keys += 1;
            if l.trim_end().ends_with(':') {
                if ENV_SECTIONS.contains(&key) {
                    c.env_sections += 1;
                } else {
                    c.feature_blocks += 1;
                }
            }
            if key == "enabled" || key == "audit_mode" || key == "high_security" {
                c.toggles += 1;
            }
            if IDENTITY_KEYS.contains(&key) {
                c.identity += 1;
            }
        }
        (c.keys > 0).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_newrelic() {
        assert!(detect(b"common:\n  license_key: k\n  app_name: svc\n  distributed_tracing:\n    enabled: true\n"));
        assert!(!detect(b"name: x\n"));
    }

    #[test]
    fn counts_blocks() {
        let c = Newrelic::parse(
            b"common:\n  license_key: k\n  app_name: a\nproduction:\n  transaction_tracer:\n    enabled: true\n  error_collector:\n    enabled: false\n",
        )
        .unwrap();
        assert_eq!(c.env_sections, 2);
        assert_eq!(c.feature_blocks, 2);
        assert_eq!(c.toggles, 2);
        assert!(c.keys >= 6);
    }

    #[test]
    fn rejects_empty() {
        assert!(Newrelic::parse(b"nothing\n").is_none());
    }
}

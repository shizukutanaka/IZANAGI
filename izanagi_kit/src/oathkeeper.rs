//! Ory Oathkeeper `oathkeeper.yaml` census.
//!
//! Sections `serve`(`proxy`/`api`/`prometheus`)/`access_rules`/
//! `authenticators`/`authorizers`/`mutators`/`errors`/`log`/`tracing`/
//! `profiling`/`version`. Handler sub-keys under the three middleware
//! sections counted separately (`noop`/`cookie_session`/
//! `oauth2_introspection`/`jwt`/`bearer_token`/
//! `oauth2_client_credentials`/`anonymous`/`unauthorized`,
//! `allow`/`deny`/`keto_engine_acp_ory`/`remote`/`remote_json`,
//! `noop`/`id_token`/`header`/`hydrator`).
//!
//! ```rust
//! let o = "serve:\n  proxy:\n    port: 4455\naccess_rules:\n  repositories: []\nauthenticators:\n  noop:\n    enabled: true\n";
//! let c = izanagi_kit::oathkeeper::Oathkeeper::parse(o.as_bytes()).unwrap();
//! assert_eq!(c.handlers, 1);
//! ```

/// Oathkeeper config census.
#[derive(Debug, Clone)]
pub struct Oathkeeper {
    /// Top-level `key:` section headers.
    pub sections: usize,
    /// Nested `key:`/`key: value` settings.
    pub settings: usize,
    /// Authenticator/authorizer/mutator handler names.
    pub handlers: usize,
    /// Recognised Oathkeeper option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "serve",
    "proxy",
    "api",
    "prometheus",
    "access_rules",
    "authenticators",
    "authorizers",
    "mutators",
    "errors",
    "log",
    "tracing",
    "profiling",
    "version",
    "host",
    "port",
    "socket",
    "cors",
    "enabled",
    "allowed_origins",
    "timeout",
    "read",
    "write",
    "idle",
    "repositories",
    "matching_strategy",
    "max_cache_size",
    "noop",
    "cookie_session",
    "oauth2_introspection",
    "jwt",
    "bearer_token",
    "oauth2_client_credentials",
    "anonymous",
    "unauthorized",
    "allow",
    "deny",
    "keto_engine_acp_ory",
    "remote",
    "remote_json",
    "id_token",
    "header",
    "hydrator",
    "config",
    "introspection_url",
    "subject_from",
    "claims",
    "required_scope",
    "target_audience",
    "trusted_issuers",
    "allowed_algorithms",
    "jwks_urls",
    "forward_http_headers",
    "strip_path",
    "preserve_host",
    "preserve_query",
    "preserve_fragment",
    "retry",
    "no_local_verification",
    "token_url",
    "scope_from",
    "limit",
    "sample_rate",
    "dialect",
    "base_url",
    "namespace_id",
    "environment",
    "endpoint",
    "api_key",
    "check",
    "check_context_key",
    "subject",
    "context",
    "headers",
    "values",
    "queries",
    "key",
    "ttl",
    "mutator",
    "issuer_url",
    "jwks_url",
    "claims_field",
    "api_passthrough",
    "fallback",
    "when",
    "handlers",
    "redirect",
    "json",
    "content_type",
    "message",
    "code",
    "verbose",
    "authenticator",
    "authorizer",
    "mutator_config",
    "level",
    "format",
    "leak_sensitive_values",
    "redaction",
    "service_name",
    "provider",
    "datastore",
    "middleware_error_handler",
    "post_decision",
    "pre",
    "post",
];

const HANDLERS: &[&str] = &[
    "noop",
    "cookie_session",
    "oauth2_introspection",
    "jwt",
    "bearer_token",
    "oauth2_client_credentials",
    "anonymous",
    "unauthorized",
    "allow",
    "deny",
    "keto_engine_acp_ory",
    "remote",
    "remote_json",
    "id_token",
    "header",
    "hydrator",
];

/// Whether the buffer looks like an Oathkeeper config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("access_rules")
        || t.contains("authenticators:")
        || t.contains("authorizers:")
        || t.contains("mutators:")
        || t.contains("keto_engine_acp_ory")
        || t.contains("oauth2_introspection")
}

impl Oathkeeper {
    /// Parse an oathkeeper.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            handlers: 0,
            named: 0,
        };
        let mut ctx = "";
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            let Some(colon) = s.find(':') else {
                continue;
            };
            let key = s[..colon].trim().trim_start_matches('-').trim();
            if indent == 0 {
                ctx = key;
                c.sections += 1;
            } else {
                c.settings += 1;
                if matches!(ctx, "authenticators" | "authorizers" | "mutators")
                    && indent <= 2
                    && HANDLERS.contains(&key)
                {
                    c.handlers += 1;
                }
            }
            if KEYS.contains(&key) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let b = concat!(
            "serve:\n",
            "  proxy:\n",
            "    port: 4455\n",
            "    timeout:\n",
            "      read: 5s\n",
            "  api:\n",
            "    port: 4456\n",
            "access_rules:\n",
            "  matching_strategy: glob\n",
            "  repositories:\n",
            "    - file:///etc/rules.json\n",
            "authenticators:\n",
            "  noop:\n",
            "    enabled: true\n",
            "  cookie_session:\n",
            "    enabled: true\n",
            "    config:\n",
            "      check_session_url: https://session\n",
            "      only:\n",
            "        - session\n",
            "  oauth2_introspection:\n",
            "    enabled: true\n",
            "    config:\n",
            "      introspection_url: https://hydra/introspect\n",
            "      required_scope: [read]\n",
            "  anonymous:\n",
            "    enabled: true\n",
            "authorizers:\n",
            "  allow:\n",
            "    enabled: true\n",
            "  deny:\n",
            "    enabled: false\n",
            "  keto_engine_acp_ory:\n",
            "    enabled: true\n",
            "    config:\n",
            "      base_url: http://keto/\n",
            "mutators:\n",
            "  noop:\n",
            "    enabled: true\n",
            "  id_token:\n",
            "    enabled: true\n",
            "    config:\n",
            "      issuer_url: http://oathkeeper/\n",
            "      jwks_url: file:///jwks.json\n",
            "  header:\n",
            "    enabled: true\n",
            "errors:\n",
            "  fallback:\n",
            "    - json\n",
            "  handlers:\n",
            "    json:\n",
            "      enabled: true\n",
            "      config:\n",
            "        verbose: true\n",
        );
        let c = Oathkeeper::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.handlers, 10);
        assert!(c.settings >= 30);
        assert!(c.named >= 25);
    }

    #[test]
    fn rejects_other() {
        assert!(Oathkeeper::parse(b"foo: 1").is_none());
    }
}

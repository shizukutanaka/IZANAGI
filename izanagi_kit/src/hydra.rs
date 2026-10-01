//! Ory Hydra `hydra.yaml` census.
//!
//! Sections `serve`(`public`/`admin`)/`urls`(`self`/`login`/`consent`/
//! `logout`/`error`/`post_logout_redirect`)/`dsn`/`secrets`(`system`/
//! `cookie`/`oidc`)/`oauth2`/`ttl`/`oidc`/`clients`/`log`/`tracing`/
//! `metrics`/`webfinger`/`pkce`/`feature_flags`/`strategies`/`dev`.
//!
//! ```rust
//! let h = "dsn: memory\nserve:\n  public:\n    port: 4444\nurls:\n  self:\n    issuer: https://hydra/\nsecrets:\n  system: [x]\n";
//! let c = izanagi_kit::hydra::Hydra::parse(h.as_bytes()).unwrap();
//! assert_eq!(c.sections, 4);
//! ```

/// Hydra config census.
#[derive(Debug, Clone)]
pub struct Hydra {
    /// Top-level `key:` section headers.
    pub sections: usize,
    /// Nested `key:`/`key: value` settings.
    pub settings: usize,
    /// Recognised Hydra option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "serve",
    "public",
    "admin",
    "urls",
    "dsn",
    "secrets",
    "oauth2",
    "ttl",
    "oidc",
    "clients",
    "log",
    "tracing",
    "metrics",
    "webfinger",
    "pkce",
    "feature_flags",
    "strategies",
    "dev",
    "host",
    "port",
    "socket",
    "owner",
    "group",
    "mode",
    "cors",
    "enabled",
    "allowed_origins",
    "allowed_methods",
    "allowed_headers",
    "exposed_headers",
    "allow_credentials",
    "max_age",
    "debug",
    "self",
    "login",
    "consent",
    "logout",
    "error",
    "post_logout_redirect",
    "identity_provider",
    "system",
    "cookie",
    "get_system_secret",
    "get_cookie_secrets",
    "global",
    "pairwise",
    "access_token_strategy",
    "allowed_top_level_claims",
    "exclude_not_before_claim",
    "grant_access_token_audience",
    "mirror_http_status_codes",
    "refresh_token_hook",
    "client_credentials",
    "token_at_hook",
    "authorization_code",
    "access_token",
    "refresh_token",
    "id_token",
    "refresh_token_lifespan",
    "access_token_lifespan",
    "authorization_code_lifespan",
    "id_token_lifespan",
    "min_refresh_entropy",
    "expose_internal_errors",
    "scope_ask_strategy",
    "subject_identifiers",
    "pairs",
    "supported_types",
    "match_key",
    "oidc_discovery",
    "subject_identifier_salt",
    "dynamic_client_registration",
    "default_scope",
    "enforce_redirect_uris",
    "http",
    "follow_www_form_urlencoded_parameters",
    "follow_uri_form_post_parameters",
    "grant_type",
    "level",
    "format",
    "redirection_for_ui",
    "service_name",
    "provider",
    "span_name",
    "zipkin",
    "jaeger",
    "otlp",
    "protocol",
    "endpoint",
    "insecure",
    "sampling",
    "server_url",
    "sampling_rate",
    "prometheus",
    "database_name",
    "collapsing_path",
    "secrets_cipher",
    "keys",
    "cookie_store",
    "same_site_mode",
    "same_site_legacy_workaround",
    "secret",
    "rotation",
    "jwt",
    "bearer_token",
    "enigma_hmac_secret",
    "internal",
    "token",
    "access",
    "code",
    "hybrid",
    "implicit",
    "refresh",
    "device",
    "client_authentication",
    "include_custom_claims",
    "secrets_excluded",
    "conformity_fake_consents",
    "dev_mode",
    "rotating_system_secrets",
];

/// Whether the buffer looks like a Hydra config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("dsn:") && t.contains("urls:"))
        || t.contains("oauth2:") && t.contains("access_token_strategy")
        || t.contains("oidc.subject_identifiers")
        || t.contains("webfinger:")
        || t.contains("expose_internal_errors")
        || t.contains("min_refresh_entropy")
}

impl Hydra {
    /// Parse a hydra.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            named: 0,
        };
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            if indent == 0 {
                if let Some(colon) = s.find(':') {
                    c.sections += 1;
                    let key = &s[..colon];
                    if KEYS.contains(&key) {
                        c.named += 1;
                    }
                }
                continue;
            }
            if let Some(colon) = s.find(':') {
                c.settings += 1;
                let key = s[..colon].trim().trim_start_matches('-').trim();
                if KEYS.contains(&key) {
                    c.named += 1;
                }
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
            "dsn: memory\n",
            "serve:\n",
            "  public:\n",
            "    port: 4444\n",
            "    cors:\n",
            "      enabled: true\n",
            "  admin:\n",
            "    port: 4445\n",
            "urls:\n",
            "  self:\n",
            "    issuer: https://hydra.example.com/\n",
            "  login: https://login/\n",
            "  consent: https://consent/\n",
            "  logout: https://logout/\n",
            "secrets:\n",
            "  system:\n",
            "    - secret1\n",
            "  cookie:\n",
            "    - secret2\n",
            "oauth2:\n",
            "  expose_internal_errors: false\n",
            "  hash_secret_custom_claims: false\n",
            "ttl:\n",
            "  access_token: 1h\n",
            "  refresh_token: 720h\n",
            "oidc:\n",
            "  subject_identifiers:\n",
            "    supported_types: [public, pairwise]\n",
        );
        let c = Hydra::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 7);
        assert_eq!(c.settings, 19);
        assert!(c.named >= 20);
    }

    #[test]
    fn rejects_other() {
        assert!(Hydra::parse(b"foo: 1").is_none());
    }
}

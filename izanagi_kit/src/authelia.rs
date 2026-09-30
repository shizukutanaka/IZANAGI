//! Authelia `configuration.yml` census.
//!
//! Top-level sections `server`/`log`/`totp`/`duo_api`/
//! `authentication_backend`/`session`/`storage`/`notifier`/
//! `access_control`/`identity_providers`/`regulation`/`password_policy`/
//! `telemetry`/`webauthn`/`ntp`/`theme`/`default_2fa_method` +
//! `access_control.rules` entries (`- domain:`+`policy:`).
//!
//! ```rust
//! let a = "server:\n  address: 'tcp://0.0.0.0:9091'\naccess_control:\n  default_policy: deny\n  rules:\n    - domain: example.com\n      policy: bypass\n";
//! let c = izanagi_kit::authelia::Authelia::parse(a.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.rules, 1);
//! ```

/// Authelia configuration census.
#[derive(Debug, Clone)]
pub struct Authelia {
    /// Top-level `key:` section headers.
    pub sections: usize,
    /// Nested `key:`/`key: value` settings.
    pub settings: usize,
    /// `access_control.rules` `- domain:` entries.
    pub rules: usize,
    /// Recognised Authelia option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "server",
    "log",
    "totp",
    "duo_api",
    "authentication_backend",
    "session",
    "storage",
    "notifier",
    "access_control",
    "identity_providers",
    "regulation",
    "password_policy",
    "telemetry",
    "webauthn",
    "ntp",
    "theme",
    "default_2fa_method",
    "address",
    "endpoints",
    "asset_path",
    "level",
    "format",
    "keep_stdout",
    "issuer",
    "period",
    "digits",
    "algorithm",
    "secret_size",
    "skew",
    "hostname",
    "port",
    "timeout",
    "disable_usage_stats",
    "disable_require_consent",
    "disable_oidc_front_channel_logout",
    "api_url",
    "integration_key",
    "secret",
    "ldap",
    "file",
    "password",
    "iterations",
    "memory",
    "parallelism",
    "salt_length",
    "key_length",
    "variant",
    "argon2",
    "sha2crypt",
    "pbkdf2",
    "bcrypt",
    "scrypt",
    "implementation",
    "url",
    "base_dn",
    "username_attribute",
    "additional_users_dn",
    "users_filter",
    "additional_groups_dn",
    "groups_filter",
    "group_name_attribute",
    "mail_attribute",
    "display_name_attribute",
    "user",
    "private_key",
    "certificate_chain",
    "starttls",
    "tls",
    "server_name",
    "minimum_version",
    "cipher_suites",
    "skip_verify",
    "path",
    "search",
    "name",
    "secret_key",
    "expiration",
    "inactivity",
    "domain",
    "authelia_url",
    "same_site",
    "redis",
    "local",
    "postgresql",
    "mysql",
    "host",
    "username",
    "database",
    "maximum_retries",
    "encryption_key",
    "smtp",
    "filesystem",
    "sender",
    "subject",
    "identifier",
    "startup_check",
    "disable_startup_check",
    "default_policy",
    "networks",
    "rules",
    "domains",
    "resources",
    "query",
    "methods",
    "policy",
    "subject",
    "oidc",
    "lifespans",
    "access_token",
    "id_token",
    "refresh_token",
    "authorize_code",
    "clients",
    "client_name",
    "client_secret",
    "public",
    "authorization_policy",
    "claims_policies",
    "discovery",
    "jwks",
    "cors",
    "grant_types",
    "response_types",
    "response_modes",
    "scopes",
    "redirect_uris",
    "audience",
    "consent_mode",
    "pre_configured_consent_duration",
    "token_endpoint_auth_method",
    "token_endpoint_auth_signing_alg",
    "userinfo_signed_response_alg",
    "id_token_signed_response_alg",
    "access_token_signed_response_alg",
    "claims",
    "signed_response_alg",
    "pushed_authorizations",
    "refresh_token_always_rotate",
    "allow_multiple_auth_methods",
    "enforce_par",
    "enforce_pkce",
    "pkce_challenge_method",
    "enable_jwt_access_token_stateless_introspection",
    "discovery_signed_response_alg",
    "authorization_policies",
    "request_object_signing_alg",
    "max_retries",
    "find_time",
    "ban_time",
    "zxcvbn",
    "enabled",
    "min_score",
    "min_length",
    "max_length",
    "require_uppercase",
    "require_lowercase",
    "require_number",
    "require_special",
    "version",
    "max_desync",
    "disable_cors",
    "cookie",
    "remember_me",
    "jwk",
    "certificate",
    "kid",
    "syntax",
    "algorithms",
    "use_secrets",
    "jwt_secret",
    "watched_directory",
    "enforcement",
    "revocation",
];

/// Whether the buffer looks like an Authelia configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("authentication_backend")
        || t.contains("access_control:")
        || (t.contains("totp:") && t.contains("notifier:"))
        || t.contains("authelia_url")
        || t.contains("default_2fa_method")
        || (t.contains("session:") && t.contains("storage:") && t.contains("notifier:"))
}

impl Authelia {
    /// Parse a configuration.yml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            rules: 0,
            named: 0,
        };
        let mut in_rules = false;
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            if indent == 0 {
                in_rules = false;
                if let Some(colon) = s.find(':') {
                    c.sections += 1;
                    let key = &s[..colon];
                    if KEYS.contains(&key) {
                        c.named += 1;
                    }
                    if key == "access_control" {
                        in_rules = true;
                    }
                }
                continue;
            }
            if in_rules && s.starts_with("- domain") {
                c.rules += 1;
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
            "server:\n",
            "  address: 'tcp://0.0.0.0:9091'\n",
            "  endpoints:\n",
            "    authz:\n",
            "      forward-auth:\n",
            "        implementation: ForwardAuth\n",
            "log:\n",
            "  level: info\n",
            "totp:\n",
            "  issuer: example.com\n",
            "  period: 30\n",
            "authentication_backend:\n",
            "  ldap:\n",
            "    url: ldap://ldap\n",
            "    base_dn: dc=example,dc=com\n",
            "session:\n",
            "  secret: secret\n",
            "  redis:\n",
            "    host: redis\n",
            "storage:\n",
            "  local:\n",
            "    path: /db.sqlite3\n",
            "notifier:\n",
            "  smtp:\n",
            "    address: smtp://mail\n",
            "access_control:\n",
            "  default_policy: deny\n",
            "  rules:\n",
            "    - domain: secure.example.com\n",
            "      policy: two_factor\n",
            "    - domain: public.example.com\n",
            "      policy: bypass\n",
            "identity_providers:\n",
            "  oidc:\n",
            "    clients:\n",
            "      - client_id: app\n",
            "        client_secret: s\n",
            "        redirect_uris: [https://app/cb]\n",
        );
        let c = Authelia::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 9);
        assert_eq!(c.rules, 2);
        assert!(c.settings >= 20);
        assert!(c.named >= 15);
    }

    #[test]
    fn rejects_other() {
        assert!(Authelia::parse(b"foo: 1").is_none());
    }
}

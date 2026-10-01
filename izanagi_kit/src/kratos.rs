//! Ory Kratos `kratos.yaml` census.
//!
//! Sections `serve`/`dsn`/`identity`/`selfservice`/`courier`/`session`/
//! `secrets`/`log`/`tracing`/`hashers`/`organizations`/`feature_flags`.
//! `selfservice.flows` kinds (`registration`/`login`/`recovery`/
//! `verification`/`settings`/`logout`/`error`) and `methods`
//! (`password`/`oidc`/`totp`/`webauthn`/`lookup_secret`/`code`/`link`/
//! `profile`/`passkey`) counted separately.
//!
//! ```rust
//! let k = "dsn: memory\nidentity:\n  default_schema_id: default\nselfservice:\n  flows:\n    login:\n      ui_url: /login\n";
//! let c = izanagi_kit::kratos::Kratos::parse(k.as_bytes()).unwrap();
//! assert_eq!(c.flows, 1);
//! ```

/// Kratos config census.
#[derive(Debug, Clone)]
pub struct Kratos {
    /// Top-level `key:` section headers.
    pub sections: usize,
    /// Nested `key:`/`key: value` settings.
    pub settings: usize,
    /// `selfservice.flows` flow-kind keys.
    pub flows: usize,
    /// `selfservice.methods` method keys.
    pub methods: usize,
    /// Recognised Kratos option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "serve",
    "public",
    "admin",
    "dsn",
    "identity",
    "selfservice",
    "courier",
    "session",
    "secrets",
    "log",
    "tracing",
    "hashers",
    "organizations",
    "feature_flags",
    "base_url",
    "port",
    "host",
    "socket",
    "cors",
    "enabled",
    "allowed_origins",
    "default_schema_id",
    "schemas",
    "id",
    "url",
    "default_browser_return_url",
    "allowed_return_urls",
    "flows",
    "registration",
    "login",
    "recovery",
    "verification",
    "settings",
    "logout",
    "error",
    "after",
    "before",
    "default_redirect_url",
    "ui_url",
    "lifespan",
    "privileged_session_max_age",
    "required_aal",
    "use_continue_with_transitions",
    "use_hook",
    "methods",
    "password",
    "oidc",
    "totp",
    "webauthn",
    "lookup_secret",
    "code",
    "link",
    "profile",
    "passkey",
    "account",
    "config",
    "providers",
    "client_id",
    "client_secret",
    "mapper_url",
    "issuer_url",
    "scope",
    "label",
    "display_name",
    "rp",
    "passwordless",
    "ttl",
    "smtp",
    "connection_uri",
    "from_address",
    "from_name",
    "headers",
    "template_override",
    "subject",
    "body",
    "recovery_strategy",
    "notify_unknown_recipients",
    "whoami",
    "tokenizer",
    "encryption_key",
    "persistent_cookie",
    "same_site",
    "path",
    "domain",
    "cookie",
    "global",
    "tracing_provider",
    "level",
    "format",
    "leak_sensitive_values",
    "redaction",
    "argonaut",
    "argon2",
    "bcrypt",
    "memory",
    "iterations",
    "parallelism",
    "salt_length",
    "key_length",
    "expected_duration",
    "expected_deviation",
    "dedicated_memory",
    "min_password_length",
    "haveibeenpwned_host",
    "ignore_network_errors",
    "max_password_length",
    "identifier_similarity_check_enabled",
    "use_auto_schema_id",
    "use_sql_comment",
];

const FLOWS: &[&str] = &[
    "registration",
    "login",
    "recovery",
    "verification",
    "settings",
    "logout",
    "error",
];

const METHODS: &[&str] = &[
    "password",
    "oidc",
    "totp",
    "webauthn",
    "lookup_secret",
    "code",
    "link",
    "profile",
    "passkey",
    "account",
    "identifier_first",
];

/// Whether the buffer looks like a Kratos config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("selfservice:")
        || t.contains("default_schema_id")
        || (t.contains("courier:") && t.contains("identity:"))
        || t.contains("privileged_session_max_age")
        || t.contains("notify_unknown_recipients")
        || t.contains("kratos.dev")
}

impl Kratos {
    /// Parse a kratos.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            flows: 0,
            methods: 0,
            named: 0,
        };
        let mut path: Vec<(usize, String)> = Vec::new();
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
                c.sections += 1;
            } else {
                c.settings += 1;
            }
            while path.last().is_some_and(|(i, _)| *i >= indent) {
                path.pop();
            }
            let parents: Vec<&str> = path.iter().map(|(_, k)| k.as_str()).collect();
            if parents.contains(&"flows") && FLOWS.contains(&key) && parents.len() == 2 {
                c.flows += 1;
            } else if parents.contains(&"methods") && METHODS.contains(&key) && parents.len() == 2 {
                c.methods += 1;
            }
            if KEYS.contains(&key) {
                c.named += 1;
            }
            if s.ends_with(':') || s.matches(':').count() == 1 && s[colon + 1..].trim().is_empty() {
                path.push((indent, key.to_string()));
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
            "  public:\n",
            "    base_url: http://localhost:4433/\n",
            "  admin:\n",
            "    base_url: http://localhost:4434/\n",
            "dsn: postgres://kratos:secret@db/kratos\n",
            "identity:\n",
            "  default_schema_id: default\n",
            "  schemas:\n",
            "    - id: default\n",
            "      url: file:///etc/schemas/identity.schema.json\n",
            "selfservice:\n",
            "  default_browser_return_url: http://localhost:4455/\n",
            "  flows:\n",
            "    registration:\n",
            "      ui_url: http://localhost:4455/registration\n",
            "      lifespan: 10m\n",
            "    login:\n",
            "      ui_url: http://localhost:4455/login\n",
            "      lifespan: 10m\n",
            "    recovery:\n",
            "      enabled: true\n",
            "      ui_url: http://localhost:4455/recovery\n",
            "    verification:\n",
            "      enabled: true\n",
            "      ui_url: http://localhost:4455/verification\n",
            "  methods:\n",
            "    password:\n",
            "      enabled: true\n",
            "    oidc:\n",
            "      enabled: true\n",
            "      config:\n",
            "        providers:\n",
            "          - id: google\n",
            "            client_id: cid\n",
            "            client_secret: csec\n",
            "            issuer_url: https://accounts.google.com\n",
            "    totp:\n",
            "      enabled: true\n",
            "courier:\n",
            "  smtp:\n",
            "    connection_uri: smtps://user:pass@smtp\n",
            "session:\n",
            "  lifespan: 24h\n",
            "secrets:\n",
            "  cookie:\n",
            "    - PLEASE-CHANGE\n",
            "hashers:\n",
            "  algorithm: bcrypt\n",
            "  bcrypt:\n",
            "    cost: 12\n",
        );
        let c = Kratos::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 8);
        assert_eq!(c.flows, 4);
        assert_eq!(c.methods, 3);
        assert!(c.settings >= 30);
        assert!(c.named >= 25);
    }

    #[test]
    fn rejects_other() {
        assert!(Kratos::parse(b"foo: 1").is_none());
    }
}

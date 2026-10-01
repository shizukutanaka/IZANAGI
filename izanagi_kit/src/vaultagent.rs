//! Vault Agent config (`.hcl`) census.
//!
//! Vault Agent config is HCL: `vault { address = … }`,
//! `auto_auth { method "approle" { config = {…} } sink "file" {…} }`,
//! `cache { use_auto_auth_token = true }`,
//! `template { source/destination/perms/command }`,
//! `listener "tcp" { address/tls_disable }`,
//! plus `exit_after_auth`, `pid_file`, `template_config`,
//! `env_template`, `telemetry { … }`.
//!
//! ```rust
//! let c = izanagi_kit::vaultagent::Vaultagent::parse(b"exit_after_auth = false\nauto_auth {\n}\n").unwrap();
//! assert_eq!(c.blocks, 1);
//! ```

/// Vault Agent `.hcl` config census.
#[derive(Debug, Clone)]
pub struct Vaultagent {
    /// `name [`"label"`] {` blocks.
    pub blocks: usize,
    /// `key = value` assignments.
    pub assigns: usize,
    /// `#`/`//` comments.
    pub comments: usize,
}

const BLOCKS: &[&str] = &[
    "auto_auth",
    "method",
    "sink",
    "template",
    "listener",
    "cache",
    "vault",
    "config",
    "template_config",
    "telemetry",
    "env_template",
    "sink_config",
    "level",
    "mime",
    "static_secret_render_interval",
    "audit",
    "api_proxy",
];

/// Whether the buffer looks like Vault Agent config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("auto_auth")
        || t.contains("exit_after_auth")
        || t.contains("use_auto_auth_token")
        || t.contains("wrap_ttl")
        || (t.contains("template") && t.contains("sink")))
        && t.contains('=')
}

impl Vaultagent {
    /// Parse Vault Agent config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            blocks: 0,
            assigns: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if s.ends_with('{') {
                let head = s.split([' ', '"']).next().unwrap_or("");
                if BLOCKS.contains(&head) || !head.is_empty() {
                    c.blocks += 1;
                }
                continue;
            }
            if s == "}" || s == "]" || s == "};" {
                continue;
            }
            if s.contains('=') {
                c.assigns += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agent() {
        let b = concat!(
            "pid_file = \"./agent.pid\"\n",
            "exit_after_auth = false\n",
            "vault {\n",
            "  address = \"http://127.0.0.1:8200\"\n",
            "}\n",
            "auto_auth {\n",
            "  method \"approle\" {\n",
            "    config = {\n",
            "      role_id_file_path = \"/etc/roleid\"\n",
            "      secret_id_file_path = \"/etc/secretid\"\n",
            "      remove_jwt_after_reading = false\n",
            "    }\n",
            "  }\n",
            "  sink \"file\" {\n",
            "    config = {\n",
            "      path = \"/tmp/token\"\n",
            "    }\n",
            "  }\n",
            "}\n",
            "template {\n",
            "  source = \"/tpl/app.env\"\n",
            "  destination = \"/etc/app.env\"\n",
            "}\n",
            "// comment\n",
        );
        let c = Vaultagent::parse(b.as_bytes()).unwrap();
        assert_eq!(c.blocks, 7);
        assert_eq!(c.assigns, 9);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Vaultagent::parse(b"foo = 1\n").is_none());
    }
}

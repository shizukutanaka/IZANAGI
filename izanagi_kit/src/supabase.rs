//! Supabase `config.toml` census (`supabase/config.toml`).
//!
//! Sections: `[api]` (`enabled`/`port`/`schemas`/`extra_search_path`/
//! `max_rows`/`tls`), `[db]` (`port`/`shadow_port`/`major_version`),
//! `[studio]` (`enabled`/`port`/`api_url`), `[auth]` (`enabled`/`site_url`/
//! `additional_redirect_urls`/`jwt_expiry`/`enable_signup`/`email`/
//! `sms`/`external`/`mfa`/`sso`/`rate_limit`/`captcha`), `[storage]`
//! (`file_size_limit`/`enabled`/`image_transformation`), `[edge_runtime]`
//! (`enabled`/`policy`/`deno_version`/`inspector_port`), `[realtime]`,
//! `[analytics]` (`enabled`/`port`/`vector_port`/`gcp_project_id`/
//! `gcp_project_number`/`gcp_jwt_path`), `[functions.<name>]`
//! (`enabled`/`verify_jwt`/`import_map`/`entrypoint`), `[experimental]`
//! (`orioledb_version`), `[dashboard]`/`[inbucket]`/`[imgproxy]`/
//! `[local]`/`[linked]`/`[migrations]`/`[seed]`/`[hooks]`/`[pooler]`/
//! `[db.pooler]` (`enabled`/`port`/`pool_mode`/`default_pool_size`/
//! `max_client_conn`), `[auth.sms.twilio]`/`[auth.external.apple]`/`…`.
//!
//! ```rust
//! let k = b"[api]\nenabled = true\nport = 54321\n[db]\nport = 54322\nmajor_version = 15\n[auth]\nsite_url = \"http://x\"\nenable_signup = true\n";
//! assert!(izanagi_kit::supabase::detect(k));
//! ```

/// supabase config.toml census.
#[derive(Debug, Clone)]
pub struct Supabase {
    /// `[x]` section headers.
    pub sections: usize,
    /// `key = value` assignments.
    pub settings: usize,
    /// recognised sections/keys present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "api",
    "db",
    "studio",
    "auth",
    "storage",
    "edge_runtime",
    "realtime",
    "analytics",
    "functions",
    "experimental",
    "dashboard",
    "inbucket",
    "imgproxy",
    "local",
    "linked",
    "migrations",
    "seed",
    "hooks",
    "pooler",
    "db.pooler",
    "auth.sms",
    "auth.external",
    "auth.email",
    "auth.mfa",
    "auth.sso",
    "auth.rate_limit",
    "auth.captcha",
    "storage.image_transformation",
    "storage.vector",
    "edge_runtime.secrets",
    "notifications",
    "web3",
];

const STRONG: &[&str] = &[
    "site_url",
    "additional_redirect_urls",
    "jwt_expiry",
    "enable_signup",
    "enable_refresh_token_rotation",
    "refresh_token_reuse_interval",
    "shadow_port",
    "major_version",
    "extra_search_path",
    "max_rows",
    "file_size_limit",
    "deno_version",
    "inspector_port",
    "verify_jwt",
    "import_map",
    "entrypoint",
    "orioledb_version",
    "vector_port",
    "gcp_project_id",
    "gcp_project_number",
    "gcp_jwt_path",
    "pool_mode",
    "default_pool_size",
    "max_client_conn",
    "api_url",
    "bucket",
    "apikey",
    "anon_key",
    "service_role_key",
    "inbucket_port",
    "smtp_port",
    "pop3_port",
    "adminer",
    "meta_url",
    "encryption_key",
    "pg_meta",
    "tl_api_key",
    "seed_paths",
    "keep_data_on_branch",
    "sql_paths",
    "mfa",
    "sso",
    "captcha",
    "twilio",
    "messagebird",
    "vonage",
    "textlocal",
    "mailer_subjects",
    "mailer_templates",
    "template_paths",
    "mailer_secure_email_change_enabled",
    "mailer_autoconfirm",
    "double_confirm_changes",
    "flow_type",
    "pkce",
    "nonce_enabled",
    "nonce_expiry",
    "pool_size",
    "connect_timeout",
    "resend",
];

const WEAK: &[&str] = &[
    "enabled",
    "port",
    "host",
    "schemas",
    "url",
    "secret",
    "username",
    "password",
    "tls",
    "proxy",
    "email",
    "sms",
    "external",
    "rate_limit",
    "expiry",
    "path",
    "client_id",
    "client_secret",
    "redirect_url",
    "name",
    "version",
    "image",
    "size",
    "policy",
    "enabled_if_configured",
    "file_name",
];

fn section(line: &str) -> Option<&str> {
    let s = line.trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    let inner = inner.trim().trim_matches('"');
    if inner.is_empty() {
        None
    } else {
        Some(inner)
    }
}

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with('[') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"');
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a Supabase `config.toml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `[edge_runtime]`/`[studio]`/`[pooler]`/`site_url`/`verify_jwt`
    // are Supabase-exclusive; `[api]`/`[db]`/`[auth]` only count
    // alongside exclusive markers.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(sec) = section(line) {
            if SECTIONS.iter().any(|s| {
                sec == *s
                    || sec.starts_with("functions.")
                    || sec.starts_with("db.")
                    || sec.starts_with("auth.")
                    || sec.starts_with("storage.")
                    || sec.starts_with("edge_runtime.")
            }) {
                strong += 1;
            }
        } else if let Some(k) = assign_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 3)
}

impl Supabase {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if section(line).is_some() {
                c.sections += 1;
                c.keys += 1;
            } else if let Some(k) = assign_key(line) {
                c.settings += 1;
                if STRONG.contains(&k) || WEAK.contains(&k) {
                    c.keys += 1;
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
    fn detects() {
        let b = b"[api]\nenabled = true\nport = 54321\n[db]\nport = 54322\nmajor_version = 15\n[auth]\nsite_url = \"http://x\"\nenable_signup = true\n";
        assert!(detect(b));
        let c = Supabase::parse(b).unwrap();
        assert!(c.sections >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[server]\nport = 1\nhost = \"x\"\n"));
        assert!(!detect(
            b"# [api]\n# enabled = true\n# [db]\n# port = 2\n[x]\ny = 1\n"
        ));
    }
}

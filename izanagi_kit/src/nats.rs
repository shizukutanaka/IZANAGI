//! NATS server configuration (`nats-server.conf`, HOCON-ish) parser.
//!
//! Detects `key: value` / `key = value` entries plus curly-brace sections
//! (`cluster {}`, `jetstream {}`, `accounts {}`, `gateway {}`, `websocket {}`,
//! `leafnodes {}`, `mqtt {}`, `tls {}` …) and counts section blocks,
//! key-value pairs, and TLS/auth-related keys.
//!
//! ```
//! let b = b"port: 4222\nserver_name: n1\njetstream {\n  store_dir: /data\n}\ncluster {\n  name: prod\n  port: 6222\n}\n";
//! assert!(izanagi_kit::nats::detect(b));
//! let c = izanagi_kit::nats::Nats::parse(b).unwrap();
//! assert_eq!(c.blocks, 2);
//! assert!(c.kv_pairs >= 4);
//! ```

/// Parsed NATS server config summary.
#[derive(Debug, Clone)]
pub struct Nats {
    /// `key: value`/`key = value` assignments (comments excluded).
    pub kv_pairs: usize,
    /// Section blocks (`cluster`/`jetstream`/`accounts`/`gateway`/`websocket`/`leafnodes`/`mqtt`/`tls`/`resolver`/`authorization`/`operator`-style `{` lines).
    pub blocks: usize,
    /// TLS keys (`tls`/`cert_file`/`key_file`/`ca_file`/`verify`/`insecure`/`cipher_suites`).
    pub tls_keys: usize,
    /// Auth keys (`authorization`/`users`/`user`/`pass`/`token`/`nkey`/`jwt`/`creds`/`resolver`/`system_account`).
    pub auth_keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

/// Section/block names that open with `{`.
const BLOCKS: &[&str] = &[
    "cluster",
    "jetstream",
    "accounts",
    "gateway",
    "websocket",
    "leafnodes",
    "mqtt",
    "tls",
    "resolver",
    "authorization",
    "lame_duck",
    "debug_and_trace",
];

/// TLS keys (word match).
const TLS_KEYS: &[&str] = &[
    "tls",
    "cert_file",
    "key_file",
    "ca_file",
    "ca_certs",
    "verify",
    "insecure",
    "cipher_suites",
    "curve_preferences",
    "verify_and_map",
    "timeout",
    "pinned_certs",
];

/// Auth keys (word match).
const AUTH_KEYS: &[&str] = &[
    "authorization",
    "users",
    "user",
    "pass",
    "password",
    "token",
    "nkey",
    "jwt",
    "creds",
    "resolver",
    "system_account",
    "no_auth_user",
    "auth_timeout",
];

fn first_word(tr: &str) -> &str {
    tr.split(|c: char| c.is_ascii_whitespace() || c == ':' || c == '=' || c == '{')
        .next()
        .unwrap_or("")
}

/// Detect a `nats-server.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let lc = t.to_ascii_lowercase();
    if lc.contains("jetstream") || lc.contains("store_dir") {
        return true;
    }
    lc.contains("server_name:")
        || lc.contains("server_name ")
        || (lc.contains("cluster") && (lc.contains("gateway") || lc.contains("accounts")))
        || (lc.contains("port:")
            && (lc.contains("nats")
                || lc.contains("websocket")
                || lc.contains("gateway")
                || lc.contains("leafnodes")
                || lc.contains("mqtt")
                || lc.contains("accounts")))
}

impl Nats {
    /// Count keys/blocks in a NATS server config. Returns `None` when the
    /// input does not match the format.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kv_pairs: 0,
            blocks: 0,
            tls_keys: 0,
            auth_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() || tr == "}" || tr == "{" {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
                continue;
            }
            let w = first_word(tr);
            if w.is_empty() {
                continue;
            }
            let has_body = tr.contains('{') || tr.contains('[');
            if BLOCKS.contains(&w) && has_body {
                c.blocks += 1;
                continue;
            }
            if tr.contains(':') || tr.contains('=') {
                c.kv_pairs += 1;
                if TLS_KEYS.contains(&w) {
                    c.tls_keys += 1;
                }
                if AUTH_KEYS.contains(&w) {
                    c.auth_keys += 1;
                }
            } else if has_body {
                c.blocks += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# nats\nport: 4222\nserver_name: n1\nhttp_port: 8222\nmax_payload: 8MB\njetstream {\n  store_dir: /data\n  max_mem: 4G\n  max_file: 10G\n}\ncluster {\n  name: prod\n  port: 6222\n  routes: [\n    nats-route://r1:6222\n  ]\n}\naccounts {\n  SYS: {\n    users: [\n      {user: sys, password: sys}\n    ]\n  }\n}\ntls {\n  cert_file: /c.pem\n  key_file: /k.pem\n}\n";
        assert!(detect(b));
        let c = Nats::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert_eq!(c.blocks, 4);
        assert_eq!(c.tls_keys, 2);
        assert!(c.kv_pairs >= 8);
        assert!(c.auth_keys >= 1);
    }

    #[test]
    fn websocket_detect() {
        let b = b"port: 4222\nwebsocket {\n  port: 8080\n  no_tls: true\n}\n";
        assert!(detect(b));
        let c = Nats::parse(b).unwrap();
        assert_eq!(c.blocks, 1);
    }

    #[test]
    fn rejects_json() {
        assert!(!detect(br#"{"a":1}"#));
        assert!(Nats::parse(b"name value\n").is_none());
    }
}

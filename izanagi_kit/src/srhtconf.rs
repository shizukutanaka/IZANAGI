//! sourcehut `config.ini` parser.
//!
//! Detects sourcehut (sr.ht) configuration by `[meta]`/`[web]`/`[mail]`
//! sections plus `srv.origin`/`oauth-client-id`/`connection-string`/
//! `*-keys` style keys and per-service `[*.sr.ht]` sections, and counts
//! structure.
//!
//! ```
//! let b = b"[meta]\nsrv.origin = https://example.org\nprotocol = https\n[web]\nroot = https://example.org\n[mail]\npgp-privkey = /etc/sr.ht/pgp\n[git.sr.ht]\noauth-client-id = abc\n";
//! assert!(izanagi_kit::srhtconf::detect(b));
//! let c = izanagi_kit::srhtconf::Srht::parse(b).unwrap();
//! assert!(c.section_keys >= 4);
//! ```

/// Parsed config.ini summary.
#[derive(Debug, Clone)]
pub struct Srht {
    /// Recognized section/key occurrences.
    pub keys: usize,
    /// `[section]` lines (core `[meta]`/`[web]`/`[mail]`/`[objects]` + `[<service>.sr.ht]` service sections).
    pub section_keys: usize,
    /// Service sections `[*.sr.ht]`/`[*.srht]`.
    pub service_keys: usize,
    /// Known option keys (`srv.origin`/`oauth-client-id`/`oauth-client-secret`/`connection-string`/`pgp-*`/`migrate-on-upgrade`/...).
    pub option_keys: usize,
    /// `key = value` assignment lines.
    pub assignments: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

/// Known option keys.
const OPTION_KEYS: &[&str] = &[
    "srv.origin",
    "protocol",
    "port",
    "debug",
    "connection-string",
    "migrate-on-upgrade",
    "service-name",
    "oauth-client-id",
    "oauth-client-secret",
    "webhooks.private-key",
    "pgp-privkey",
    "pgp-pubkey",
    "pgp-key-id",
    "smtp-host",
    "smtp-port",
    "smtp-user",
    "smtp-password",
    "smtp-encryption",
    "root",
    "static-key",
    "object-storage-backend",
    "s3-upstream",
    "s3-access-key",
    "s3-secret-key",
    "s3-bucket",
    "s3-prefix",
    "redis-host",
    "redis-db",
    "builds.sr.ht::worker",
    "git.sr.ht::dispatch",
    "git.sr.ht::repos",
    "post-update-script",
    "origin",
    "shell",
    "authorized-keys-path",
    "ip-whitelist",
    "ssl-crt-path",
    "ssl-key-path",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["srv.origin", ".sr.ht]", "oauth-client-"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "srv.origin",
    ".sr.ht]",
    "oauth-client-id",
    "oauth-client-secret",
    "connection-string",
    "pgp-privkey",
    "migrate-on-upgrade",
    "post-update-script",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

fn is_service_section(tr: &str) -> bool {
    tr.starts_with('[') && tr.ends_with("sr.ht]")
}

/// Detect a sourcehut config.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Srht {
    /// Count categories in a config.ini. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            section_keys: 0,
            service_keys: 0,
            option_keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with(';') || tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('[') && tr.ends_with(']') {
                c.section_keys += 1;
                if is_service_section(tr) {
                    c.service_keys += 1;
                }
                continue;
            }
            if tr.contains('=') {
                c.assignments += 1;
            }
        }
        for k in OPTION_KEYS {
            c.option_keys += t.matches(k).count();
        }
        c.keys = c.section_keys + c.option_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"; sr.ht config\n[meta]\nsrv.origin = https://example.org\nprotocol = https\n[web]\nroot = https://example.org\n[mail]\npgp-privkey = /etc/sr.ht/pgp\n[git.sr.ht]\noauth-client-id = abc\noauth-client-secret = xyz\n[todo.sr.ht]\noauth-client-id = def\n";
        assert!(detect(b));
        let c = Srht::parse(b).unwrap();
        assert_eq!(c.section_keys, 5);
        assert_eq!(c.service_keys, 2);
        assert!(c.option_keys >= 4);
        assert_eq!(c.comments, 1);
        assert!(c.keys >= 9);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[section]\nkey = value\n"));
        assert!(Srht::parse(b"a = b\n").is_none());
    }
}

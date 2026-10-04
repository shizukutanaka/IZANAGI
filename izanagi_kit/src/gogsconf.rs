//! Gogs `app.ini` parser.
//!
//! Detects Gogs configuration by its `[server]`/`[database]`/`[repository]`
//! INI sections plus `RUN_USER`/`ROOT_URL`/`INSTALL_LOCK`/`ROOT =` style
//! keys (the Gogs ancestor of the Gitea config), and counts structure.
//!
//! ```
//! let b = b"APP_NAME = Gogs\nRUN_USER = git\nRUN_MODE = prod\n[server]\nDOMAIN = git.example.com\nROOT_URL = https://git.example.com/\nHTTP_PORT = 3000\n[repository]\nROOT = /home/git/gogs-repositories\n[security]\nINSTALL_LOCK = true\n";
//! assert!(izanagi_kit::gogsconf::detect(b));
//! let c = izanagi_kit::gogsconf::Gogs::parse(b).unwrap();
//! assert!(c.section_keys >= 3);
//! ```

/// Parsed app.ini summary.
#[derive(Debug, Clone)]
pub struct Gogs {
    /// Recognized section/key occurrences.
    pub keys: usize,
    /// `[section]` lines.
    pub section_keys: usize,
    /// Known option keys (`RUN_USER`/`ROOT_URL`/`INSTALL_LOCK`/`ROOT`/`SECRET_KEY`/...).
    pub option_keys: usize,
    /// `key = value` assignment lines.
    pub assignments: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

/// Known option keys.
const OPTION_KEYS: &[&str] = &[
    "APP_NAME",
    "RUN_USER",
    "RUN_MODE",
    "DOMAIN",
    "ROOT_URL",
    "HTTP_ADDR",
    "HTTP_PORT",
    "OFFLINE_MODE",
    "CERT_FILE",
    "KEY_FILE",
    "STATIC_ROOT_PATH",
    "SSH_PORT",
    "DB_TYPE",
    "HOST",
    "NAME",
    "USER",
    "PASSWD",
    "PATH",
    "SSL_MODE",
    "ROOT",
    "SECRET_KEY",
    "INSTALL_LOCK",
    "DISABLE_REGISTRATION",
    "REQUIRE_SIGNIN_VIEW",
    "MODE",
    "LEVEL",
    "PROVIDER",
    "AVATAR_UPLOAD_PATH",
    "ENABLE_CAPTCHA",
    "ACTIVE_CODE_LIVE_MINUTES",
    "RESET_PASSWD_CODE_LIVE_MINUTES",
    "REGISTER_EMAIL_CONFIRM",
    "ENABLE_NOTIFY_MAIL",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["RUN_USER", "ROOT_URL", "INSTALL_LOCK", "ROOT ="];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "[server]",
    "[database]",
    "[repository]",
    "RUN_USER",
    "RUN_MODE",
    "ROOT_URL",
    "INSTALL_LOCK",
    "APP_NAME",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Gogs app.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Gogs {
    /// Count categories in an app.ini. Returns `None` when the input does
    /// not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            section_keys: 0,
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
        let b = b"APP_NAME = Gogs\nRUN_USER = git\nRUN_MODE = prod\n[server]\nDOMAIN = git.example.com\nROOT_URL = https://git.example.com/\nHTTP_PORT = 3000\n[repository]\nROOT = /home/git/gogs-repositories\n[security]\nINSTALL_LOCK = true\n";
        assert!(detect(b));
        let c = Gogs::parse(b).unwrap();
        assert_eq!(c.section_keys, 3);
        assert!(c.option_keys >= 7);
        assert!(c.assignments >= 8);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[section]\nkey = value\nother = 1\n"));
        assert!(Gogs::parse(b"a = b\n").is_none());
    }
}

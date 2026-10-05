//! Gitea `app.ini` parser.
//!
//! Detects Gitea configuration by its `[server]`/`[database]`/`[repository]`
//! INI sections plus `RUN_MODE`/`ROOT_URL`/`APP_NAME`/`INSTALL_LOCK` style
//! keys, and counts structure.
//!
//! ```
//! let b = b"[server]\nRUN_MODE = prod\nDOMAIN = git.example.com\nROOT_URL = https://git.example.com/\nHTTP_PORT = 3000\n[database]\nDB_TYPE = sqlite3\nPATH = data/gitea.db\n[security]\nINSTALL_LOCK = true\n";
//! assert!(izanagi_kit::giteaapp::detect(b));
//! let c = izanagi_kit::giteaapp::Gitea::parse(b).unwrap();
//! assert!(c.section_keys >= 3);
//! assert!(c.option_keys >= 3);
//! ```

/// Parsed app.ini summary.
#[derive(Debug, Clone)]
pub struct Gitea {
    /// Recognized section/key occurrences.
    pub keys: usize,
    /// `[section]` lines.
    pub section_keys: usize,
    /// Known option keys (`RUN_MODE`/`ROOT_URL`/`APP_NAME`/`DB_TYPE`/`INSTALL_LOCK`/...).
    pub option_keys: usize,
    /// `key = value` assignment lines.
    pub assignments: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

/// Known option keys.
const OPTION_KEYS: &[&str] = &[
    "APP_NAME",
    "RUN_MODE",
    "RUN_USER",
    "WORK_PATH",
    "DOMAIN",
    "ROOT_URL",
    "HTTP_ADDR",
    "HTTP_PORT",
    "SSH_PORT",
    "SSH_DOMAIN",
    "START_SSH_SERVER",
    "OFFLINE_MODE",
    "CERT_FILE",
    "KEY_FILE",
    "STATIC_ROOT_PATH",
    "APP_DATA_PATH",
    "DB_TYPE",
    "HOST",
    "NAME",
    "USER",
    "PASSWD",
    "PATH",
    "SECRET_KEY",
    "INTERNAL_TOKEN",
    "INSTALL_LOCK",
    "DISABLE_REGISTRATION",
    "REQUIRE_SIGNIN_VIEW",
    "LFS_START_SERVER",
    "MODE",
    "LEVEL",
    "ROOT",
    "ENABLED",
    "DISABLE_GIT_HOOKS",
    "DEFAULT_BRANCH",
    "DEFAULT_PRIVATE",
    "ALLOW_ONLY_INTERNAL_REGISTRATION",
    "SHOW_REGISTRATION_BUTTON",
    "PROVIDER",
    "AVATAR_UPLOAD_PATH",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &[
    "RUN_MODE",
    "ROOT_URL",
    "APP_NAME",
    "INSTALL_LOCK",
    "WORK_PATH",
    "GITEA",
];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "[server]",
    "[database]",
    "[repository]",
    "RUN_MODE",
    "ROOT_URL",
    "APP_NAME",
    "INSTALL_LOCK",
    "DB_TYPE",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Gitea app.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Gitea {
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
        let b = b"APP_NAME = Gitea\nRUN_USER = git\nRUN_MODE = prod\n[server]\nDOMAIN = git.example.com\nROOT_URL = https://git.example.com/\nHTTP_PORT = 3000\n[database]\nDB_TYPE = sqlite3\nPATH = data/gitea.db\n[security]\nINSTALL_LOCK = true\nSECRET_KEY = xyz\n";
        assert!(detect(b));
        let c = Gitea::parse(b).unwrap();
        assert_eq!(c.section_keys, 3);
        assert!(c.option_keys >= 7);
        assert!(c.assignments >= 9);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[section]\nkey = value\nother = 1\n"));
        assert!(Gitea::parse(b"a = b\n").is_none());
    }
}

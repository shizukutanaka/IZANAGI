//! Phabricator `local.json` / config JSON parser.
//!
//! Detects Phabricator configuration JSON by `"phabricator.*"`/`"metamta.*"`/
//! `"diffusion.*"`/`"phd.*"`/`"cluster.*"`/`"mysql.*"`/`"security.*"` key
//! prefixes, and counts structure.
//!
//! ```
//! let b = b"{\n  \"phabricator.base-uri\": \"https://phab.example.com\",\n  \"mysql.host\": \"localhost\",\n  \"mysql.user\": \"root\",\n  \"metamta.default-address\": \"noreply@example.com\",\n  \"diffusion.ssh-user\": \"git\",\n  \"phd.user\": \"phd\",\n  \"cluster.addresses\": []\n}\n";
//! assert!(izanagi_kit::phabricatorconf::detect(b));
//! let c = izanagi_kit::phabricatorconf::Phab::parse(b).unwrap();
//! assert!(c.prefix_keys >= 4);
//! ```

/// Parsed local.json summary.
#[derive(Debug, Clone)]
pub struct Phab {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `phabricator.*` core keys (`phabricator.base-uri`/`phabricator.serious-business`/`phabricator.show-prototype-ribbon`/`phabricator.timezone`/`phabricator.production-uri`/`phabricator.allowed-uris`/...).
    pub core_keys: usize,
    /// Service-prefix keys (`"<prefix>.` family: metamta/diffusion/phd/cluster/drydock/notification/repository/files/search/auth/policy/security/storage/celerity/remarkup/pygments/translation/user/account/maniphest/differential/audit/almanac/harbormaster/environment/darkconsole/config/bin).
    pub prefix_keys: usize,
    /// `mysql.*` database keys.
    pub mysql_keys: usize,
    /// `"key": value` JSON pairs.
    pub assignments: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

/// Core `phabricator.*` keys.
const CORE_KEYS: &[&str] = &[
    "\"phabricator.base-uri\"",
    "\"phabricator.production-uri\"",
    "\"phabricator.allowed-uris\"",
    "\"phabricator.serious-business\"",
    "\"phabricator.show-prototype-ribbon\"",
    "\"phabricator.timezone\"",
    "\"phabricator.cookie-prefix\"",
    "\"phabricator.show-beta-warning\"",
    "\"phabricator.uninstalled-applications\"",
    "\"phabricator.application-options\"",
    "\"phabricator.custom-css\"",
    "\"phabricator.user-custom-field\"",
];

/// Service key prefixes.
const PREFIX_KEYS: &[&str] = &[
    "\"metamta.",
    "\"diffusion.",
    "\"phd.",
    "\"cluster.",
    "\"drydock.",
    "\"notification.",
    "\"repository.",
    "\"files.",
    "\"search.",
    "\"auth.",
    "\"policy.",
    "\"security.",
    "\"storage.",
    "\"celerity.",
    "\"remarkup.",
    "\"pygments.",
    "\"translation.",
    "\"user.",
    "\"account.",
    "\"maniphest.",
    "\"differential.",
    "\"audit.",
    "\"almanac.",
    "\"harbormaster.",
    "\"environment.",
    "\"darkconsole.",
    "\"config.",
    "\"bin.",
];

/// MySQL keys.
const MYSQL_KEYS: &[&str] = &[
    "\"mysql.host\"",
    "\"mysql.user\"",
    "\"mysql.pass\"",
    "\"mysql.port\"",
    "\"mysql.implementation\"",
    "\"mysql.configuration-provider\"",
];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "\"phabricator.",
    "\"metamta.",
    "\"diffusion.",
    "\"phd.",
    "\"cluster.",
    "\"mysql.",
    "\"security.alternate-file-domain\"",
    "\"storage.default-namespace\"",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Phabricator config JSON.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Phab {
    /// Count categories in a config JSON. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            core_keys: 0,
            prefix_keys: 0,
            mysql_keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.contains("\":") {
                c.assignments += 1;
            }
        }
        for k in CORE_KEYS {
            c.core_keys += t.matches(k).count();
        }
        for k in PREFIX_KEYS {
            c.prefix_keys += t.matches(k).count();
        }
        for k in MYSQL_KEYS {
            c.mysql_keys += t.matches(k).count();
        }
        c.keys = c.core_keys + c.prefix_keys + c.mysql_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"{\n  \"phabricator.base-uri\": \"https://phab.example.com\",\n  \"phabricator.show-prototype-ribbon\": false,\n  \"mysql.host\": \"localhost\",\n  \"mysql.user\": \"root\",\n  \"mysql.pass\": \"secret\",\n  \"metamta.default-address\": \"noreply@example.com\",\n  \"metamta.domain\": \"example.com\",\n  \"diffusion.ssh-user\": \"git\",\n  \"phd.user\": \"phd\",\n  \"cluster.addresses\": [\"10.0.0.1\"]\n}\n";
        assert!(detect(b));
        let c = Phab::parse(b).unwrap();
        assert!(c.core_keys >= 2);
        assert!(c.prefix_keys >= 4);
        assert_eq!(c.mysql_keys, 3);
        assert!(c.assignments >= 9);
        assert!(c.keys >= 9);
    }

    #[test]
    fn rejects_json() {
        assert!(!detect(b"{\"a\": 1, \"b\": 2}\n"));
        assert!(Phab::parse(b"{}").is_none());
    }
}

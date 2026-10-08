//! Census of a Duplicacy `.duplicacy/preferences` file.
//!
//! A JSON array of storage objects:
//! `[{"name":"default","id":"…","repository":"…","storage":"s3://…",
//!    "encrypted":true,"no_backup":false,"no_restore":false,
//!    "no_save_password":false,"nobackup_file":"","keys":{…},
//!    "filters":"","exclude_by_attribute":false}]`.
//! Counts storage entries, encrypted storages, boolean flags, keys.
//!
//! ```rust
//! let c = izanagi_kit::duplicacy::Duplicacy::parse(
//!     b"[{\"name\":\"default\",\"id\":\"x\",\"repository\":\"/r\",\
//!        \"storage\":\"local\",\"encrypted\":true}]",
//! ).unwrap();
//! assert_eq!(c.storages, 1);
//! assert_eq!(c.encrypted, 1);
//! ```
#![forbid(unsafe_code)]

use crate::textutil::strip_bom;
/// duplicacy preferences census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duplicacy {
    /// Objects in the preferences array (named storages).
    pub storages: usize,
    /// `"name"` keys.
    pub names: usize,
    /// `"encrypted": true` storages.
    pub encrypted: usize,
    /// `no_*`/`exclude_*` boolean flag keys.
    pub flags: usize,
    /// Other `"key":` keys.
    pub keys: usize,
}

/// Flag keys counted separately.
const FLAGS: &[&str] = &[
    "no_backup",
    "no_restore",
    "no_save_password",
    "exclude_by_attribute",
    "dropbox_no_redirect",
];

/// True if `b` looks like .duplicacy/preferences.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.trim_start().starts_with('[')
        && t.contains("\"storage\"")
        && (t.contains("\"repository\"") || t.contains("\"name\""))
}

impl Duplicacy {
    /// Parse a preferences file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        if !t.trim_start().starts_with('[') {
            return None;
        }
        let mut c = Self {
            storages: t.matches("\"repository\"").count(),
            names: t.matches("\"name\"").count(),
            encrypted: t.matches("\"encrypted\":true").count()
                + t.matches("\"encrypted\": true").count(),
            flags: 0,
            keys: 0,
        };
        let mut rest = t;
        while let Some(q) = rest.find('"') {
            let after = &rest[q + 1..];
            let Some(end) = after.find('"') else {
                break;
            };
            let key = &after[..end];
            if after[end + 1..].trim_start().starts_with(':') {
                if FLAGS.contains(&key) {
                    c.flags += 1;
                } else if key != "name" && key != "repository" && key != "encrypted" {
                    c.keys += 1;
                }
            }
            rest = &after[end..];
        }
        if c.storages == 0 && c.names == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        concat!(
            "[{\"name\":\"default\",\"id\":\"x\",\"repository\":\"/r\",",
            "\"storage\":\"local\",\"encrypted\":true,",
            "\"no_backup\":false,\"filters\":\"\"}]",
        )
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Duplicacy::parse(b.as_bytes()).unwrap();
        assert_eq!(c.storages, 1);
        assert_eq!(c.names, 1);
        assert_eq!(c.encrypted, 1);
        assert_eq!(c.flags, 1);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"a\":1}"));
        assert!(Duplicacy::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

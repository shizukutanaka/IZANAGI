//! Census of a containers `storage.conf` file.
//!
//! TOML `[storage]` root plus `[storage.options.<driver>]` driver
//! option tables: `driver`, `graphroot`, `runroot`, `rootless_storage_path`,
//! `mount_program`, `size`, `remap-uids`/`remap-gids`.
//! Counts sections, entries, driver option tables and comments.
//!
//! ```rust
//! let c = izanagi_kit::storageconf::StorageConf::parse(
//!     b"[storage]\ndriver = \"overlay\"\ngraphroot = \"/var/lib/containers/storage\"\n\
//!       [storage.options.overlay]\nmount_program = \"/usr/bin/fuse-overlayfs\"\n",
//! ).unwrap();
//! assert_eq!(c.driver_options, 1);
//! ```
#![forbid(unsafe_code)]

/// storage.conf census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageConf {
    /// Total `[table]` headers.
    pub sections: usize,
    /// `key = value` lines.
    pub entries: usize,
    /// `[storage.options.<driver>]` subtables.
    pub driver_options: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Known storage keys for `detect`.
const KEYS: &[&str] = &[
    "graphroot",
    "runroot",
    "rootless_storage_path",
    "mount_program",
    "additionalimagestores",
    "remap-uids",
    "remap-gids",
];

/// True if `b` looks like storage.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let storage_section = t
        .lines()
        .map(str::trim)
        .any(|l| l == "[storage]" || l.starts_with("[storage.options"));
    let keys = KEYS.iter().filter(|k| t.contains(**k)).count();
    storage_section && (t.contains("driver") || keys >= 1) || keys >= 3
}

impl StorageConf {
    /// Parse a storage.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut sections = 0usize;
        let mut entries = 0usize;
        let mut driver_options = 0usize;
        let mut comments = 0usize;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') || l.starts_with(';') {
                comments += 1;
            } else if l.starts_with('[') && l.ends_with(']') {
                sections += 1;
                if l.starts_with("[storage.options.") {
                    driver_options += 1;
                }
            } else if l.contains('=') {
                entries += 1;
            }
        }
        if entries == 0 {
            return None;
        }
        Some(Self {
            sections,
            entries,
            driver_options,
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# storage\n",
            "[storage]\n",
            "driver = \"overlay\"\n",
            "graphroot = \"/var/lib/containers/storage\"\n",
            "runroot = \"/run/containers/storage\"\n",
            "[storage.options.overlay]\n",
            "mount_program = \"/usr/bin/fuse-overlayfs\"\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = StorageConf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.entries, 4);
        assert_eq!(c.driver_options, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[main]\nport = 1\n"));
        assert!(StorageConf::parse(b"# none\n").is_none());
    }
}

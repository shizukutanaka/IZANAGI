//! borgmatic configuration YAML census.
//!
//! borgmatic configs use top-level sections (`source_directories:`,
//! `repositories:`, `storage:`, `retention:`, `consistency:`, `output:`,
//! `hooks:`) with `repositories:` entries (`- path:` / `path:`+`label:`),
//! `exclude_patterns:`/`exclude_from`, `keep_*`/`prefix*` retention keys,
//! `encryption_passphrase` and `before_backup`/`after_backup`/`on_error`/
//! monitoring hooks. `parse` counts sections, items and key classes.
//!
//! ```rust
//! let b = concat!(
//!     "source_directories:\n",
//!     "    - /home\n",
//!     "    - /etc\n",
//!     "repositories:\n",
//!     "    - path: /mnt/borg\n",
//!     "      label: main\n",
//!     "retention:\n",
//!     "    keep_daily: 7\n",
//!     "    keep_weekly: 4\n",
//!     "hooks:\n",
//!     "    before_backup:\n",
//!     "        - echo start\n",
//! );
//! let c = izanagi_kit::borgmatic::Borgmatic::parse(b.as_bytes()).unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.repositories, 1);
//! ```

const SECTIONS: &[&str] = &[
    "location:",
    "source_directories:",
    "repositories:",
    "repository:",
    "storage:",
    "archive_name_format:",
    "retention:",
    "consistency:",
    "output:",
    "hooks:",
    "bootstrap:",
    "monitoring:",
    "encryption_passphrase:",
    "ssh_command:",
    "borg_base_directory:",
    "exclude_patterns:",
    "patterns:",
    "exclude_from:",
    "extra_borg_options:",
    "umask:",
    "checkpoint_interval:",
];

const KEEPS: &[&str] = &[
    "keep_daily:",
    "keep_weekly:",
    "keep_monthly:",
    "keep_yearly:",
    "keep_hourly:",
    "keep_minutely:",
    "keep_secondly:",
    "keep_within:",
    "keep_last:",
    "prefix_hostname:",
    "prefix_source:",
    "prefix_archives:",
];

const HOOKS: &[&str] = &[
    "before_backup:",
    "after_backup:",
    "on_error:",
    "before_prune:",
    "after_prune:",
    "before_check:",
    "after_check:",
    "before_compact:",
    "after_compact:",
    "before_create:",
    "after_create:",
    "before_extract:",
    "after_extract:",
    "before_everything:",
    "after_everything:",
    "on_error_and_monitoring:",
    "healthchecks:",
    "loki:",
    "ntfy:",
    "pagerduty:",
    "cronitor:",
    "cronhub:",
    "monitoring:",
];

/// borgmatic YAML census.
#[derive(Debug, Clone)]
pub struct Borgmatic {
    /// Recognized top-level sections.
    pub sections: usize,
    /// `repositories:` entries (`- path:`/`path:` items).
    pub repositories: usize,
    /// Items under `source_directories:`.
    pub sources: usize,
    /// `keep_*`/`prefix*` retention keys.
    pub retention_keys: usize,
    /// `exclude_*`/`patterns` keys and `- ` pattern items.
    pub excludes: usize,
    /// Hook keys (`before_*`/`after_*`/`on_error`/monitoring names).
    pub hooks: usize,
    /// `- ` list items anywhere.
    pub items: usize,
    /// `encryption_passphrase`/`ssh_command`/`borg_base_directory`/`umask` keys.
    pub security: usize,
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// Whether the buffer looks like a borgmatic configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("repositories:")
        && (t.contains("source_directories:")
            || t.contains("keep_")
            || t.contains("encryption_passphrase"))
}

impl Borgmatic {
    /// Parse a borgmatic configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            repositories: 0,
            sources: 0,
            retention_keys: 0,
            excludes: 0,
            hooks: 0,
            items: 0,
            security: 0,
        };
        let mut scope = "";
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let ind = indent(l);
            if ind == 0 {
                scope = s;
                if SECTIONS.contains(&s) {
                    c.sections += 1;
                }
                if KEEPS.contains(&s) {
                    c.retention_keys += 1;
                }
                if HOOKS.contains(&s) {
                    c.hooks += 1;
                }
                if s.starts_with("exclude") || s == "patterns:" {
                    c.excludes += 1;
                }
                if s.starts_with("encryption_passphrase")
                    || s.starts_with("ssh_command")
                    || s.starts_with("borg_base_directory")
                    || s.starts_with("umask")
                    || s.starts_with("passcommand")
                {
                    c.security += 1;
                }
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                match scope {
                    "repositories:" | "repository:" => c.repositories += 1,
                    "source_directories:" => c.sources += 1,
                    "exclude_patterns:" | "patterns:" | "exclude_from:" => c.excludes += 1,
                    _ => {}
                }
                continue;
            }
            if HOOKS.iter().any(|k| s.starts_with(k)) {
                c.hooks += 1;
            }
            if KEEPS.iter().any(|k| s.starts_with(k)) {
                c.retention_keys += 1;
            }
            if s.ends_with(':')
                && (scope == "repositories:" || scope == "repository:")
                && s == "path:"
            {
                c.repositories += 1;
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
            "source_directories:\n",
            "    - /home\n",
            "    - /etc\n",
            "repositories:\n",
            "    - path: /mnt/borg\n",
            "      label: main\n",
            "    - path: /mnt/borg2\n",
            "storage:\n",
            "    encryption_passphrase: x\n",
            "retention:\n",
            "    keep_daily: 7\n",
            "    keep_weekly: 4\n",
            "exclude_patterns:\n",
            "    - '*.tmp'\n",
            "hooks:\n",
            "    before_backup:\n",
            "        - echo start\n",
            "    healthchecks:\n",
            "        ping_url: u\n",
        );
        let c = Borgmatic::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.repositories, 2);
        assert_eq!(c.sources, 2);
        assert_eq!(c.retention_keys, 2);
        assert_eq!(c.excludes, 2);
        assert_eq!(c.hooks, 2);
        assert_eq!(c.items, 6);
    }

    #[test]
    fn rejects_other() {
        assert!(Borgmatic::parse(b"foo: bar").is_none());
    }
}

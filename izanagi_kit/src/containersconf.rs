//! Census of a `containers.conf` configuration file.
//!
//! Podman's `containers.conf` is INI-shaped: `[engine]`/
//! `[containers]`/`[network]`/`[machine]`/`[secrets]`/`[pods]`
//! sections plus `key = value` entries (`#`/`;` comments).
//! Counts total sections, entries, engine keys and comments.
//!
//! ```rust
//! let c = izanagi_kit::containersconf::ContainersConf::parse(
//!     b"[engine]\n# com\nruntime = \"crun\"\n[containers]\ndefault_sysctls = []\n",
//! ).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.entries, 2);
//! ```
#![forbid(unsafe_code)]

/// containers.conf census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainersConf {
    /// Total `[section]` headers.
    pub sections: usize,
    /// `key = value` assignment lines.
    pub entries: usize,
    /// Entries under `[engine]`.
    pub engine_entries: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Recognised sections.
const SECTIONS: &[&str] = &[
    "engine",
    "containers",
    "network",
    "machine",
    "secrets",
    "pods",
    "service_destination",
];

/// Well-known `key` names (used by `detect`).
const KEYS: &[&str] = &[
    "runtime",
    "graphroot",
    "runroot",
    "default_sysctls",
    "cgroup_manager",
    "events_logger",
    "env",
    "init",
    "ipcns",
    "netns",
    "pidns",
    "userns",
    "utsns",
    "log_driver",
    "log_size_max",
    "default_capabilities",
    "devices",
    "dns_servers",
    "network_backend",
    "rootless_networking",
    "machine_image",
];

/// True if `b` looks like containers.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let sections = t
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('[') && l.ends_with(']'))
        .filter(|l| SECTIONS.iter().any(|s| l.trim_matches(['[', ']']) == *s))
        .count();
    let keys = KEYS.iter().filter(|k| t.contains(**k)).count();
    (sections >= 1 && keys >= 1) || keys >= 3
}

impl ContainersConf {
    /// Parse a containers.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut sections = 0usize;
        let mut entries = 0usize;
        let mut engine_entries = 0usize;
        let mut comments = 0usize;
        let mut in_engine = false;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') || l.starts_with(';') {
                comments += 1;
            } else if l.starts_with('[') && l.ends_with(']') {
                sections += 1;
                in_engine = l.trim_matches(['[', ']']) == "engine";
            } else if l.contains('=') {
                entries += 1;
                if in_engine {
                    engine_entries += 1;
                }
            }
        }
        if entries == 0 {
            return None;
        }
        Some(Self {
            sections,
            entries,
            engine_entries,
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# containers.conf\n",
            "[engine]\n",
            "runtime = \"crun\"\n",
            "events_logger = \"file\"\n",
            "[containers]\n",
            "default_sysctls = []\n",
            "[network]\n",
            "network_backend = \"netavark\"\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = ContainersConf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 4);
        assert_eq!(c.engine_entries, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"key = value\nother = 1\n"));
        assert!(ContainersConf::parse(b"# only a comment\n").is_none());
    }
}

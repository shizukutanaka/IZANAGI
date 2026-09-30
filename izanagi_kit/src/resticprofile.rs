//! Census of a `resticprofile` `profiles.yaml` file.
//!
//! Top-level sections: `global:`/`default:`/`groups:`/`includes:`/
//! `priority:` and named profile blocks `<name>:` each holding
//! `initialize`, `repository`, `password-file`, `env-file`,
//! `base-dir`, `cache`, plus command sections `backup:`/`check:`/
//! `forget:`/`retention:`/`snapshots:`/`prune:`/`copy:`/`mount:`/
//! `unlock:`/`rest:`/`self-update:`/`dump:`/`diff:`/`list:`/`ls:`/
//! `restore:`/`version:` with `source:`/`schedule:`/`check-before`/
//! `run-finally`. Counts profiles, command sections, schedules.
//!
//! ```rust
//! let c = izanagi_kit::resticprofile::ResticProfile::parse(
//!     b"default:\n  repository: /repo\nhome:\n  backup:\n    source: ~\n\
//!       schedule: '0 3 * * *'\n",
//! ).unwrap();
//! assert_eq!(c.profiles, 1);
//! assert_eq!(c.command_sections, 1);
//! ```
#![forbid(unsafe_code)]

/// resticprofile profiles.yaml census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResticProfile {
    /// Named profile blocks (top-level `name:` that is not a reserved section).
    pub profiles: usize,
    /// `backup:`/`check:`/`forget:`/… command sections inside profiles.
    pub command_sections: usize,
    /// `schedule:` lines.
    pub schedules: usize,
    /// `global:`/`default:`/`groups:`/`includes:` top sections.
    pub top_sections: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Reserved top-level sections that are not profiles.
const TOP: &[&str] = &[
    "global", "default", "groups", "includes", "priority", "version",
];

/// Profile command sections.
const COMMANDS: &[&str] = &[
    "backup",
    "check",
    "forget",
    "retention",
    "snapshots",
    "prune",
    "copy",
    "mount",
    "unlock",
    "rest",
    "self-update",
    "dump",
    "diff",
    "list",
    "ls",
    "restore",
    "version",
    "generate",
    "stats",
    "repair",
    "tag",
    "init",
    "run-before",
    "run-after",
    "schedule",
];

/// True if `b` looks like profiles.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("repository:") && t.contains("backup:"))
        || t.contains("password-file:")
        || (t.contains("default:") && t.contains("initialize:"))
        || t.contains("resticprofile")
}

impl ResticProfile {
    /// Parse profiles.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            profiles: 0,
            command_sections: 0,
            schedules: 0,
            top_sections: 0,
            comments: 0,
        };
        let mut in_profile = false;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if l.starts_with("schedule:") {
                c.schedules += 1;
                continue;
            }
            if !l.ends_with(':') {
                continue;
            }
            let key = l.trim_end_matches(':');
            if indent == 0 {
                if TOP.contains(&key) {
                    c.top_sections += 1;
                    in_profile = key == "default";
                } else {
                    c.profiles += 1;
                    in_profile = true;
                }
            } else if in_profile && COMMANDS.contains(&key) {
                c.command_sections += 1;
            }
        }
        if c.profiles == 0 && c.top_sections == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# profiles\n",
            "global:\n",
            "  priority: low\n",
            "default:\n",
            "  repository: /repo\n",
            "home:\n",
            "  initialize: true\n",
            "  backup:\n",
            "    source: ~\n",
            "    schedule: '0 3 * * *'\n",
            "  check:\n",
            "    schedule: '0 4 * * 0'\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = ResticProfile::parse(b.as_bytes()).unwrap();
        assert_eq!(c.profiles, 1);
        assert_eq!(c.command_sections, 2);
        assert_eq!(c.schedules, 2);
        assert_eq!(c.top_sections, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(ResticProfile::parse(b"# none\n").is_none());
    }
}

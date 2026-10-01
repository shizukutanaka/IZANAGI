//! Census of an AppArmor profile file.
//!
//! `#include <tunables/global>` includes, `profile name /path {`
//! profile heads, `capability <name>` rules, `network <family> <type>`
//! rules, file rules (`deny owner /path rwx,` — path + perm letters +
//! trailing comma), `@{VAR}=` variable assignments and `r, w, x, k, l,
//! m, ix/px/cx/ux` permission tokens. Counts rules by kind plus
//! comments and deny flags.
//!
//! ```rust
//! let c = izanagi_kit::apparmor::AppArmor::parse(
//!     b"#include <tunables/global>\nprofile foo /usr/bin/foo {\n\
//!       capability net_admin,\ndeny /tmp/** w,\n}\n",
//! ).unwrap();
//! assert_eq!(c.profiles, 1);
//! assert_eq!(c.denies, 1);
//! ```
#![forbid(unsafe_code)]

/// AppArmor profile census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppArmor {
    /// `profile … {` / `hat … {` heads.
    pub profiles: usize,
    /// `#include <…>` lines.
    pub includes: usize,
    /// `capability …,` lines.
    pub capabilities: usize,
    /// `network …` rule lines.
    pub network_rules: usize,
    /// File/path rule lines ending in `,` with perm letters.
    pub file_rules: usize,
    /// Lines carrying `deny`/`audit deny`.
    pub denies: usize,
    /// `@{VAR} = …` tunable assignments.
    pub variables: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// True if `b` looks like an AppArmor profile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("#include <tunables")
        || t.contains("profile ")
        || t.contains("capability ")
        || t.contains("@{")
}

fn is_file_rule(l: &str) -> bool {
    if !l.ends_with(',') {
        return false;
    }
    const PERMS: &[&str] = &[
        "r", "w", "x", "k", "l", "m", "ix", "px", "cx", "ux", "rw", "rwx", "mr", "mk", "ml", "rm",
        "rx", "rwm", "rix",
    ];
    let body = &l[..l.len() - 1];
    body.split_whitespace()
        .next_back()
        .is_some_and(|last| PERMS.contains(&last))
}

impl AppArmor {
    /// Parse an AppArmor profile into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            profiles: 0,
            includes: 0,
            capabilities: 0,
            network_rules: 0,
            file_rules: 0,
            denies: 0,
            variables: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("#include") || l.starts_with("include ") {
                c.includes += 1;
            } else if l.starts_with('#') {
                c.comments += 1;
            } else if (l.starts_with("profile ") || l.starts_with("hat ")) && l.contains('{') {
                c.profiles += 1;
            } else if l.starts_with("capability ") {
                c.capabilities += 1;
            } else if l.starts_with("network ") {
                c.network_rules += 1;
            } else if l.starts_with("@{") && l.contains('=') {
                c.variables += 1;
            } else if is_file_rule(l) {
                c.file_rules += 1;
            }
            if l.contains("deny ") || l.starts_with("deny") {
                c.denies += 1;
            }
        }
        if c.profiles == 0 && c.capabilities == 0 && c.file_rules == 0 {
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
            "# apparmor\n",
            "#include <tunables/global>\n",
            "profile foo /usr/bin/foo {\n",
            "  capability net_admin,\n",
            "  network inet tcp,\n",
            "  @{HOME} = /home/\n",
            "  deny /tmp/** w,\n",
            "  /usr/bin/foo r,\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = AppArmor::parse(b.as_bytes()).unwrap();
        assert_eq!(c.profiles, 1);
        assert_eq!(c.includes, 1);
        assert_eq!(c.capabilities, 1);
        assert_eq!(c.network_rules, 1);
        assert_eq!(c.variables, 1);
        assert_eq!(c.file_rules, 2);
        assert_eq!(c.denies, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[section]\nkey=1\n"));
        assert!(AppArmor::parse(b"# empty\n").is_none());
    }
}

//! sudoers file census.
//!
//! sudoers syntax: `Defaults` lines, `User_Alias`/`Runas_Alias`/
//! `Host_Alias`/`Cmnd_Alias` declarations, user specs
//! `root ALL=(ALL:ALL) ALL`, `%group ALL=(ALL) NOPASSWD: cmds`,
//! `#include`/`#includedir`/`@include`/`@includedir` directives
//! (note: `#include`/`#includedir` start with `#` but are NOT
//! comments).
//!
//! ```rust
//! let k = b"Defaults env_reset\nroot ALL=(ALL:ALL) ALL\n%admin ALL=(ALL) NOPASSWD: /usr/bin/systemctl\n";
//! assert!(izanagi_kit::sudoers::detect(k));
//! ```

/// sudoers census.
#[derive(Debug, Clone)]
pub struct Sudoers {
    /// `Defaults`/`*_Alias` declarations.
    pub declarations: usize,
    /// `user host=(runas) cmds` spec lines.
    pub specs: usize,
    /// `#include(d)`/`@include(d)` lines.
    pub includes: usize,
    /// `#` comment lines (excluding `#include*`).
    pub comments: usize,
}

const ALIASES: &[&str] = &["User_Alias", "Runas_Alias", "Host_Alias", "Cmnd_Alias"];

// Only the tags that are sudoers-exclusive — `MAIL:`/`TYPE=`/`CWD:`-style
// markers also appear in unrelated config formats and were the top source
// of foreign-document false positives.
const TAGS: &[&str] = &[
    "NOPASSWD:",
    "PASSWD:",
    "NOEXEC:",
    "EXEC:",
    "SETENV:",
    "NOSETENV:",
];

fn is_include(s: &str) -> bool {
    s.starts_with("#include")
        || s.starts_with("@include")
        || s.starts_with("#includedir")
        || s.starts_with("@includedir")
}

fn marker(line: &str) -> usize {
    let s = line.trim();
    if s.is_empty() || is_include(s) || s.starts_with('#') {
        return 0;
    }
    let mut n = 0;
    if s == "Defaults" || s.starts_with("Defaults") || ALIASES.iter().any(|a| s.starts_with(a)) {
        n += 1;
    }
    if TAGS.iter().any(|t| s.contains(t)) {
        n += 1;
    }
    // `who where=(runas) command` spec: `x ALL=(` or `x host=`. The
    // `where=` token must be a hostname glued to `=` — a bare `=` (the
    // `key = value` separator of ini-style files) is not a spec.
    if s.split([' ', '\t']).nth(1).is_some_and(|h| {
        h.len() > 1
            && h.ends_with('=')
            && h[..h.len() - 1]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '%' | ','))
    }) || s.contains("=(")
        || s.starts_with("ALL=(")
    {
        n += 1;
    }
    n
}

/// Detect a sudoers-format file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `Defaults`/`*_Alias`/`(runas)` specs/`NOPASSWD:` are
    // sudoers-exclusive constructs.
    let mut strong = 0usize;
    let mut includes = 0usize;
    for line in t.lines() {
        if is_include(line.trim()) {
            includes += 1;
            continue;
        }
        strong += marker(line);
    }
    strong >= 2 || (strong >= 1 && includes >= 1)
}

impl Sudoers {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            declarations: 0,
            specs: 0,
            includes: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if is_include(s) {
                c.includes += 1;
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("Defaults") || ALIASES.iter().any(|a| s.starts_with(a)) {
                c.declarations += 1;
            } else {
                c.specs += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"Defaults env_reset\nroot ALL=(ALL:ALL) ALL\n%admin ALL=(ALL) NOPASSWD: /usr/bin/systemctl\n";
        assert!(detect(b));
        let c = Sudoers::parse(b).unwrap();
        assert_eq!(c.declarations, 1);
        assert_eq!(c.specs, 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"root:x:0:0:root:/root:/bin/sh\n"));
        assert!(!detect(b"# Defaults env_reset\n# root ALL=(ALL) ALL\n"));
        // ini-style `key = value` — the bare `=` separator is not a spec
        assert!(!detect(b"[section]\nkey = value\nother = x\n"));
        assert!(!detect(b"key = value\nother = x\n"));
    }
}

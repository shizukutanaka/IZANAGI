//! Census of a runit service `run`/`finish`/`check` script.
//!
//! `#!/bin/sh` (or `#!/bin/bash`/`#!/usr/bin/env …`) scripts that
//! typically `exec` the daemon under supervision, optionally wrapped
//! by `chpst`, `setuidgid`, `umask`, `ulimit`, `svlogd`, or guarded by
//! `check`/`down` file conventions (`if [ ! -e … ]`). Counts exec
//! lines, supervision wrappers, log helpers and comments.
//!
//! ```rust
//! let c = izanagi_kit::runit::Runit::parse(
//!     b"#!/bin/sh\nexec 2>&1\nexec chpst -u svc /usr/sbin/sshd -D\n",
//! ).unwrap();
//! assert_eq!(c.execs, 2);
//! ```
#![forbid(unsafe_code)]

/// runit script census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Runit {
    /// `exec` lines (service launch).
    pub execs: usize,
    /// `chpst`/`setuidgid`/`softlimit`/`envdir`/`envuidgid` wrappers.
    pub wrappers: usize,
    /// `svlogd`/`svlogd -tt` log sinks.
    pub log_sinks: usize,
    /// `[ -e … ]`/`if`/`test` guard lines.
    pub guards: usize,
    /// `#` comment lines (shebang excluded).
    pub comments: usize,
}

/// Wrapper binaries typical of runit supervision.
const WRAPPERS: &[&str] = &[
    "chpst",
    "setuidgid",
    "softlimit",
    "envdir",
    "envuidgid",
    "fghack",
    "setsid",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// True if `b` looks like a runit run script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let shebang = t.starts_with("#!");
    let exec = t
        .lines()
        .any(|l| l.trim().starts_with("exec ") || l.trim() == "exec 2>&1");
    let wrapper = WRAPPERS.iter().any(|w| t.contains(w)) || t.contains("svlogd");
    (shebang && exec) || (exec && wrapper)
}

impl Runit {
    /// Parse a runit script into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            execs: 0,
            wrappers: 0,
            log_sinks: 0,
            guards: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') && !l.starts_with("#!") {
                c.comments += 1;
                continue;
            }
            if l.starts_with("exec ") || l.starts_with("exec\n") {
                c.execs += 1;
            }
            if WRAPPERS.iter().any(|w| l.contains(w)) {
                c.wrappers += 1;
            }
            if l.contains("svlogd") {
                c.log_sinks += 1;
            }
            if l.starts_with("if ")
                || l.starts_with("if[")
                || l.starts_with("[ ")
                || l.starts_with("test ")
            {
                c.guards += 1;
            }
        }
        if c.execs == 0 {
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
            "#!/bin/sh\n",
            "# sshd service\n",
            "exec 2>&1\n",
            "[ -e /etc/ssh/sshd_config ] || exit 1\n",
            "exec chpst -u root /usr/sbin/sshd -D\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Runit::parse(b.as_bytes()).unwrap();
        assert_eq!(c.execs, 2);
        assert_eq!(c.wrappers, 1);
        assert_eq!(c.guards, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[section]\nkey=1\n"));
        assert!(Runit::parse(b"echo hi\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

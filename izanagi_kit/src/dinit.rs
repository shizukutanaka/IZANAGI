//! Census of a Dinit service file (`/etc/dinit.d/*`).
//!
//! `key = value` settings plus `depends-on`/`depends-ms`/`waits-for`
//! dependency clauses and the service `type = process`/`bgprocess`/
//! `triggered`/`internal`/`scripted`/`preinit`. Other common keys:
//! `command`, `working-dir`, `restart`, `smooth-recovery`,
//! `logfile`, `socket-listen`, `env-file`, `options`, `load-options`,
//! `run-as`, `signal`, `stop-command`, `ready-notification`,
//! `before`/`after`, `pid-file`. Counts entries, dependencies and
//! comments.
//!
//! ```rust
//! let c = izanagi_kit::dinit::Dinit::parse(
//!     b"type = process\ncommand = /usr/sbin/sshd -D\ndepends-on = net\nrestart = true\n",
//! ).unwrap();
//! assert_eq!(c.entries, 4);
//! assert_eq!(c.dependencies, 1);
//! ```
#![forbid(unsafe_code)]

/// Dinit service census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dinit {
    /// `key = value` entries.
    pub entries: usize,
    /// `depends-on`/`depends-ms`/`waits-for`/`before`/`after` clauses.
    pub dependencies: usize,
    /// `restart`/`smooth-recovery`/`log-to-console`/`always-chain` booleans.
    pub flags: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Dependency clause names.
const DEP_KEYS: &[&str] = &[
    "depends-on",
    "depends-ms",
    "waits-for",
    "before",
    "after",
    "waits-for.d",
];

/// Known Dinit keys.
const KEYS: &[&str] = &[
    "type",
    "command",
    "stop-command",
    "working-dir",
    "restart",
    "smooth-recovery",
    "restart-delay",
    "restart-limit-interval",
    "restart-limit-count",
    "logfile",
    "socket-listen",
    "socket-uid",
    "socket-gid",
    "socket-permissions",
    "env-file",
    "options",
    "load-options",
    "run-as",
    "signal",
    "ready-notification",
    "pid-file",
    "depends-on",
    "depends-ms",
    "waits-for",
    "before",
    "after",
    "log-type",
    "log-buffer-size",
    "consumer-of",
    "rlimit-nofile",
    "rlimit-core",
    "rlimit-data",
    "rlimit-addr-space",
];

/// True if `b` looks like a Dinit service file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for line in t.lines() {
        let l = line.trim();
        if l.starts_with('#') || l.is_empty() {
            continue;
        }
        if let Some((k, _)) = l.split_once('=') {
            let k = k.trim();
            if KEYS.contains(&k) || DEP_KEYS.contains(&k) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

impl Dinit {
    /// Parse a Dinit service file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            dependencies: 0,
            flags: 0,
            comments: 0,
        };
        let mut hits = 0usize;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let Some((k, v)) = l.split_once('=') else {
                continue;
            };
            let k = k.trim();
            if !KEYS.contains(&k) && !DEP_KEYS.contains(&k) {
                continue;
            }
            hits += 1;
            c.entries += 1;
            if DEP_KEYS.contains(&k) {
                c.dependencies += 1;
            }
            if matches!(
                k,
                "restart" | "smooth-recovery" | "log-to-console" | "always-chain"
            ) && v.trim() == "true"
            {
                c.flags += 1;
            }
        }
        if hits == 0 {
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
            "# sshd service\n",
            "type = process\n",
            "command = /usr/sbin/sshd -D\n",
            "depends-on = net\n",
            "restart = true\n",
            "logfile = /var/log/sshd.log\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Dinit::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 5);
        assert_eq!(c.dependencies, 1);
        assert_eq!(c.flags, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(Dinit::parse(b"# only\n").is_none());
    }
}

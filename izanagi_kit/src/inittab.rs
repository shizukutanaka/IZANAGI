//! Census of a sysvinit/busybox `/etc/inittab` file.
//!
//! Each entry is `id:runlevels:action:process` where action is one of
//! `sysinit`/`boot`/`bootwait`/`wait`/`respawn`/`once`/`initdefault`/
//! `ctrlaltdel`/`poweroff`/`powerfail`/`restart`/`askfirst`/`shutdown`.
//! Counts entries, respawn entries, per-action tallies and comments.
//!
//! ```rust
//! let c = izanagi_kit::inittab::Inittab::parse(
//!     b"# inittab\nsi::sysinit:/etc/rc.d/rc.sysinit\n1:2345:respawn:/sbin/getty\nid:5:initdefault:\n",
//! ).unwrap();
//! assert_eq!(c.entries, 3);
//! assert_eq!(c.respawn, 1);
//! ```
#![forbid(unsafe_code)]

/// inittab census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inittab {
    /// Valid `id:runlevels:action:process` entry lines.
    pub entries: usize,
    /// Entries with `respawn`/`once`/`wait` style restart actions.
    pub respawn: usize,
    /// `sysinit`/`boot`/`bootwait` entries.
    pub boot_entries: usize,
    /// `initdefault` entries (usually exactly one).
    pub initdefault: usize,
    /// `ctrlaltdel`/`power*` entries.
    pub power_entries: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Known inittab action names.
const ACTIONS: &[&str] = &[
    "sysinit",
    "boot",
    "bootwait",
    "wait",
    "respawn",
    "once",
    "initdefault",
    "ctrlaltdel",
    "poweroff",
    "powerwait",
    "powerokwait",
    "powerfail",
    "powerfailnow",
    "restart",
    "askfirst",
    "shutdown",
    "off",
    "ondemand",
];

/// True if `b` looks like an inittab.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    ACTIONS.iter().any(|a| {
        t.lines().any(|l| {
            let f: Vec<&str> = l.split(':').collect();
            f.len() >= 3 && f[2].trim() == *a
        })
    })
}

impl Inittab {
    /// Parse an inittab into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            respawn: 0,
            boot_entries: 0,
            initdefault: 0,
            power_entries: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let f: Vec<&str> = l.split(':').collect();
            if f.len() >= 3 && ACTIONS.contains(&f[2].trim()) {
                c.entries += 1;
                let a = f[2].trim();
                if a == "respawn" || a == "askfirst" || a == "restart" {
                    c.respawn += 1;
                } else if a == "sysinit" || a == "boot" || a == "bootwait" {
                    c.boot_entries += 1;
                } else if a == "initdefault" {
                    c.initdefault += 1;
                } else if a.starts_with("power") || a == "ctrlaltdel" {
                    c.power_entries += 1;
                }
            }
        }
        if c.entries == 0 {
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
            "# system inittab\n",
            "si::sysinit:/etc/rc.d/rc.sysinit\n",
            "rc:0123456:wait:/etc/rc.d/rc\n",
            "id:5:initdefault:\n",
            "ca::ctrlaltdel:/sbin/shutdown -r\n",
            "1:2345:respawn:/sbin/getty tty1\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Inittab::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 5);
        assert_eq!(c.respawn, 1);
        assert_eq!(c.boot_entries, 1);
        assert_eq!(c.initdefault, 1);
        assert_eq!(c.power_entries, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[section]\nkey = value\n"));
        assert!(Inittab::parse(b"a:b:c\n").is_none());
    }
}

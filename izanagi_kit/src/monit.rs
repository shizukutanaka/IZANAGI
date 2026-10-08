//! Monit `monitrc`/`conf.d/*` census.
//!
//! monitrc uses keyword statements: `check process NAME pidfile PATH`,
//! `if <test> then <action>` rules, `set` globals
//! (`daemon`/`log`/`idfile`/`statefile`/`mailserver`/`mail-format`/
//! `httpd`/`alert`/`eventqueue`/`mmonit`/`limits`), `include` globs,
//! `alert` routes, `start program`/`stop program`/`restart program`,
//! `depends on`, `every`, `mode`, `group`, `host`, `port`,
//! `protocol`, `expect`, `send`, `timeout`, `retries`, `cpu usage`,
//! `memory usage`, `children`, `uptime`, `failed`, `changed`,
//! `exists`, `does not exist`, `monitored by`, `with`, `and`,
//! `within`, `cycles`, `times`. Check types: `process`/`file`/
//! `directory`/`filesystem`/`host`/`network`/`system`/`fifo`/
//! `program`/`interface`/`pid`.
//!
//! ```rust
//! let c = izanagi_kit::monit::Monit::parse(
//!     b"check process nginx pidfile /run/nginx.pid\n  start program = \"/bin/s\"\n",
//! ).unwrap();
//! assert_eq!(c.checks, 1);
//! ```

use crate::textutil::strip_bom;
/// monitrc census.
#[derive(Debug, Clone)]
pub struct Monit {
    /// `check <type> NAME` blocks.
    pub checks: usize,
    /// `if` condition lines.
    pub conditions: usize,
    /// `set` global statements.
    pub settings: usize,
    /// `include`/`alert`/`mailserver`/`start program`/`stop program`/`restart program` lines.
    pub controls: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const CHECK_TYPES: &[&str] = &[
    "process",
    "file",
    "directory",
    "filesystem",
    "host",
    "network",
    "system",
    "fifo",
    "program",
    "interface",
    "pid",
];

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

/// Whether the buffer looks like monitrc.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.contains("check ")
        && (t.contains("pidfile") || t.contains("pid file") || t.contains("if "))
        && t.lines().any(|l| {
            let s = l.trim_start();
            s.starts_with("check ")
                && CHECK_TYPES
                    .iter()
                    .any(|ty| s[6..].starts_with(ty) && s[6 + ty.len()..].starts_with(' '))
        })
}

impl Monit {
    /// Parse monitrc into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            checks: 0,
            conditions: 0,
            settings: 0,
            controls: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let head = first_word(s);
            if s.starts_with("check ") {
                c.checks += 1;
            } else if head == "if" || s.starts_with("if ") {
                c.conditions += 1;
            } else if head == "set" {
                c.settings += 1;
            } else if matches!(
                head,
                "include" | "alert" | "mailserver" | "start" | "stop" | "restart" | "exec"
            ) {
                c.controls += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_monitrc() {
        let b = concat!(
            "# monitrc\n",
            "set daemon 30\n",
            "set log /var/log/monit.log\n",
            "set mailserver localhost\n",
            "check process nginx pidfile /run/nginx.pid\n",
            "  start program = \"/bin/systemctl start nginx\"\n",
            "  stop program = \"/bin/systemctl stop nginx\"\n",
            "  if failed port 80 then restart\n",
            "  if cpu usage > 80% for 3 cycles then alert\n",
            "check host myhost address example.com\n",
            "  if failed ping then alert\n",
            "include /etc/monit.d/*\n",
            "alert ops@example.com\n",
        );
        let c = Monit::parse(b.as_bytes()).unwrap();
        assert_eq!(c.checks, 2);
        assert_eq!(c.conditions, 3);
        assert_eq!(c.settings, 3);
        assert_eq!(c.controls, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Monit::parse(b"check something\nx=1\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

//! Supervisor `supervisord.conf`/`supervisor.d/*.ini` census.
//!
//! supervisord.conf is INI: `[unix_http_server]`/`[inet_http_server]`/
//! `[supervisord]`/`[supervisorctl]`/`[rpcinterface:*]`/
//! `[program:name]`/`[group:name]`/`[eventlistener:name]`/
//! `[fcgi-program:name]`/`[include]`; program options include
//! `command`/`directory`/`user`/`autostart`/`autorestart`/
//! `startsecs`/`startretries`/`exitcodes`/`stopsignal`/`stopwaitsecs`/
//! `stopasgroup`/`killasgroup`/`redirect_stderr`/`stdout_logfile`/
//! `stdout_logfile_maxbytes`/`stdout_logfile_backups`/`stderr_logfile`/
//! `environment`/`numprocs`/`numprocs_start`/`process_name`/`priority`/
//! `serverurl`.
//!
//! ```rust
//! let c = izanagi_kit::supervisor::Supervisor::parse(
//!     b"[supervisord]\nlogfile=/var/log/sv.log\n[program:web]\ncommand=/bin/web\n",
//! ).unwrap();
//! assert_eq!(c.programs, 1);
//! ```

use crate::textutil::strip_bom;
/// supervisord.conf census.
#[derive(Debug, Clone)]
pub struct Supervisor {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value` entries.
    pub entries: usize,
    /// `[program:*]` sections.
    pub programs: usize,
    /// `[group:*]`/`[eventlistener:*]`/`[fcgi-program:*]` sections.
    pub aux_sections: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "unix_http_server",
    "inet_http_server",
    "supervisord",
    "supervisorctl",
    "include",
];

const PREFIXES: &[&str] = &[
    "program:",
    "group:",
    "eventlistener:",
    "fcgi-program:",
    "rpcinterface:",
];

fn section_name(s: &str) -> &str {
    s.trim_start_matches('[').split(']').next().unwrap_or("")
}

/// Whether the buffer looks like supervisord.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    (t.contains("[supervisord]")
        || t.contains("[program:")
        || t.contains("[unix_http_server]")
        || t.contains("[inet_http_server]"))
        && t.lines().any(|l| {
            let s = l.trim();
            s.contains('=')
                && (s.starts_with("command") || s.starts_with("logfile") || s.starts_with("user"))
        })
}

impl Supervisor {
    /// Parse supervisord.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sections: 0,
            entries: 0,
            programs: 0,
            aux_sections: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') || s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
                let name = section_name(s);
                if name.starts_with("program:") {
                    c.programs += 1;
                } else if PREFIXES.iter().any(|p| name.starts_with(p)) && !SECTIONS.contains(&name)
                {
                    c.aux_sections += 1;
                }
                continue;
            }
            if s.contains('=') {
                c.entries += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supervisord_conf() {
        let b = concat!(
            "; supervisor config\n",
            "[unix_http_server]\n",
            "file=/tmp/sv.sock\n",
            "[supervisord]\n",
            "logfile=/var/log/sv.log\n",
            "nodaemon=false\n",
            "[supervisorctl]\n",
            "serverurl=unix:///tmp/sv.sock\n",
            "[program:web]\n",
            "command=/bin/web\n",
            "autostart=true\n",
            "[group:app]\n",
            "programs=web,db\n",
        );
        let c = Supervisor::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.entries, 7);
        assert_eq!(c.programs, 1);
        assert_eq!(c.aux_sections, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Supervisor::parse(b"[a]\nx=1\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

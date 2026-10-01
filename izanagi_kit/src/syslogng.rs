//! Parser for syslog-ng OSE configuration files (`syslog-ng.conf`).
//!
//! Counts `@version:` declaration, `source`/`destination`/`filter`/`parser`/
//! `rewrite`/`template`/`log` block statements, driver calls (`file(...)`,
//! `udp(...)`, `program(...)`), `flags(...)`, and comments.
//!
//! ```
//! let b = b"@version: 4\nsource s_sys { system(); };\ndestination d_f { file(\"/var/log/x\"); };\nlog { source(s_sys); destination(d_f); };\n";
//! assert!(izanagi_kit::syslogng::detect(b));
//! let c = izanagi_kit::syslogng::Syslogng::parse(b).unwrap();
//! assert_eq!(c.sources, 1);
//! assert_eq!(c.destinations, 1);
//! assert_eq!(c.logs, 1);
//! ```

/// Parsed syslog-ng configuration summary.
#[derive(Debug, Clone)]
pub struct Syslogng {
    /// `@version: N` header present (value captured digits-only).
    pub version: usize,
    /// `@module`/`@include`/`@define` directives.
    pub at_directives: usize,
    /// `source NAME { … };` statements.
    pub sources: usize,
    /// `destination NAME { … };` statements.
    pub destinations: usize,
    /// `filter NAME { … };` statements.
    pub filters: usize,
    /// `parser`/`rewrite`/`template`/`junction`/`channel` statements.
    pub other_statements: usize,
    /// `log { … };` path statements.
    pub logs: usize,
    /// Driver invocations `name(...)` inside blocks (file/udp/tcp/system/internal/…).
    pub drivers: usize,
    /// `flags(...)` calls.
    pub flags: usize,
    /// `option(...)`/`option value` option settings.
    pub options: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STMTS: &[&str] = &[
    "source",
    "destination",
    "filter",
    "parser",
    "rewrite",
    "template",
    "junction",
    "channel",
    "log",
];

/// Returns `true` when the bytes look like a syslog-ng configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("@version")
        || (t.contains("source ") && t.contains("destination ") && t.contains("log {"))
        || (t.contains("source(") && t.contains("destination("))
}

impl Syslogng {
    /// Parses a syslog-ng configuration, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            version: 0,
            at_directives: 0,
            sources: 0,
            destinations: 0,
            filters: 0,
            other_statements: 0,
            logs: 0,
            drivers: 0,
            flags: 0,
            options: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(rest) = tr.strip_prefix('@') {
                if rest.starts_with("version") {
                    c.version += 1;
                } else {
                    c.at_directives += 1;
                }
                continue;
            }
            // statement starts: `word NAME {` or bare `word NAME` (single-line)
            let mut it = tr.split_whitespace();
            let kw = it.next().unwrap_or("");
            if STMTS.contains(&kw) && it.next().is_some() {
                match kw {
                    "source" => c.sources += 1,
                    "destination" => c.destinations += 1,
                    "filter" => c.filters += 1,
                    "log" => c.logs += 1,
                    _ => c.other_statements += 1,
                }
            }
            // driver calls inside blocks: `name(...)`/`name (...)` per occurrence
            let mut idx = 0;
            let bytes = tr.as_bytes();
            while idx < bytes.len() {
                if bytes[idx] == b'(' {
                    let start = tr[..idx].rfind([' ', ';', '{']).map_or(0, |p| p + 1);
                    let name = &tr[start..idx];
                    if !name.is_empty()
                        && name
                            .chars()
                            .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
                    {
                        if name == "flags" {
                            c.flags += 1;
                        } else if name == "options" || name == "option" {
                            c.options += 1;
                        } else if !STMTS.contains(&name) {
                            c.drivers += 1;
                        }
                    }
                }
                idx += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"@version: 4\n# conf\nsource s_sys { system(); internal(); };\ndestination d_file { file(\"/var/log/messages\"); };\nfilter f_err { level(err..emerg); };\nlog { source(s_sys); filter(f_err); destination(d_file); flags(flow-control); };\n";

    #[test]
    fn parses_syslogng() {
        let c = Syslogng::parse(CONF).unwrap();
        assert_eq!(c.version, 1);
        assert_eq!(c.sources, 1);
        assert_eq!(c.destinations, 1);
        assert_eq!(c.filters, 1);
        assert_eq!(c.logs, 1);
        assert_eq!(c.flags, 1);
        assert_eq!(c.comments, 1);
        assert_eq!(c.drivers, 4);
    }

    #[test]
    fn rejects_non_syslogng() {
        assert!(!detect(b"Sep 26 00:00 host app[1]: msg"));
        assert!(Syslogng::parse(b"x").is_none());
    }
}

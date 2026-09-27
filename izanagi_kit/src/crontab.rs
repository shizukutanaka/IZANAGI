//! `crontab` files (crontab(5)): `#` comments, `NAME=value` environment
//! settings, `@macro command`, or `min hour dom mon dow command` lines.
//!
//! ```
//! use izanagi_kit::crontab::parse;
//!
//! let d = b"SHELL=/bin/sh\n0 5 * * * /backup.sh\n@daily clean\n";
//! let c = parse(d).unwrap();
//! assert_eq!(c.env.len(), 1);
//! assert_eq!(c.entries.len(), 2);
//! assert_eq!(c.entries[1].schedule, "@daily");
//! ```

/// One scheduled job line.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Either the five whitespace-separated fields joined by spaces, or a
    /// `@macro` name (`@daily`, `@reboot`, ...).
    pub schedule: String,
    /// The command, verbatim.
    pub command: String,
}

/// Parsed crontab.
#[derive(Debug, Clone)]
pub struct Crontab {
    /// `NAME=value` assignments.
    pub env: Vec<(String, String)>,
    /// Job lines in file order.
    pub entries: Vec<Entry>,
}

/// Parse a crontab. Blank lines and `#` comments are skipped; a line with `=`
/// before the first whitespace is an environment assignment.
pub fn parse(data: &[u8]) -> Option<Crontab> {
    let text = std::str::from_utf8(data).ok()?;
    let mut env = Vec::new();
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('@') {
            let mut it = line.splitn(2, char::is_whitespace);
            let schedule = it.next()?.to_string();
            let command = it.next().map(str::trim).unwrap_or("").to_string();
            if command.is_empty() {
                return None;
            }
            entries.push(Entry { schedule, command });
            continue;
        }
        let first_ws = line.find(char::is_whitespace).unwrap_or(line.len());
        let head = &line[..first_ws];
        if !head.starts_with('@') && head.contains('=') && !head.contains('*') {
            let (k, v) = line.split_once('=')?;
            env.push((k.trim().to_string(), v.trim().to_string()));
            continue;
        }
        let f: Vec<&str> = line.splitn(6, char::is_whitespace).collect();
        if f.len() != 6 || f[5].trim().is_empty() {
            return None;
        }
        let schedule = f[..5]
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if schedule.split(' ').count() != 5 {
            return None;
        }
        entries.push(Entry {
            schedule,
            command: f[5].trim().to_string(),
        });
    }
    Some(Crontab { env, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"# comment\nPATH=/bin\n*/15 0 1-5 * MON cmd --flag\n@reboot init\n";
        let c = parse(d).unwrap();
        assert_eq!(c.env[0], ("PATH".to_string(), "/bin".to_string()));
        assert_eq!(c.entries[0].schedule, "*/15 0 1-5 * MON");
        assert_eq!(c.entries[0].command, "cmd --flag");
        assert_eq!(c.entries[1].schedule, "@reboot");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"0 5 * * *\n").is_none()); // schedule only, no command
        assert!(parse(b"0 5 * * /x\n").is_none()); // 4 fields
        assert!(parse(b"@daily\n").is_none()); // macro without command
    }
}

//! Heroku Procfile parsing.
//!
//! Each non-blank, non-comment line is `name: command` where
//! `name` is `[a-z][a-z0-9_-]*` (Heroku requires lowercase;
//! `web` is conventional). No nesting.
//!
//! ```
//! use izanagi_kit::procfile;
//! let d = b"web: bundle exec puma -C config/puma.rb\nworker: rake jobs:work\n";
//! let p = procfile::parse(d).unwrap();
//! assert_eq!(p.processes.len(), 2);
//! assert_eq!(p.get("web").unwrap(), "bundle exec puma -C config/puma.rb");
//! ```

use std::string::String;
use std::vec::Vec;

/// One Procfile entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Process name (e.g. `web`, `worker`).
    pub name: String,
    /// Command line.
    pub command: String,
    /// 1-based line number.
    pub line: usize,
}

/// A parsed Procfile.
#[derive(Clone, Debug, PartialEq)]
pub struct Procfile {
    /// Entries in file order.
    pub processes: Vec<Entry>,
}

impl Procfile {
    /// Looks up a command by process name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.processes
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.command.as_str())
    }
}

fn name_ok(s: &str) -> bool {
    let mut cs = s.chars();
    match cs.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Parses a Procfile: every content line must be `name: cmd`
/// with a lowercase process name; `#` comments and blanks skip.
pub fn parse(d: &[u8]) -> Option<Procfile> {
    let text = std::str::from_utf8(d).ok()?;
    let mut processes = Vec::new();
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, cmd) = line.split_once(':')?;
        let name = name.trim();
        let cmd = cmd.trim();
        if name.is_empty() || !name_ok(name) || cmd.is_empty() {
            return None;
        }
        processes.push(Entry {
            name: name.to_string(),
            command: cmd.to_string(),
            line: i + 1,
        });
    }
    if processes.is_empty() {
        return None;
    }
    Some(Procfile { processes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let p = parse(b"web: bin/web\nrelease: bin/migrate\n").unwrap();
        assert_eq!(p.processes.len(), 2);
        assert_eq!(p.get("release").unwrap(), "bin/migrate");
        assert_eq!(p.processes[0].line, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"# nothing\n").is_none());
        assert!(parse(b"Web: up\n").is_none()); // uppercase name
        assert!(parse(b"web\n").is_none()); // no colon
        assert!(parse(b"web:\n").is_none()); // empty cmd
    }
}

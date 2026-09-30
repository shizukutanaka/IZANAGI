//! Interfile 3.3 header (medical imaging key/value format): starts
//! with `!INTERFILE` (or `INTERFILE`), then `key := value` lines
//! (a leading `!` on keys marks reserved/standard keys), until
//! `END OF INTERFILE` or EOF.
//!
//! ```
//! let d = b"!INTERFILE\n!patient name := Smith\n!study date := 2020:01:01\nEND OF INTERFILE\n";
//! let f = izanagi_kit::interfile::parse(d).unwrap();
//! assert_eq!(f.get("patient name"), Some("Smith"));
//! assert_eq!(f.entries.len(), 2);
//! ```

use std::string::String;
use std::vec::Vec;

/// One `key := value` entry. `reserved` marks keys that began with
/// `!` (Interfile's "private/standard" prefix).
#[derive(Clone, Debug)]
pub struct Entry {
    /// Key text (without leading `!`).
    pub key: String,
    /// Raw value text.
    pub value: String,
    /// `true` when the key carried a leading `!`.
    pub reserved: bool,
}

/// A parsed Interfile header.
#[derive(Clone, Debug)]
pub struct Interfile {
    /// Version string from the `INTERFILE` line (after `:=` if any).
    pub version: Option<String>,
    /// All entries in order.
    pub entries: Vec<Entry>,
    /// Whether `END OF INTERFILE` was seen.
    pub terminated: bool,
}

impl Interfile {
    /// Look up the last value for a key (case-insensitive, `!`
    /// prefix ignored).
    pub fn get(&self, key: &str) -> Option<&str> {
        let want = key.trim_start_matches('!').to_lowercase();
        self.entries
            .iter()
            .rev()
            .find(|e| e.key.to_lowercase() == want)
            .map(|e| e.value.as_str())
    }
}

/// Parse an Interfile header; `None` without the `INTERFILE` marker
/// or when a non-comment line isn't `key := value`.
pub fn parse(d: &[u8]) -> Option<Interfile> {
    let s = std::str::from_utf8(d).ok()?;
    let mut entries = Vec::new();
    let mut version = None;
    let mut seen_marker = false;
    let mut terminated = false;
    for raw in s.lines() {
        let line = raw.trim_end();
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let upper = t.to_uppercase();
        if upper.starts_with("END OF INTERFILE") {
            terminated = true;
            break;
        }
        if !seen_marker {
            let m = t.trim_start_matches('!');
            if m.to_uppercase().starts_with("INTERFILE") {
                seen_marker = true;
                if let Some((_, v)) = m.split_once(":=") {
                    let v = v.trim();
                    if !v.is_empty() {
                        version = Some(v.to_string());
                    }
                }
                continue;
            }
            return None;
        }
        // comment-only line
        if upper.starts_with(";") {
            continue;
        }
        let body = t.strip_prefix('!').unwrap_or(t);
        let reserved = t.starts_with('!');
        let (k, v) = body.split_once(":=")?;
        let k = k.trim();
        if k.is_empty() {
            return None;
        }
        entries.push(Entry {
            key: k.to_string(),
            value: v.trim().to_string(),
            reserved,
        });
    }
    if !seen_marker || entries.is_empty() {
        return None;
    }
    Some(Interfile {
        version,
        entries,
        terminated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"!INTERFILE := 3.3\n!imaging modality := nucmed\nmodality (local) := PT\n; comment\nEND OF INTERFILE\n";
        let f = parse(d).unwrap();
        assert_eq!(f.version.as_deref(), Some("3.3"));
        assert!(f.terminated);
        assert_eq!(f.entries.len(), 2);
        assert_eq!(f.get("IMAGING MODALITY"), Some("nucmed"));
        assert_eq!(f.get("modality (local)"), Some("PT"));
        assert!(f.entries[0].reserved);
        assert!(!f.entries[1].reserved);
        assert_eq!(f.get("missing"), None);
    }

    #[test]
    fn no_end_marker() {
        let f = parse(b"!INTERFILE\n!a := 1\n").unwrap();
        assert!(!f.terminated);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello").is_none()); // no marker first
        assert!(parse(b"!INTERFILE\n").is_none()); // no entries
        assert!(parse(b"!INTERFILE\nbad line\n").is_none()); // no :=
    }
}

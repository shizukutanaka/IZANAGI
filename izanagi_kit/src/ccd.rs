//! CloneCD control file (`.ccd`) — INI-like disc descriptor.
//!
//! `[Section]` headers followed by `Key=Value` lines: `[CloneCD]`,
//! `[Disc]` (`TocEntries`, `DataTracksScrambled`...), `[Session N]`,
//! `[Track N]`, `[Entry N]`. Values may be empty; anything else fails.
//!
//! ```
//! use izanagi_kit::ccd::parse;
//!
//! let c = parse(b"[CloneCD]\nVersion=3\n[Disc]\nTocEntries=4\n").unwrap();
//! assert_eq!(c.get("CloneCD", "Version"), Some("3"));
//! assert_eq!(c.get("Disc", "TocEntries"), Some("4"));
//! ```

/// One `[Section]` block.
#[derive(Clone, Debug)]
pub struct Section {
    /// Section name without brackets (e.g. `"Track 1"`).
    pub name: String,
    /// `key=value` pairs in file order.
    pub pairs: Vec<(String, String)>,
}

/// A parsed `.ccd` file.
#[derive(Clone, Debug)]
pub struct Ccd {
    /// Sections in file order.
    pub sections: Vec<Section>,
}

impl Ccd {
    /// First `value` for `key` inside `section`.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .iter()
            .find(|s| s.name == section)
            .and_then(|s| s.pairs.iter().find(|(k, _)| k == key))
            .map(|(_, v)| v.as_str())
    }
}

/// Parse a `.ccd` file. `None` on a line before the first section, a
/// non-`=` line inside one, or no sections at all.
pub fn parse(d: &[u8]) -> Option<Ccd> {
    let text = std::str::from_utf8(d).ok()?;
    let mut sections: Vec<Section> = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(name) = t.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if name.is_empty() {
                return None;
            }
            sections.push(Section {
                name: name.to_string(),
                pairs: Vec::new(),
            });
            continue;
        }
        let (k, v) = t.split_once('=')?;
        let sec = sections.last_mut()?;
        if k.is_empty() {
            return None;
        }
        sec.pairs.push((k.to_string(), v.to_string()));
    }
    if sections.is_empty() {
        return None;
    }
    Some(Ccd { sections })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let c = parse(
            b"[CloneCD]\nVersion=3\n[Session 1]\nPreGapMode=1\n[Track 1]\nMODE=1\nIndex 1=150\n",
        )
        .unwrap();
        assert_eq!(c.sections.len(), 3);
        assert_eq!(c.get("Track 1", "Index 1"), Some("150"));
        assert_eq!(c.get("Track 1", "nope"), None);
        assert_eq!(c.get("nope", "x"), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key=value\n").is_none());
        assert!(parse(b"[]\nk=v\n").is_none());
        assert!(parse(b"[S]\nbadline\n").is_none());
        assert!(parse(b"[S]\n=v\n").is_none());
    }
}

//! Minecraft `whitelist.json` census.
//!
//! JSON array of `{ "uuid": "…", "name": "…" }` entries.
//!
//! ```rust
//! let w = r#"[{"uuid": "a-b-c-d-e", "name": "Steve"}]"#;
//! let c = izanagi_kit::whitelist::Whitelist::parse(w.as_bytes()).unwrap();
//! assert_eq!(c.entries, 1);
//! ```

/// whitelist.json census.
#[derive(Debug, Clone)]
pub struct Whitelist {
    /// `{…}` entries.
    pub entries: usize,
    /// `uuid` fields.
    pub uuids: usize,
    /// `name` fields.
    pub names: usize,
}

/// Whether the buffer looks like whitelist.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"uuid\"") && t.contains("\"name\"") && !t.contains("\"level\"")
}

impl Whitelist {
    /// Parse a whitelist.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            uuids: 0,
            names: 0,
        };
        for seg in t.split('{') {
            if seg.contains("\"uuid\"") {
                c.entries += 1;
                c.uuids += seg.matches("\"uuid\"").count();
                c.names += seg.matches("\"name\"").count();
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whitelist() {
        let b = concat!(
            "[\n",
            "  {\"uuid\": \"00000000-0000-0000-0000-000000000001\", \"name\": \"Steve\"},\n",
            "  {\"uuid\": \"00000000-0000-0000-0000-000000000002\", \"name\": \"Alex\"},\n",
            "  {\"uuid\": \"00000000-0000-0000-0000-000000000003\", \"name\": \"Herobrine\"}\n",
            "]\n",
        );
        let c = Whitelist::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 3);
        assert_eq!(c.uuids, 3);
        assert_eq!(c.names, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Whitelist::parse(b"[{\"a\":1}]").is_none());
    }
}

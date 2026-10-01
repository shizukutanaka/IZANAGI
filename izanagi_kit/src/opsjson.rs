//! Minecraft `ops.json` census.
//!
//! JSON array of operator objects:
//! `[{ "uuid": "…", "name": "…", "level": 4, "bypassesPlayerLimit": false }]`.
//!
//! ```rust
//! let o = r#"[{"uuid": "a-b-c-d-e", "name": "Steve", "level": 4, "bypassesPlayerLimit": true}]"#;
//! let c = izanagi_kit::opsjson::Opsjson::parse(o.as_bytes()).unwrap();
//! assert_eq!(c.entries, 1);
//! assert_eq!(c.bypassers, 1);
//! ```

/// ops.json census.
#[derive(Debug, Clone)]
pub struct Opsjson {
    /// `{…}` operator entries.
    pub entries: usize,
    /// `uuid` fields seen.
    pub uuids: usize,
    /// `level` field sum.
    pub levels: u64,
    /// `bypassesPlayerLimit: true` count.
    pub bypassers: usize,
}

/// Whether the buffer looks like ops.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"uuid\"") && t.contains("\"level\"") || t.contains("bypassesPlayerLimit")
}

impl Opsjson {
    /// Parse an ops.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            uuids: 0,
            levels: 0,
            bypassers: 0,
        };
        let mut cur = false;
        for tok in t.split(['{', '}']) {
            if tok.contains("\"uuid\"") {
                cur = true;
            }
            if cur && tok.contains("\"uuid\"") {
                c.entries += 1;
                c.uuids += 1;
            }
            if cur && tok.contains("\"level\"") {
                for seg in tok.split("\"level\"").skip(1) {
                    let seg = seg.trim_start_matches([':', ' ', '"']);
                    let num: String = seg.chars().take_while(|ch| ch.is_ascii_digit()).collect();
                    c.levels += num.parse::<u64>().unwrap_or(0);
                }
            }
            if cur && tok.contains("bypassesPlayerLimit") {
                for seg in tok.split("bypassesPlayerLimit").skip(1) {
                    if seg.trim_start_matches(['"', ':', ' ']).starts_with("true") {
                        c.bypassers += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ops() {
        let b = concat!(
            "[\n",
            "  {\"uuid\": \"00000000-0000-0000-0000-000000000001\", \"name\": \"Steve\", \"level\": 4, \"bypassesPlayerLimit\": true},\n",
            "  {\"uuid\": \"00000000-0000-0000-0000-000000000002\", \"name\": \"Alex\", \"level\": 2, \"bypassesPlayerLimit\": false},\n",
            "  {\"uuid\": \"00000000-0000-0000-0000-000000000003\", \"name\": \"Herobrine\", \"level\": 1}\n",
            "]\n",
        );
        let c = Opsjson::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 3);
        assert_eq!(c.uuids, 3);
        assert_eq!(c.levels, 7);
        assert_eq!(c.bypassers, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Opsjson::parse(b"[{\"a\":1}]").is_none());
    }
}

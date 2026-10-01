//! Minecraft `banned-ips.json` census.
//!
//! JSON array of ban objects:
//! `{ "ip": "1.2.3.4", "created": "2026-01-01 00:00:00 +0000",
//! "source": "Server", "expires": "forever", "reason": "…" }`.
//!
//! ```rust
//! let b = r#"[{"ip": "1.2.3.4", "created": "x", "source": "s", "expires": "forever", "reason": "r"}]"#;
//! let c = izanagi_kit::bannedips::Bannedips::parse(b.as_bytes()).unwrap();
//! assert_eq!(c.entries, 1);
//! assert_eq!(c.forever, 1);
//! ```

/// banned-ips.json census.
#[derive(Debug, Clone)]
pub struct Bannedips {
    /// `{…}` ban entries.
    pub entries: usize,
    /// `ip` fields.
    pub ips: usize,
    /// `expires: "forever"` count.
    pub forever: usize,
    /// Non-empty `reason` fields.
    pub reasons: usize,
}

/// Whether the buffer looks like banned-ips.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"ip\"") && (t.contains("\"expires\"") || t.contains("\"reason\""))
}

impl Bannedips {
    /// Parse a banned-ips.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            ips: 0,
            forever: 0,
            reasons: 0,
        };
        for seg in t.split('{') {
            if !seg.contains("\"ip\"") {
                continue;
            }
            c.entries += 1;
            c.ips += seg.matches("\"ip\"").count();
            if seg.contains("\"expires\"") && seg.contains("forever") {
                c.forever += 1;
            }
            for r in seg.split("\"reason\"").skip(1) {
                let v = r.trim_start_matches([':', ' ', '"']);
                if !v.starts_with('"') || !v.starts_with("\"\"") {
                    c.reasons += 1;
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
    fn parses_bans() {
        let b = concat!(
            "[\n",
            "  {\"ip\": \"1.2.3.4\", \"created\": \"2026-01-01\", \"source\": \"Server\", \"expires\": \"forever\", \"reason\": \"griefing\"},\n",
            "  {\"ip\": \"5.6.7.8\", \"created\": \"2026-01-02\", \"source\": \"Admin\", \"expires\": \"2027-01-01\", \"reason\": \"spam\"}\n",
            "]\n",
        );
        let c = Bannedips::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 2);
        assert_eq!(c.ips, 2);
        assert_eq!(c.forever, 1);
        assert_eq!(c.reasons, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Bannedips::parse(b"[{\"a\":1}]").is_none());
    }
}

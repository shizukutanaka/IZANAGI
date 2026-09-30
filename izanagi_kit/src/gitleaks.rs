//! gitleaks `gitleaks.toml` census.
//!
//! gitleaks config is TOML with `[extend]` (`useDefault`/`path`),
//! `[[rules]]` blocks (`id`/`description`/`regex`/`path`/`secretGroup`/
//! `keywords`/`entropy`/`minTargetMatch`/`tags`/`priority`),
//! `[whitelist]`/`[allowlist]`/`[[rules.whitelists]]`/
//! `[[rules.allowlists]]`/`[target]` sections.
//!
//! ```rust
//! let c = izanagi_kit::gitleaks::Gitleaks::parse(b"[[rules]]\nid = \"r\"\nregex = \"x\"\n").unwrap();
//! assert_eq!(c.rules, 1);
//! ```

/// `gitleaks.toml` census.
#[derive(Debug, Clone)]
pub struct Gitleaks {
    /// `[[rules]]` rule blocks.
    pub rules: usize,
    /// `[extend]`/`[whitelist]`/`[allowlist]`/`[target]`/other sections.
    pub sections: usize,
    /// `key = value` settings.
    pub settings: usize,
    /// `regex =`/`path =`/`keywords =` entries.
    pub detectors: usize,
    /// `#` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a gitleaks.toml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[[rules]]")
        || (t.contains("[extend]") && (t.contains("useDefault") || t.contains("rules")))
        || (t.contains("secretGroup") && t.contains("regex"))
}

impl Gitleaks {
    /// Parse a gitleaks.toml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            rules: 0,
            sections: 0,
            settings: 0,
            detectors: 0,
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
            if s.starts_with("[[") {
                if s.contains("rules]]") {
                    c.rules += 1;
                } else {
                    c.sections += 1;
                }
                continue;
            }
            if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
                continue;
            }
            if s.contains('=') {
                c.settings += 1;
                let key = s.split('=').next().unwrap_or("").trim();
                if key == "regex" || key == "path" || key == "keywords" || key == "secretGroup" {
                    c.detectors += 1;
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
    fn parses_gitleaks() {
        let b = concat!(
            "[extend]\n",
            "useDefault = true\n",
            "[[rules]]\n",
            "  id = \"aws-access-key\"\n",
            "  description = \"AWS key\"\n",
            "  regex = '''AKIA[0-9A-Z]{16}'''\n",
            "  keywords = [\"akia\"]\n",
            "  entropy = 4.5\n",
            "  tags = [\"aws\", \"key\"]\n",
            "[[rules]]\n",
            "  id = \"slack-token\"\n",
            "  regex = '''xox[baprs]-[0-9a-zA-Z-]{10,48}'''\n",
            "  secretGroup = 0\n",
            "[whitelist]\n",
            "  paths = [\"tests\"]\n",
            "# comment\n",
        );
        let c = Gitleaks::parse(b.as_bytes()).unwrap();
        assert_eq!(c.rules, 2);
        assert_eq!(c.sections, 2);
        assert_eq!(c.settings, 11);
        assert_eq!(c.detectors, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Gitleaks::parse(b"[foo]\nx = 1\n").is_none());
    }
}

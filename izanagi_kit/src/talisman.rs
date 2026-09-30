//! Talisman `.talismanrc` census.
//!
//! `.talismanrc` is YAML configuring the Talisman secret scanner:
//! `fileignoreconfig:` (`- filename:`/`checksum:`/`allowed_patterns`/
//! `allowed_binaries`), `threshold:`/`scope:`/`severity`/
//! `expectedreportlocation`/`custom_severities`/`custom_patterns`/
//! `scaninfo`/`gitignore`/`errors`/`warnings`/`allowed_methods`.
//!
//! ```rust
//! let c = izanagi_kit::talisman::Talisman::parse(b"fileignoreconfig:\n  - filename: a.pem\n  - filename: b.key\nthreshold: high\n").unwrap();
//! assert_eq!(c.ignores, 2);
//! ```

/// `.talismanrc` census.
#[derive(Debug, Clone)]
pub struct Talisman {
    /// `fileignoreconfig` `- filename:` entries.
    pub ignores: usize,
    /// `allowed_patterns`/`allowed_binaries`/`custom_patterns` entries.
    pub patterns: usize,
    /// Other `key:`/`key: value` settings.
    pub settings: usize,
    /// `#` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a `.talismanrc`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("fileignoreconfig")
        || t.contains("expectedreportlocation")
        || (t.contains("talisman") && (t.contains("threshold") || t.contains("severity")))
        || (t.contains("checksum:") && t.contains("filename:"))
}

impl Talisman {
    /// Parse a `.talismanrc` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            ignores: 0,
            patterns: 0,
            settings: 0,
            comments: 0,
        };
        let mut scope = "";
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") || s == "-" {
                let body = s.trim_start_matches('-').trim();
                if body.starts_with("filename:") {
                    c.ignores += 1;
                } else if scope == "allowed_patterns"
                    || scope == "allowed_binaries"
                    || scope == "custom_patterns"
                    || body.starts_with("pattern")
                {
                    c.patterns += 1;
                } else if body.ends_with(':') || body.contains(": ") {
                    c.settings += 1;
                }
                continue;
            }
            if s.ends_with(':') || s.contains(": ") {
                let key = s.split(':').next().unwrap_or("").trim();
                scope = key;
                if key == "checksum" {
                    // checksum pairs with a filename already counted
                } else if key == "filename" {
                    c.ignores += 1;
                } else {
                    c.settings += 1;
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
    fn parses_talismanrc() {
        let b = concat!(
            "fileignoreconfig:\n",
            "  - filename: certs/root.pem\n",
            "    checksum: abcdef\n",
            "  - filename: keys/app.key\n",
            "    checksum: 123456\n",
            "  - filename: src/config.go\n",
            "    allowed_patterns:\n",
            "      - password\n",
            "      - secret\n",
            "threshold: high\n",
            "scope: .\n",
            "expectedreportlocation: talisman_reports/\n",
            "# comment\n",
        );
        let c = Talisman::parse(b.as_bytes()).unwrap();
        assert_eq!(c.ignores, 3);
        assert_eq!(c.patterns, 2);
        assert_eq!(c.settings, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Talisman::parse(b"foo: bar\n").is_none());
    }
}

//! detect-secrets `.secrets.baseline` census.
//!
//! `.secrets.baseline` is JSON produced by `detect-secrets`:
//! `"version"`, `"generated_at"`, `"plugins_used"` (entries with
//! `"name"`), `"filters_used"` (`"path"` entries), `"results"` —
//! a filename map of findings, each `"type"`, `"line_number"`,
//! `"hashed_secret"`, `"is_verified"`, `"is_secret"`,
//! `"verified_result"`, `"line"`.
//!
//! ```rust
//! let c = izanagi_kit::secretsbaseline::SecretsBaseline::parse(b"{\"version\":\"x\",\"results\":{\"f.rs\":[{\"hashed_secret\":\"h\",\"is_verified\":true}]}}").unwrap();
//! assert_eq!(c.findings, 1);
//! ```

/// `.secrets.baseline` census.
#[derive(Debug, Clone)]
pub struct SecretsBaseline {
    /// `"plugins_used"`/`"filters_used"` entries.
    pub plugins: usize,
    /// Finding objects under `"results"`.
    pub findings: usize,
    /// `is_verified: true`/`is_secret: true` findings.
    pub verified: usize,
    /// Other `"key":` entries.
    pub keys: usize,
    /// Comment bytes are invalid in JSON; always 0.
    pub comments: usize,
}

/// Whether the buffer looks like a `.secrets.baseline`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"plugins_used\"")
        || t.contains("\"filters_used\"")
        || t.contains("\"generated_at\""))
        && t.contains("\"results\"")
        || t.contains("\"hashed_secret\"")
}

impl SecretsBaseline {
    /// Parse a `.secrets.baseline` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            plugins: 0,
            findings: 0,
            verified: 0,
            keys: 0,
            comments: 0,
        };
        // Each `"name":` marks a plugin entry; filters share the shape.
        c.plugins = t.matches("\"name\":").count();
        // findings: each object under "results" has "type"/"hashed_secret"
        c.findings = t
            .matches("\"hashed_secret\"")
            .count()
            .max(t.matches("\"type\":").count());
        c.verified =
            t.matches("\"is_verified\": true").count() + t.matches("\"is_secret\": true").count();
        let mut in_string = false;
        let mut esc = false;
        let mut expect_key = false;
        for ch in t.bytes() {
            if in_string {
                if esc {
                    esc = false;
                } else if ch == b'\\' {
                    esc = true;
                } else if ch == b'"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                b'"' => {
                    in_string = true;
                    expect_key = true;
                }
                b':' => {
                    if expect_key {
                        c.keys += 1;
                        expect_key = false;
                    }
                }
                b',' | b'{' | b'}' | b'[' | b']' => expect_key = false,
                _ => {}
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_baseline() {
        let b = concat!(
            "{\n",
            "  \"version\": \"1.5.0\",\n",
            "  \"generated_at\": \"2024-01-01\",\n",
            "  \"plugins_used\": [\n",
            "    {\"name\": \"AWSKeyDetector\"},\n",
            "    {\"name\": \"GitHubTokenDetector\"},\n",
            "    {\"name\": \"HexHighEntropyString\"}\n",
            "  ],\n",
            "  \"filters_used\": [\n",
            "    {\"path\": \"detect_secrets.filters.common.is_ignored_due_to_verification_policies\"}\n",
            "  ],\n",
            "  \"results\": {\n",
            "    \"src/a.py\": [\n",
            "      {\"type\": \"AWS\", \"line_number\": 10, \"hashed_secret\": \"h1\", \"is_verified\": true, \"is_secret\": true},\n",
            "      {\"type\": \"Hex\", \"line_number\": 12, \"hashed_secret\": \"h2\", \"is_verified\": false}\n",
            "    ],\n",
            "    \"src/b.rs\": [\n",
            "      {\"type\": \"Base64\", \"line_number\": 3, \"hashed_secret\": \"h3\", \"is_verified\": false}\n",
            "    ]\n",
            "  }\n",
            "}\n",
        );
        let c = SecretsBaseline::parse(b.as_bytes()).unwrap();
        assert_eq!(c.plugins, 3);
        assert_eq!(c.findings, 3);
        assert_eq!(c.verified, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(SecretsBaseline::parse(b"{\"a\":1}").is_none());
    }
}

//! Census of a `.babelrc` / `babel.config.json` / `babel.config.js`.
//!
//! Keys: `presets` (`@babel/preset-env`/`preset-typescript`/`preset-react`/
//! `preset-flow`/preset names and `[name, options]` tuples), `plugins`
//! (`@babel/plugin-*` entries), `env` (`development`/`production`/`test`
//! sub-objects), `overrides`, `sourceMaps`, `compact`, `minified`,
//! `retainLines`, `comments`, `ignore`, `only`, `include`, `exclude`,
//! `extends`, `assumptions`, `targets`, `browserslistConfigFile`,
//! `browserslistEnv`, `inputSourceMap`, `sourceType`, `caller`, `rootMode`.
//!
//! ```rust
//! let c = izanagi_kit::babelrc::BabelRc::parse(
//!     b"{ \"presets\": [\"@babel/preset-env\"], \"plugins\": [\"@babel/plugin-transform-runtime\"] }",
//! ).unwrap();
//! assert!(c.presets >= 1);
//! ```
#![forbid(unsafe_code)]

/// .babelrc census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BabelRc {
    /// `presets` array entries.
    pub presets: usize,
    /// `plugins` array entries.
    pub plugins: usize,
    /// `env` sub-object names (`development`/`production`/`test`).
    pub env: usize,
    /// `overrides` entries.
    pub overrides: usize,
    /// Other `key:`/`"key":` keys.
    pub keys: usize,
    /// Preset/plugin entries carrying `[name, opts]` tuples.
    pub tuples: usize,
}

/// True if `b` looks like babel config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("presets") || t.contains("plugins") || t.contains("babel"))
        && (t.contains('"') || t.contains('\''))
        && (t.contains('{') || t.contains('['))
}

impl BabelRc {
    /// Count the census fields; `None` unless [`detect`] holds.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            presets: 0,
            plugins: 0,
            env: 0,
            overrides: 0,
            keys: 0,
            tuples: 0,
        };
        let mut in_presets = false;
        let mut in_plugins = false;
        let mut in_env = false;
        let mut in_overrides = false;
        for raw in t.lines() {
            let l = raw.trim();
            if l.is_empty() || l.starts_with("//") {
                continue;
            }
            if l.contains("presets") && l.contains('[') {
                in_presets = true;
            }
            if l.contains("plugins") && l.contains('[') {
                in_plugins = true;
            }
            if (l.contains("\"env\"") || l.contains("env:") || l.contains("env\""))
                && l.contains('{')
            {
                in_env = true;
            }
            if l.contains("overrides") && (l.contains('[') || l.contains('{')) {
                in_overrides = true;
            }
            if l.starts_with(']') {
                in_presets = false;
                in_plugins = false;
                in_overrides = false;
            }
            if l.starts_with('}') {
                in_env = false;
            }
            // count quoted array items inside presets/plugins lists
            if in_presets || in_plugins {
                let mut items = 0usize;
                for ch in l.chars() {
                    if ch == '"' || ch == '\'' {
                        items += 1;
                    }
                    if ch == '[' && l.find('[') != l.rfind('[') {
                        c.tuples += 1;
                    }
                }
                // two quotes per string → items/2 entries; on bracket lines the
                // `presets:` key itself contributes 2 quotes — skip if key-only.
                if in_presets {
                    c.presets += items / 2;
                } else {
                    c.plugins += items / 2;
                }
            }
            if in_env {
                // env names like `"development":`
                if (l.contains("\"development\"") || l.contains("development:"))
                    || (l.contains("\"production\"") || l.contains("production:"))
                    || (l.contains("\"test\"") || l.contains("test:"))
                {
                    c.env += 1;
                }
            }
            if in_overrides && l.contains('{') {
                c.overrides += 1;
            }
            // count `"key":` keys
            let bytes = l.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'"' || bytes[i] == b'\'' {
                    let q = bytes[i];
                    i += 1;
                    let start = i;
                    while i < bytes.len() && bytes[i] != q {
                        i += 1;
                    }
                    let s = &l[start..i];
                    i += 1;
                    if i < bytes.len()
                        && bytes[i] == b':'
                        && !s.is_empty()
                        && s.chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    {
                        c.keys += 1;
                    }
                } else {
                    i += 1;
                }
            }
            // also `key:` unquoted keys
            if !l.starts_with('"') && !l.starts_with('\'') {
                if let Some(colon) = l.find(':') {
                    let key = l[..colon]
                        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("");
                    if !key.is_empty()
                        && key
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    {
                        c.keys += 1;
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

    const SAMPLE: &[u8] = b"{
  \"presets\": [
    \"@babel/preset-env\",
    [\"@babel/preset-react\", { \"runtime\": \"automatic\" }]
  ],
  \"plugins\": [
    \"@babel/plugin-transform-runtime\",
    \"babel-plugin-styled-components\"
  ],
  \"env\": {
    \"production\": { \"compact\": true },
    \"development\": { \"sourceMaps\": true }
  },
  \"overrides\": [
    { \"test\": \"*.ts\", \"presets\": [\"@babel/preset-typescript\"] }
  ]
}";

    #[test]
    fn detects_config() {
        assert!(detect(SAMPLE));
        assert!(!detect(b"plain text"));
    }

    #[test]
    fn parses() {
        let c = BabelRc::parse(SAMPLE).unwrap();
        assert!(c.presets >= 3);
        assert!(c.plugins >= 2);
        assert!(c.env >= 2);
        assert!(c.overrides >= 1);
        assert!(c.keys >= 4);
    }

    #[test]
    fn babel_config_js() {
        let c = BabelRc::parse(
            b"module.exports = { presets: [['@babel/preset-env', { targets: { node: 'current' } }]] };",
        )
        .unwrap();
        assert!(c.presets >= 2);
    }

    #[test]
    fn rejects() {
        assert!(BabelRc::parse(b"").is_none());
        assert!(BabelRc::parse(b"{}").is_none());
    }
}

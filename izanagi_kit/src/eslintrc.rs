//! ESLint legacy config (`.eslintrc.json`/`.eslintrc.yml`) parser.
//!
//! Detects JSON or YAML configs by the characteristic top-level keys
//! (`env`/`extends`/`plugins`/`rules`/`parserOptions`/`overrides`/`root` …)
//! and counts sections, severity-value rule entries, and `true`-valued env
//! entries.
//!
//! ```
//! let b = br#"{
//!   "env": { "browser": true, "node": true },
//!   "extends": ["eslint:recommended"],
//!   "rules": { "no-console": "warn", "eqeqeq": ["error", "always"] }
//! }"#;
//! assert!(izanagi_kit::eslintrc::detect(b));
//! let c = izanagi_kit::eslintrc::Eslintrc::parse(b).unwrap();
//! assert_eq!(c.rule_entries, 2);
//! ```

/// Parsed ESLint config summary.
#[derive(Debug, Clone)]
pub struct Eslintrc {
    /// Key lines (`"key":`/`key:` shape).
    pub keys: usize,
    /// Known top-level config sections present.
    pub sections: usize,
    /// Rule entries (value token is a severity: `error`/`warn`/`off`/`0`/`1`/`2` or a `[`-array).
    pub rule_entries: usize,
    /// `": true"` / `: true` boolean entries (env/globals style).
    pub bool_entries: usize,
    /// `//`/`#`/`/*` comment lines (JSONC tolerated).
    pub comments: usize,
}

/// Known top-level keys.
const SECTIONS: &[&str] = &[
    "env",
    "extends",
    "globals",
    "ignorePatterns",
    "noInlineConfig",
    "overrides",
    "parser",
    "parserOptions",
    "plugins",
    "processor",
    "reportUnusedDisableDirectives",
    "root",
    "rules",
    "settings",
];

fn is_key_line(tr: &str) -> bool {
    // `"x":` or `x:` — key before the colon.
    let mut t = tr;
    if let Some(st) = t.strip_prefix('-') {
        t = st.trim_start();
    }
    let t = t.trim_start_matches('"').trim_start_matches('\'');
    let Some(colon) = t.find(':') else {
        return false;
    };
    colon > 0
        && colon < 80
        && t[..colon]
            .trim_end_matches('"')
            .trim_end_matches('\'')
            .trim_end()
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | '@' | '$'))
}

fn value_tok(s: &str) -> &str {
    s.trim_start()
        .trim_start_matches('"')
        .trim_start_matches('\'')
        .split(|c: char| c.is_ascii_whitespace() || c == ',' || c == '[' || c == ']')
        .next()
        .unwrap_or("")
        .trim_end_matches('"')
        .trim_end_matches('\'')
        .trim_end_matches(',')
}

fn value_token(tr: &str) -> &str {
    let Some(colon) = tr.find(':') else { return "" };
    let raw = tr[colon + 1..].trim_start();
    if let Some(rest) = raw.strip_prefix('[') {
        let first = value_tok(rest);
        return if matches!(first, "error" | "warn" | "off" | "0" | "1" | "2") {
            "["
        } else {
            ""
        };
    }
    value_tok(raw)
}

const SEVERITIES: &[&str] = &[
    "error", "warn", "off", "0", "1", "2", "\"error", "\"warn", "\"off",
];

/// Detect an `.eslintrc`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = SECTIONS
        .iter()
        .filter(|k| {
            t.contains(&format!("\"{k}\"",))
                || t.contains(&format!("{k}:"))
                || t.contains(&format!("{k} :"))
        })
        .count();
    hits >= 2 || t.contains("\"parserOptions\"") || t.contains("parserOptions:")
}

impl Eslintrc {
    /// Count sections/rules in an ESLint config. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            rule_entries: 0,
            bool_entries: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("//") || tr.starts_with('#') || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            // Mid-line JSON objects may hold nested `key: severity` pairs —
            // scan each `{`-/`}`-joined segment separately.
            for part in tr.split(['{', '}', ',']) {
                let p = part.trim();
                if is_key_line(p) {
                    let v = value_token(p);
                    if SEVERITIES.contains(&v) || v.starts_with('[') {
                        c.rule_entries += 1;
                    }
                }
            }
            if !is_key_line(tr) {
                continue;
            }
            c.keys += 1;
            if value_token(tr) == "true" {
                c.bool_entries += 1;
            }
        }
        c.sections = SECTIONS
            .iter()
            .filter(|k| {
                t.contains(&format!("\"{k}\""))
                    || t.contains(&format!("{k}:"))
                    || t.contains(&format!("{k} :"))
            })
            .count();
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts_json() {
        let b = br#"{
  "root": true,
  "env": { "browser": true, "node": true, "es2022": true },
  "extends": ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
  "parserOptions": { "ecmaVersion": 2022, "sourceType": "module" },
  "plugins": ["import"],
  "rules": {
    "no-console": "warn",
    "eqeqeq": ["error", "always"],
    "@typescript-eslint/no-unused-vars": "error",
    "semi": 2
  },
  "overrides": [
    { "files": ["*.test.ts"], "rules": { "no-undef": "off" } }
  ]
}"#;
        assert!(detect(b));
        let c = Eslintrc::parse(b).unwrap();
        assert_eq!(c.rule_entries, 5);
        assert_eq!(c.bool_entries, 1);
        assert!(c.sections >= 6);
        assert!(c.keys >= 10);
    }

    #[test]
    fn detects_yaml() {
        let b = b"root: true\nenv:\n  node: true\nextends:\n  - eslint:recommended\nrules:\n  no-console: warn\n  eqeqeq: error\n";
        assert!(detect(b));
        let c = Eslintrc::parse(b).unwrap();
        assert_eq!(c.rule_entries, 2);
        assert_eq!(c.sections, 4);
    }

    #[test]
    fn rejects_package_json() {
        assert!(!detect(br#"{"name": "x", "version": "1"}"#));
        assert!(Eslintrc::parse(b"a: 1\n").is_none());
    }
}

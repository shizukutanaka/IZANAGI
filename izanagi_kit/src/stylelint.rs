//! Stylelint config (`.stylelintrc.json`/`.stylelintrc.yml`) parser.
//!
//! Detects `extends`/`plugins`/`rules`/`customSyntax`/`report*`/
//! `ignoreFiles` keys and counts rule entries (dash-named keys like
//! `declaration-block-no-*`, `selector-*`, `no-*`) plus `extends`/`plugins`
//! references.
//!
//! ```
//! let b = br#"{"extends": "stylelint-config-standard", "rules": {"declaration-block-no-duplicate-properties": true, "color-hex-length": "short"}, "customSyntax": "postcss-scss"}"#;
//! assert!(izanagi_kit::stylelint::detect(b));
//! let c = izanagi_kit::stylelint::Stylelint::parse(b).unwrap();
//! assert_eq!(c.rule_entries, 2);
//! ```

/// Parsed stylelint config summary.
#[derive(Debug, Clone)]
pub struct Stylelint {
    /// Key lines.
    pub keys: usize,
    /// Known top-level sections present.
    pub sections: usize,
    /// Rule entries (dash-named keys or severity/`[`-array values).
    pub rule_entries: usize,
    /// `extends`/`plugins` string or array references.
    pub refs: usize,
    /// `//`/`#`/`/*` comment lines.
    pub comments: usize,
}

/// Known top-level keys.
const SECTIONS: &[&str] = &[
    "extends",
    "plugins",
    "rules",
    "customSyntax",
    "processors",
    "reportDescriptionlessDisables",
    "reportInvalidScopeDisables",
    "reportNeedlessDisables",
    "ignoreFiles",
    "ignoreDisables",
    "defaultSeverity",
    "codeProcessors",
    "overrides",
    "cache",
    "fix",
    "quiet",
    "allowEmptyInput",
];

fn key_of(tr: &str) -> Option<(&str, &str)> {
    let t = tr
        .trim_start_matches('-')
        .trim_start()
        .trim_start_matches('"')
        .trim_start_matches('\'');
    let colon = t.find(':')?;
    if colon == 0 || colon > 90 {
        return None;
    }
    let key = t[..colon]
        .trim_end_matches('"')
        .trim_end_matches('\'')
        .trim_end();
    if key
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | '@' | '$'))
    {
        Some((key, t[colon + 1..].trim_start()))
    } else {
        None
    }
}

/// Detect a `.stylelintrc`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if t.contains("stylelint-config") || t.contains("customSyntax") || t.contains("stylelint") {
        return true;
    }
    let hits = SECTIONS
        .iter()
        .filter(|k| t.contains(&format!("\"{k}\"")) || t.contains(&format!("{k}:")))
        .count();
    hits >= 2
}

impl Stylelint {
    /// Count sections/rules in a stylelint config. Returns `None` when the
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
            refs: 0,
            comments: 0,
        };
        c.sections = SECTIONS
            .iter()
            .filter(|k| t.contains(&format!("\"{k}\"")) || t.contains(&format!("{k}:")))
            .count();
        let mut in_rules = false;
        let mut rules_indent: Option<usize> = None;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() || tr == "}" || tr == "{" || tr == "]" {
                continue;
            }
            if tr.starts_with("//") || tr.starts_with('#') || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            let indent = l.len() - tr.len();
            // Inline JSON (`"rules": {"a": v, "b": v}`) nests several pairs on
            // one line — evaluate each `{`-/`}`-joined segment separately.
            for part in tr.split(['{', '}', ',']) {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                if let Some((key, _)) = key_of(p) {
                    if key == "rules" {
                        in_rules = true;
                        rules_indent = Some(indent);
                    } else if in_rules
                        && rules_indent.is_some_and(|ri| indent <= ri)
                        && SECTIONS.contains(&key)
                    {
                        in_rules = false;
                        rules_indent = None;
                    }
                    c.keys += 1;
                    if key == "extends" || key == "plugins" {
                        c.refs += 1;
                    }
                    if in_rules && key != "rules" {
                        c.rule_entries += 1;
                    }
                } else if in_rules && p.starts_with('-') {
                    c.rule_entries += 1;
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
    fn detects_and_counts_json() {
        let b = br#"{
  "extends": ["stylelint-config-standard", "stylelint-config-recess-order"],
  "plugins": ["stylelint-order"],
  "customSyntax": "postcss-scss",
  "rules": {
    "declaration-block-no-duplicate-properties": true,
    "color-hex-length": "short",
    "selector-class-pattern": null,
    "order/order": "always"
  },
  "ignoreFiles": ["dist/**"]
}"#;
        assert!(detect(b));
        let c = Stylelint::parse(b).unwrap();
        assert_eq!(c.rule_entries, 4);
        assert_eq!(c.refs, 2);
        assert!(c.sections >= 4);
    }

    #[test]
    fn detects_yaml() {
        let b = b"extends: stylelint-config-standard\nrules:\n  color-hex-length: short\n  no-descending-specificity: true\n";
        assert!(detect(b));
        let c = Stylelint::parse(b).unwrap();
        assert_eq!(c.rule_entries, 2);
        assert_eq!(c.refs, 1);
    }

    #[test]
    fn rejects_pkg_json() {
        assert!(!detect(br#"{"name": "x", "scripts": {}}"#));
        assert!(Stylelint::parse(b"a: b\n").is_none());
    }
}

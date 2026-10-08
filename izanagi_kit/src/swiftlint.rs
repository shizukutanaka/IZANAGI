//! SwiftLint `.swiftlint.yml` census.
//!
//! `.swiftlint.yml` is YAML: `disabled_rules`/`opt_in_rules`/
//! `only_rules`/`analyzer_rules`/`included`/`excluded` lists,
//! `reporter`/`severity`/`use_nested_configs`/`baseline` scalars,
//! `custom_rules:` entries (`name:`/`regex:`/`message:`/
//! `match_kinds:`/`severity:`), and per-rule blocks
//! (`line_length:`/`identifier_name:`/`nesting:`…) with scalar or
//! nested config.
//!
//! ```rust
//! let c = izanagi_kit::swiftlint::Swiftlint::parse(b"disabled_rules:\n  - todo\n  - line_length\nopt_in_rules:\n  - empty_count\n").unwrap();
//! assert_eq!(c.listitems, 3);
//! ```

/// `.swiftlint.yml` census.
#[derive(Debug, Clone)]
pub struct Swiftlint {
    /// Scalar `key: value` settings (nested rule options included).
    pub settings: usize,
    /// Per-rule configuration blocks/scalars at top level.
    pub rules: usize,
    /// `- ` list items (rule lists, included/excluded paths…).
    pub listitems: usize,
    /// Named entries inside `custom_rules:`.
    pub custom_rules: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "disabled_rules",
    "opt_in_rules",
    "only_rules",
    "analyzer_rules",
    "whitelist_rules",
    "included",
    "excluded",
    "reporter",
    "severity",
    "custom_rules",
    "use_nested_configs",
    "baseline",
    "write_baseline",
    "indentation",
    "excluded_paths",
    "inclusive_source_paths",
];

const DETECT_KEYS: &[&str] = &[
    "disabled_rules",
    "opt_in_rules",
    "only_rules",
    "analyzer_rules",
    "custom_rules",
    "included",
    "excluded",
    "reporter",
    "use_nested_configs",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a `.swiftlint.yml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines()
        .filter(|l| {
            let s = l.trim();
            s.split(':')
                .next()
                .map(|k| DETECT_KEYS.contains(&k.trim()))
                .unwrap_or(false)
        })
        .count()
        >= 2
}

impl Swiftlint {
    /// Parse a `.swiftlint.yml` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            settings: 0,
            rules: 0,
            listitems: 0,
            custom_rules: 0,
            comments: 0,
        };
        let mut in_custom = false;
        let mut rule_indent = 0usize;
        let mut in_rule = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let ind = l.len() - l.trim_start().len();
            if ind == 0 {
                in_custom = false;
                in_rule = false;
            }
            if let Some((k, v)) = s.split_once(':') {
                let k = k.trim();
                let v = v.trim();
                if ind == 0 {
                    if k == "custom_rules" {
                        in_custom = true;
                        c.settings += 1;
                    } else if TOP_KEYS.contains(&k) {
                        c.settings += 1;
                    } else {
                        c.rules += 1;
                        if v.is_empty() {
                            in_rule = true;
                            rule_indent = ind;
                        }
                    }
                } else if in_custom && ind == 2 {
                    c.custom_rules += 1;
                } else if in_rule && ind <= rule_indent {
                    in_rule = false;
                    c.settings += 1;
                } else {
                    c.settings += 1;
                }
                continue;
            }
            if s.strip_prefix("- ").is_some() || s == "-" {
                c.listitems += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_swiftlint() {
        let b = concat!(
            "# swiftlint\n",
            "disabled_rules:\n",
            "  - todo\n",
            "  - line_length\n",
            "opt_in_rules:\n",
            "  - empty_count\n",
            "only_rules:\n",
            "  - force_cast\n",
            "included:\n",
            "  - Sources\n",
            "excluded:\n",
            "  - Tests\n",
            "reporter: xcode\n",
            "severity: warning\n",
            "custom_rules:\n",
            "  pirates_beat_ninjas:\n",
            "    name: Pirates\n",
            "    regex: ninja\n",
            "line_length: 120\n",
            "identifier_name:\n",
            "  min_length: 4\n",
            "nesting:\n",
            "  type_level: 2\n",
        );
        let c = Swiftlint::parse(b.as_bytes()).unwrap();
        assert_eq!(c.listitems, 6);
        assert_eq!(c.custom_rules, 1);
        assert_eq!(c.rules, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Swiftlint::parse(b"foo: bar\nbaz: qux").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

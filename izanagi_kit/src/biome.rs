//! Biome `biome.json` parser.
//!
//! Detects the `$schema: https://biomejs.dev/…` URL or the tool's
//! characteristic top-level objects (`organizeImports`/`linter`/`formatter`/
//! `javascript`/`json`/`css`/`graphql`/`assist`/`files`/`vcs`/`overrides`),
//! then counts sections, linter rule groups, and `//`-style comments.
//!
//! ```
//! let b = br#"{
//!   "$schema": "https://biomejs.dev/schemas/2/biome.json",
//!   "organizeImports": { "enabled": true },
//!   "linter": { "enabled": true, "rules": { "recommended": true, "suspicious": { "noExplicitAny": "error" } } },
//!   "formatter": { "indentWidth": 2 }
//! }"#;
//! assert!(izanagi_kit::biome::detect(b));
//! let c = izanagi_kit::biome::Biome::parse(b).unwrap();
//! assert_eq!(c.rule_groups, 3);
//! ```

/// Parsed biome.json summary.
#[derive(Debug, Clone)]
pub struct Biome {
    /// Key lines (`"key":`).
    pub keys: usize,
    /// Known top-level sections present.
    pub sections: usize,
    /// Linter rule groups/individual rules keyed inside `rules` (`a11y`/`complexity`/`correctness`/`nursery`/`performance`/`security`/`style`/`suspicious`/`recommended` or `no*`/`use*` names).
    pub rule_groups: usize,
    /// `//`/`#`/`/*` comment lines (JSONC).
    pub comments: usize,
}

/// Known top-level objects.
const SECTIONS: &[&str] = &[
    "$schema",
    "organizeImports",
    "linter",
    "formatter",
    "javascript",
    "json",
    "css",
    "graphql",
    "html",
    "assist",
    "files",
    "vcs",
    "overrides",
    "extends",
    "plugins",
    "grit",
];

/// Linter rule group keys plus individual rule-name prefixes.
const RULE_GROUPS: &[&str] = &[
    "a11y",
    "complexity",
    "correctness",
    "nursery",
    "performance",
    "security",
    "style",
    "suspicious",
    "recommended",
    "useImportRestrictions",
];

/// Detect a `biome.json`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("biomejs.dev")
        || t.contains("\"organizeImports\"")
        || t.contains("\"assist\"")
        || (t.contains("\"linter\"")
            && (t.contains("\"javascript\"") || t.contains("\"formatter\"")))
}

impl Biome {
    /// Count sections/rule groups in a biome.json. Returns `None` when the
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
            rule_groups: 0,
            comments: 0,
        };
        c.sections = SECTIONS
            .iter()
            .filter(|k| t.contains(&format!("\"{k}\"")))
            .count();
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with('#') || tr.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('"') && tr.contains("\":") {
                c.keys += 1;
            }
        }
        for g in RULE_GROUPS {
            c.rule_groups += t.matches(&format!("\"{g}\"",)).count();
        }
        c.rule_groups += t.matches("\"no").count() + t.matches("\"use").count();
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = br#"{
  "$schema": "https://biomejs.dev/schemas/2/biome.json",
  "vcs": { "enabled": true, "clientKind": "git" },
  "organizeImports": { "enabled": true },
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true,
      "suspicious": { "noExplicitAny": "error" },
      "style": { "useConst": "error", "useTemplate": "warn" },
      "correctness": { "noUndeclaredVariables": "error" }
    }
  },
  "formatter": { "indentWidth": 2, "lineWidth": 100 },
  "javascript": { "formatter": { "quoteStyle": "single" } }
}"#;
        assert!(detect(b));
        let c = Biome::parse(b).unwrap();
        assert!(c.sections >= 5);
        assert!(c.rule_groups >= 7);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(br#"{"name": "x", "formatter": {}}"#));
        assert!(Biome::parse(b"a: 1\n").is_none());
    }
}

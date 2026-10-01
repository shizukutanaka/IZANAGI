//! Prettier config (`.prettierrc`/`.prettierrc.json`/`.prettierrc.yml`) parser.
//!
//! Detects the characteristic option names (`printWidth`/`tabWidth`/`semi`/
//! `singleQuote`/`trailingComma`/`endOfLine`/`arrowParens`/`overrides` …) in
//! JSON or YAML form and counts how many option categories are set.
//!
//! ```
//! let b = br#"{"printWidth": 100, "tabWidth": 2, "semi": false, "singleQuote": true, "trailingComma": "es5", "arrowParens": "always"}"#;
//! assert!(izanagi_kit::prettier::detect(b));
//! let c = izanagi_kit::prettier::Prettier::parse(b).unwrap();
//! assert_eq!(c.keys, 6);
//! ```

/// Parsed Prettier config summary.
#[derive(Debug, Clone)]
pub struct Prettier {
    /// Recognized option keys present.
    pub keys: usize,
    /// Boolean-valued options (`semi`/`singleQuote`/`bracketSpacing`/`jsxSingleQuote`/`useTabs`/…).
    pub bool_options: usize,
    /// String-valued options (`trailingComma`/`endOfLine`/`arrowParens`/`proseWrap`/`quoteProps`/`parser`/…).
    pub string_options: usize,
    /// `overrides` blocks present.
    pub overrides: usize,
    /// `//`/`#`/`/*` comment lines.
    pub comments: usize,
}

/// Recognized option names.
const OPTIONS: &[&str] = &[
    "printWidth",
    "tabWidth",
    "useTabs",
    "semi",
    "singleQuote",
    "quoteProps",
    "jsxSingleQuote",
    "trailingComma",
    "bracketSpacing",
    "bracketSameLine",
    "jsxBracketSameLine",
    "arrowParens",
    "proseWrap",
    "htmlWhitespaceSensitivity",
    "vueIndentScriptAndStyle",
    "endOfLine",
    "embeddedLanguageFormatting",
    "singleAttributePerLine",
    "experimentalTernaries",
    "objectWrap",
    "insertPragma",
    "requirePragma",
    "check",
    "parser",
    "filepath",
    "plugins",
    "pluginSearchDirs",
    "overrides",
    "rangeStart",
    "rangeEnd",
    "cursorOffset",
];

/// Options that conventionally take booleans.
const BOOL_OPTIONS: &[&str] = &[
    "semi",
    "singleQuote",
    "jsxSingleQuote",
    "useTabs",
    "bracketSpacing",
    "bracketSameLine",
    "jsxBracketSameLine",
    "vueIndentScriptAndStyle",
    "singleAttributePerLine",
    "experimentalTernaries",
    "insertPragma",
    "requirePragma",
    "check",
    "pluginSearchDirs",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\"")) || t.contains(&format!("{k}:")) || t.contains(&format!("{k} :"))
}

/// Detect a `.prettierrc`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = OPTIONS.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Prettier {
    /// Count option categories in a Prettier config. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            bool_options: 0,
            string_options: 0,
            overrides: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with('#') || tr.starts_with("/*") {
                c.comments += 1;
            }
        }
        const NUMERIC: &[&str] = &[
            "printWidth",
            "tabWidth",
            "rangeStart",
            "rangeEnd",
            "cursorOffset",
        ];
        const SKIP: &[&str] = &["plugins", "overrides", "pluginSearchDirs"];
        for k in OPTIONS {
            if !key_present(t, k) {
                continue;
            }
            c.keys += 1;
            if BOOL_OPTIONS.contains(k) {
                c.bool_options += 1;
            } else if !NUMERIC.contains(k) && !SKIP.contains(k) {
                c.string_options += 1;
            }
        }
        c.overrides = t.matches("\"overrides\"").count()
            + t.lines()
                .filter(|l| l.trim() == "overrides:" || l.trim() == "overrides :")
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
  "printWidth": 100,
  "tabWidth": 4,
  "useTabs": false,
  "semi": true,
  "singleQuote": true,
  "trailingComma": "all",
  "bracketSpacing": false,
  "arrowParens": "avoid",
  "endOfLine": "lf",
  "overrides": [
    { "files": "*.md", "options": { "proseWrap": "always" } }
  ]
}"#;
        assert!(detect(b));
        let c = Prettier::parse(b).unwrap();
        assert_eq!(c.keys, 11);
        assert_eq!(c.bool_options, 4);
        assert_eq!(c.overrides, 1);
    }

    #[test]
    fn detects_yaml() {
        let b = b"printWidth: 120\nsemi: false\nsingleQuote: true\ntrailingComma: none\n";
        assert!(detect(b));
        let c = Prettier::parse(b).unwrap();
        assert_eq!(c.keys, 4);
        assert_eq!(c.bool_options, 2);
    }

    #[test]
    fn rejects_random_json() {
        assert!(!detect(br#"{"name": "x", "version": "1"}"#));
        assert!(Prettier::parse(b"a: 1\n").is_none());
    }
}

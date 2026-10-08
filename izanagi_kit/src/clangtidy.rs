//! ClangTidy `.clang-tidy` census.
//!
//! `.clang-tidy` is YAML: `Checks:` comma-glob list (`-` prefix disables),
//! `WarningsAsErrors:`, `HeaderFilterRegex:`, `FormatStyle:`,
//! `CheckOptions:` block of `- key:`/`value:` pairs, plus `User:`/
//! `UseColor:`/`SystemHeaders:`/`InheritParentConfig:`/
//! `HeaderFileExtensions:`/`ImplementationFileExtensions:`/
//! `ExcludeHeaderFilterRegex:`.
//!
//! ```rust
//! let c = izanagi_kit::clangtidy::ClangTidy::parse(b"Checks: '-*,readability-*'\nWarningsAsErrors: ''\n").unwrap();
//! assert_eq!(c.checks_disabled, 1);
//! ```

/// `.clang-tidy` census.
#[derive(Debug, Clone)]
pub struct ClangTidy {
    /// Enabled check globs in `Checks:` (entries not starting with `-`).
    pub checks_enabled: usize,
    /// Disabled check globs in `Checks:` (entries starting with `-`).
    pub checks_disabled: usize,
    /// `- key:` entries inside `CheckOptions:`.
    pub options: usize,
    /// Other `key: value` settings.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "Checks",
    "CheckOptions",
    "WarningsAsErrors",
    "HeaderFilterRegex",
    "ExcludeHeaderFilterRegex",
    "FormatStyle",
    "CheckColor",
    "User",
    "UseColor",
    "SystemHeaders",
    "InheritParentConfig",
    "HeaderFileExtensions",
    "ImplementationFileExtensions",
    "CheckGlob",
];

fn count_globs(v: &str, c: &mut ClangTidy) {
    for tok in v.split(',') {
        let tok = tok.trim().trim_matches('"').trim_matches('\'');
        if tok.is_empty() {
            continue;
        }
        if tok.starts_with('-') {
            c.checks_disabled += 1;
        } else {
            c.checks_enabled += 1;
        }
    }
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a `.clang-tidy` file.
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
                .map(|k| KEYS.contains(&k.trim()))
                .unwrap_or(false)
        })
        .count()
        >= 2
        && t.contains("Checks")
}

impl ClangTidy {
    /// Parse a `.clang-tidy` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            checks_enabled: 0,
            checks_disabled: 0,
            options: 0,
            settings: 0,
            comments: 0,
        };
        let mut scope = "";
        let mut scope_indent = 0usize;
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
            if !scope.is_empty() && ind <= scope_indent && !s.starts_with('-') {
                scope = "";
            }
            match scope {
                "Checks" => {
                    if s.starts_with('-') || !s.contains(':') {
                        count_globs(s, &mut c);
                        continue;
                    }
                    scope = "";
                }
                "CheckOptions" => {
                    if let Some(rest) = s.strip_prefix("- ") {
                        if rest.starts_with("key:") || rest.starts_with("key :") {
                            c.options += 1;
                        }
                        continue;
                    }
                }
                _ => {}
            }
            if let Some(rest) = s.strip_prefix("- ") {
                if rest.starts_with("key:") {
                    c.options += 1;
                    continue;
                }
            }
            if let Some((k, v)) = s.split_once(':') {
                let k = k.trim();
                let v = v.trim();
                if k == "Checks" {
                    c.settings += 1;
                    if v.is_empty() || v.starts_with('>') || v.starts_with('|') {
                        scope = "Checks";
                        scope_indent = ind;
                    } else {
                        count_globs(v, &mut c);
                    }
                } else if k == "CheckOptions" {
                    c.settings += 1;
                    scope = "CheckOptions";
                    scope_indent = ind;
                } else if k == "WarningsAsErrors" {
                    c.settings += 1;
                    count_globs(v, &mut c);
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
    fn parses_clang_tidy() {
        let b = concat!(
            "# tidy\n",
            "Checks: '-*,bugprone-*,clang-analyzer-*,readability-*,-readability-magic-numbers'\n",
            "WarningsAsErrors: 'bugprone-*'\n",
            "HeaderFilterRegex: '^src/.*'\n",
            "FormatStyle: file\n",
            "CheckOptions:\n",
            "  - key: readability-identifier-naming.ClassCase\n",
            "    value: CamelCase\n",
            "  - key: readability-identifier-naming.FunctionCase\n",
            "    value: lower_case\n",
            "UseColor: true\n",
        );
        let c = ClangTidy::parse(b.as_bytes()).unwrap();
        assert_eq!(c.checks_disabled, 2);
        assert_eq!(c.checks_enabled, 4);
        assert_eq!(c.options, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(ClangTidy::parse(b"foo: bar\nbar: baz").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

//! flake8 `.flake8` / `setup.cfg` `[flake8]` / `tox.ini` parser.
//!
//! Detects flake8 config by `[flake8]` / `[flake8:local-plugins]`
//! sections plus `select`/`ignore`/`max-line-length`/`per-file-ignores`
//! option keys, and counts structure.
//!
//! ```
//! let b = b"[flake8]\nmax-line-length = 100\nselect = E,W,F\nextend-ignore = E203\nper-file-ignores =\n    __init__.py:F401\n";
//! assert!(izanagi_kit::flake8conf::detect(b));
//! let c = izanagi_kit::flake8conf::Flake8::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed .flake8 summary.
#[derive(Debug, Clone)]
pub struct Flake8 {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `[flake8]`/`[flake8:*]` section headers.
    pub sections: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Known option keys.
const KEYS: &[&str] = &[
    "select",
    "ignore",
    "extend-ignore",
    "extend-select",
    "enable-extensions",
    "require-plugins",
    "max-line-length",
    "max-complexity",
    "max-doc-length",
    "min-python-version",
    "per-file-ignores",
    "exclude",
    "extend-exclude",
    "filename",
    "stdin-display-name",
    "format",
    "quiet",
    "count",
    "statistics",
    "tee",
    "benchmark",
    "doctests",
    "docstring-convention",
    "inline-quotes",
    "multiline-quotes",
    "docstring-quotes",
    "avoid-escape",
    "verbose",
    "jobs",
    "isort-show-traceback",
    "noqa-require-all",
    "noqa-include-name",
    "additional-builtins",
    "ban-relative-imports",
    "builtins",
    "copyright-check",
    "copyright-author",
    "copyright-regexp",
    "copyright-min-file-size",
    "suppress-dummy-args",
    "format-string",
    "radon-max-c\u{63}",
    "max-declarations",
    "unused-arguments-ignore-abstract-functions",
    "unused-arguments-ignore-dunder",
    "unused-arguments-ignore-overload",
    "unused-arguments-ignore-stub-functions",
    "unused-arguments-ignore-variadic-names",
    "unused-arguments-lambda-to-def",
    "unused-arguments-nested-lambdas",
    "rst-roles",
    "rst-directives",
    "rst-substitutions",
    "max-function-body-length",
    "max-module-members",
    "max-local-variables",
    "max-methods",
    "max-module-expressions",
    "max-function-expressions",
    "max-family-size",
    "max-returns",
    "max-raises",
    "max-asserts",
    "max-try-except",
    "max-public-methods",
    "max-cognitive-score",
    "max-cognitive-complexity",
    "min-public-methods",
    "max-annotations-complexity",
    "max-attributes",
    "max-args",
    "max-branches",
    "max-complexity",
    "max-cognitive-score",
    "min-public-methods",
    "max-expression-usages",
    "max-variable-usages",
    "max-try-body-length",
    "max-if-for-else",
    "max-conditional-expressions",
    "max-return-values",
    "max-member-accesses",
    "max-local-variables",
    "dummy-variable-rgx",
    "class-attribute-order",
    "accept-names",
    "type-checking-strict",
    "type-checking-pydantic-enabled",
    "type-checking-fastapi-enabled",
    "type-checking-exempt-modules",
    "type-checking-strict",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a flake8 config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if t.contains("[flake8]") || t.contains("[flake8:") {
        return true;
    }
    let hits = KEYS.iter().filter(|k| key_present(t, k)).count();
    hits >= 4
}

impl Flake8 {
    /// Count categories. Returns `None` when the input does not look like
    /// a flake8 config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
            } else if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
            } else if tr.contains('=') || tr.contains(':') {
                c.assignments += 1;
            }
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[flake8]\nmax-line-length = 100\nselect = E,W,F\nextend-ignore = E203,W503\nexclude = .venv,build\nper-file-ignores =\n    __init__.py:F401\n[flake8:local-plugins]\nextension =\n    MC1 = mchess:Checker\n";
        assert!(detect(b));
        let c = Flake8::parse(b).unwrap();
        assert_eq!(c.sections, 2);
        assert!(c.keys >= 5);
        assert!(c.assignments >= 5);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[main]\nfoo = bar\n"));
        assert!(Flake8::parse(b"").is_none());
    }
}

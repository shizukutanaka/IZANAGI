//! Ruff `ruff.toml` / `[tool.ruff]` parser.
//!
//! Detects Ruff linter config by `[tool.ruff]`/`[tool.ruff.lint]`/
//! `[tool.ruff.format]`/`[tool.ruff.server]` tables or top-level
//! `select`/`ignore`/`line-length`/`target-version`/`extend-select`
//! keys, and counts structure.
//!
//! ```
//! let b = b"[tool.ruff]\nline-length = 100\ntarget-version = \"py311\"\n[tool.ruff.lint]\nselect = [\"E\", \"F\"]\nignore = [\"E501\"]\n[tool.ruff.format]\nquote-style = \"double\"\n";
//! assert!(izanagi_kit::ruffconf::detect(b));
//! let c = izanagi_kit::ruffconf::Ruff::parse(b).unwrap();
//! assert_eq!(c.sections, 3);
//! ```

/// Parsed ruff.toml summary.
#[derive(Debug, Clone)]
pub struct Ruff {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `[tool.ruff]`/`[tool.ruff.*]`/`[ruff]`/`[ruff.*]` tables.
    pub sections: usize,
    /// Lint rule-selection keys (`select`/`ignore`/`extend-*`/`fixable`/`unfixable`/`per-file-ignores`/`flake8-*`).
    pub lint_keys: usize,
    /// Top-level keys (`line-length`/`target-version`/`src`/`extend`/`respect-gitignore`/`required-version`/`builtin-*`).
    pub top_keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Table prefixes.
const SECTION_KEYS: &[&str] = &[
    "[tool.ruff]",
    "[tool.ruff.lint]",
    "[tool.ruff.format]",
    "[tool.ruff.server]",
    "[tool.ruff.lint.per-file-ignores]",
    "[ruff]",
    "[ruff.lint]",
    "[ruff.format]",
    "[ruff.server]",
];

/// Lint-selection keys.
const LINT_KEYS: &[&str] = &[
    "select",
    "ignore",
    "extend-select",
    "extend-ignore",
    "fixable",
    "unfixable",
    "per-file-ignores",
    "extend-per-file-ignores",
    "flake8-annotations",
    "flake8-bandit",
    "flake8-bugbear",
    "flake8-builtins",
    "flake8-comprehensions",
    "flake8-copyright",
    "flake8-datetimez",
    "flake8-debugger",
    "flake8-django",
    "flake8-docstrings",
    "flake8-errmsg",
    "flake8-executable",
    "flake8-future-annotations",
    "flake8-gettext",
    "flake8-implicit-str-concat",
    "flake8-import-conventions",
    "flake8-logging",
    "flake8-logging-format",
    "flake8-no-pep420",
    "flake8-pie",
    "flake8-print",
    "flake8-pyi",
    "flake8-pytest-style",
    "flake8-quotes",
    "flake8-raise",
    "flake8-return",
    "flake8-self",
    "flake8-slots",
    "flake8-simplify",
    "flake8-tidy-imports",
    "flake8-type-checking",
    "flake8-unused-arguments",
    "flake8-use-pathlib",
    "isort",
    "mccabe",
    "pep8-naming",
    "pycodestyle",
    "pydocstyle",
    "pyflakes",
    "pygrep-hooks",
    "pylint",
    "pyupgrade",
    "refurb",
    "ruff",
    "fix",
    "preview",
    "explicit-preview-rules",
    "unsafe-fixes",
    "dummy-variable-rgx",
    "external",
    "extend-safe-fixes",
    "extend-unsafe-fixes",
    "logger-objects",
    "namespace-packages",
    "task-tags",
    "typing-modules",
    "allowed-confusables",
    "disallowed-decorators",
    "ignore-init-module-imports",
];

/// Top-level table keys.
const TOP_KEYS: &[&str] = &[
    "line-length",
    "target-version",
    "sr\u{63}",
    "extend",
    "respect-gitignore",
    "required-version",
    "force-exclude",
    "include",
    "extend-include",
    "exclude",
    "extend-exclude",
    "builtins",
    "namespace-packages",
    "cache-dir",
    "output-format",
    "indent-width",
    "tab-size",
    "line-ending",
    "quote-style",
    "indent-style",
    "skip-magic-trailing-comma",
    "line-ending",
    "docstring-code-format",
    "docstring-code-line-length",
    "analyze-source",
    "type-checking-imports",
    "type-checking-explicit-type-annotations-required",
    "type-checking-strict",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Ruff config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let sec = SECTION_KEYS.iter().filter(|k| key_present(t, k)).count();
    let hits = TOP_KEYS
        .iter()
        .chain(LINT_KEYS.iter())
        .filter(|k| key_present(t, k))
        .count();
    sec >= 1 || hits >= 3
}

impl Ruff {
    /// Count categories. Returns `None` when the input does not look like
    /// a Ruff config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            lint_keys: 0,
            top_keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
            }
        }
        for k in LINT_KEYS {
            c.lint_keys += t.matches(k).count();
        }
        for k in TOP_KEYS {
            c.top_keys += t.matches(k).count();
        }
        c.keys = c.lint_keys + c.top_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[tool.ruff]\nline-length = 100\ntarget-version = \"py311\"\nexclude = [\".venv\"]\n[tool.ruff.lint]\nselect = [\"E\", \"F\", \"B\"]\nignore = [\"E501\"]\nextend-select = [\"D\"]\nfixable = [\"E\", \"F\"]\n[tool.ruff.format]\nquote-style = \"double\"\nindent-style = \"space\"\n";
        assert!(detect(b));
        let c = Ruff::parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert!(c.lint_keys >= 6);
        assert!(c.top_keys >= 5);
        assert_eq!(c.assignments, 9);
    }

    #[test]
    fn rejects_toml() {
        assert!(!detect(b"[project]\nname = \"x\"\n"));
        assert!(Ruff::parse(b"").is_none());
    }
}

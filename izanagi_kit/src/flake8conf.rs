//! Flake8 `.flake8` / `tox.ini` `[flake8]` の検出と構造カウント。
//!
//! `[flake8]`/`[flake8:local-plugins]` セクションと `max-line-length`/
//! `select`/`ignore`/`extend-ignore`/`per-file-ignores`/`exclude` 等を分類する。
//!
//! ```
//! let c = izanagi_kit::flake8conf::parse(
//!     b"[flake8]\nmax-line-length = 100\nextend-ignore = E203\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::flake8conf::detect(b"[flake8]\nignore = E501\n"));
//! ```

/// セクション名の既知パターン。
fn is_section(name: &str) -> bool {
    name == "flake8" || name.starts_with("flake8:")
}
/// 既知オプションキー。
const KEYS: &[&str] = &[
    "accept-encodings",
    "additional-builtins",
    "aggressive",
    "allowed-module-names",
    "allowed-names",
    "append-config",
    "application-import-names",
    "application-relative-directories",
    "assertive-snake-case",
    "assertive-test-case-naming",
    "ban-relative-imports",
    "builtins",
    "check-assertions",
    "class-order",
    "classmethod-decorators",
    "comma-safe-imports",
    "config",
    "count",
    "disable-noqa",
    "docstring-convention",
    "docstring-style",
    "enable-extensions",
    "exclude",
    "exclude-lines",
    "extend-default-ignore",
    "extend-default-select",
    "extend-ignore",
    "extend-immutable-calls",
    "extend-select",
    "extended-default-ignore",
    "extended-default-select",
    "filename",
    "format",
    "force-single-line",
    "from-first",
    "hang-closing",
    "ignore",
    "ignore-decorators",
    "ignore-names",
    "inline-quotes",
    "import-order-style",
    "indent-size",
    "isort-show-traceback",
    "jobs",
    "max-annotations-complexity",
    "max-cognitive-complexity",
    "max-complexity",
    "max-doc-length",
    "max-expression-complexity",
    "max-function-body-length",
    "max-function-params",
    "max-locals",
    "max-line-complexity",
    "max-line-length",
    "max-module-members",
    "max-local-variables",
    "max-nested-blocks",
    "max-module-name-length",
    "max-return-amount",
    "max-statements",
    "max-string-imports",
    "max-top-level-expressions",
    "max-tuple-return-length",
    "method-order",
    "min-name-length",
    "mock-symbols",
    "multiline-quotes",
    "no-accept-encodings",
    "no-explicit-stacklevel",
    "no-isort-config",
    "not-skip",
    "output-file",
    "per-file-ignores",
    "plugins",
    "pytest-fixture-naming-convention",
    "pytest-mark-no-parentheses",
    "pytest-parametrize-names-type",
    "pytest-parametrize-values-type",
    "pytest-parametrize-values-row-type",
    "pytest-signature-annotations",
    "require-param-type",
    "require-return-type",
    "rst-roles",
    "rst-directives",
    "rst-substitutions",
    "select",
    "show-source",
    "signature-mutators",
    "staticmethod-decorators",
    "statistics",
    "tee",
    "typename-mismatch",
    "verbose",
    "version",
    "whitelist",
];

/// Flake8 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[flake8*]` セクション。
    pub sections: usize,
    /// 既知オプション行。
    pub options: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が flake8 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut keys = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && is_section(&t[1..t.len() - 1]) {
            secs += 1;
        } else if t.find('=').is_some_and(|p| KEYS.contains(&t[..p].trim())) {
            keys += 1;
        }
    }
    secs >= 1 && keys >= 1
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            if is_section(&t[1..t.len() - 1]) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if KEYS.contains(&t[..pos].trim()) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[flake8]\nmax-line-length = 100\nselect = E,W,F\nextend-ignore = E203, W503\nper-file-ignores = __init__.py:F401\nexclude = .git,venv\nmax-complexity = 10\ninline-quotes = double\n\n[flake8:local-plugins]\nextension = MC1 = flake8_mccabe\n\n[pytest]\nx = 1\n";

    #[test]
    fn flake8conf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.options, 7);
        assert_eq!(c.misc, 3);
    }

    #[test]
    fn not_flake8() {
        assert!(!detect(b"[mypy]\npython_version = 3.12\n"));
        assert!(!detect(b"hello\n"));
    }
}

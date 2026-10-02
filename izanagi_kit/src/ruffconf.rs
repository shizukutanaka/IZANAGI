//! Ruff `ruff.toml` / `pyproject.toml` `[tool.ruff]` の検出と構造カウント。
//!
//! `[tool.ruff]`/`[tool.ruff.lint]`/`[tool.ruff.format]`/`[tool.ruff.server]`/
//! `[tool.ruff.*]` セクションと `select`/`ignore`/`line-length`/`target-version`/
//! `src`/`fix` 等の既知キーを TOML 風に分類する。
//!
//! ```
//! let c = izanagi_kit::ruffconf::parse(
//!     b"[tool.ruff]\nline-length = 100\n[tool.ruff.lint]\nselect = [\"E\",\"F\"]\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::ruffconf::detect(b"[tool.ruff]\nline-length = 88\n"));
//! ```

/// `[tool.ruff]` 配下の既知サブセクション名。
const SUBSECTIONS: &[&str] = &[
    "tool.ruff",
    "tool.ruff.analysis",
    "tool.ruff.builtins",
    "tool.ruff.format",
    "tool.ruff.lint",
    "tool.ruff.lint.flake8-annotations",
    "tool.ruff.lint.flake8-bandit",
    "tool.ruff.lint.flake8-boolean-trap",
    "tool.ruff.lint.flake8-bugbear",
    "tool.ruff.lint.flake8-builtins",
    "tool.ruff.lint.flake8-comprehensions",
    "tool.ruff.lint.flake8-copyright",
    "tool.ruff.lint.flake8-datetimez",
    "tool.ruff.lint.flake8-django",
    "tool.ruff.lint.flake8-docstrings",
    "tool.ruff.lint.flake8-errmsg",
    "tool.ruff.lint.flake8-future-annotations",
    "tool.ruff.lint.flake8-gettext",
    "tool.ruff.lint.flake8-implicit-str-concat",
    "tool.ruff.lint.flake8-import-conventions",
    "tool.ruff.lint.flake8-logging",
    "tool.ruff.lint.flake8-logging-format",
    "tool.ruff.lint.flake8-no-pep420",
    "tool.ruff.lint.flake8-pie",
    "tool.ruff.lint.flake8-print",
    "tool.ruff.lint.flake8-pyi",
    "tool.ruff.lint.flake8-quotes",
    "tool.ruff.lint.flake8-raise",
    "tool.ruff.lint.flake8-return",
    "tool.ruff.lint.flake8-self",
    "tool.ruff.lint.flake8-simplify",
    "tool.ruff.lint.flake8-slots",
    "tool.ruff.lint.flake8-tidy-imports",
    "tool.ruff.lint.flake8-type-checking",
    "tool.ruff.lint.flake8-unused-arguments",
    "tool.ruff.lint.flake8-use-pathlib",
    "tool.ruff.lint.flake8-debugger",
    "tool.ruff.lint.flynt",
    "tool.ruff.lint.isort",
    "tool.ruff.lint.mccabe",
    "tool.ruff.lint.pep8-naming",
    "tool.ruff.lint.per-file-ignores",
    "tool.ruff.lint.pycodestyle",
    "tool.ruff.lint.pydocstyle",
    "tool.ruff.lint.pydoclint",
    "tool.ruff.lint.pyflakes",
    "tool.ruff.lint.pygrep-hooks",
    "tool.ruff.lint.pylint",
    "tool.ruff.lint.pyupgrade",
    "tool.ruff.lint.refurb",
    "tool.ruff.lint.ruff",
    "tool.ruff.server",
];
/// `[tool.ruff]` 直下または ruff.toml 直下の既知キー。
const KEYS: &[&str] = &[
    "analyze",
    "builtins",
    "cache-dir",
    "detect",
    "dummy-variable-rgx",
    "exclude",
    "explicit-preview-rules",
    "extend",
    "extend-exclude",
    "extend-include",
    "extend-safe-fixes",
    "extend-unsafe-fixes",
    "fix",
    "fix-only",
    "force-exclude",
    "format",
    "include",
    "indent-width",
    "lint",
    "line-length",
    "namespace-packages",
    "output-format",
    "preview",
    "required-version",
    "respect-gitignore",
    "select",
    "show-fixes",
    "show-source",
    "src",
    "target-version",
    "unsafe-fixes",
];
/// `[tool.ruff.lint]` 配下の既知キー。
const LINT_KEYS: &[&str] = &[
    "allowed-confusables",
    "allowed-names",
    "async-type-checking-timeout",
    "dummy-variable-rgx",
    "explicit-preview-rules",
    "extend-fixable",
    "extend-ignores",
    "extend-ignore-names",
    "extend-safe-fixes",
    "extend-select",
    "extend-unfixable",
    "extend-unsafe-fixes",
    "fixable",
    "flake8-",
    "ignore",
    "ignore-init-module-imports",
    "logger-objects",
    "mark-octal",
    "per-file-ignores",
    "pydocstyle",
    "pydoclint",
    "pyflakes",
    "pylint",
    "select",
    "task-tags",
    "typing-modules",
    "unfixable",
    "unsafe-fixes",
];

/// Ruff 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[tool.ruff*]` セクション。
    pub sections: usize,
    /// 既知キー行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// キーが既知かどうか(ruff.toml 直下は ruff のコンテキストとして扱う)。
fn key_known(key: &str, in_lint: bool, in_ruff: bool) -> bool {
    if in_lint {
        LINT_KEYS
            .iter()
            .any(|k| key == *k || key.starts_with(&format!("{k}.")))
            || key.starts_with("flake8-")
    } else if in_ruff {
        KEYS.contains(&key)
            || key.starts_with("lint.")
            || key.starts_with("format.")
            || key.starts_with("analysis.")
    } else {
        false
    }
}

/// b が ruff.toml/pyproject `[tool.ruff]` かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines().any(|l| {
        let t = l.trim();
        (t.starts_with('[') && t.ends_with(']'))
            && SUBSECTIONS.iter().any(|s| t[1..t.len() - 1] == **s)
    })
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
    let mut in_lint = false;
    let mut in_ruff = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            let name = &t[1..t.len() - 1];
            if SUBSECTIONS.contains(&name) {
                c.sections += 1;
                in_lint = name.starts_with("tool.ruff.lint");
                in_ruff = !in_lint;
            } else {
                c.misc += 1;
                in_lint = false;
                in_ruff = false;
            }
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        let key = t[..pos].trim();
        if key_known(key, in_lint, in_ruff) {
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

    const SAMPLE: &[u8] = b"# ruff config\n[tool.ruff]\ntarget-version = \"py312\"\nline-length = 100\nsrc = [\"src\", \"tests\"]\nextend-exclude = [\"migrations\"]\n\n[tool.ruff.lint]\nselect = [\"E\", \"F\", \"I\", \"UP\", \"B\"]\nignore = [\"E501\"]\nextend-select = [\"RUF100\"]\nfixable = [\"ALL\"]\n\n[tool.ruff.lint.pydocstyle]\nconvention = \"google\"\n\n[tool.ruff.format]\nquote-style = \"double\"\n\n[project]\nname = \"x\"\n";

    #[test]
    fn ruffconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.options, 8);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 4);
    }

    #[test]
    fn not_ruff() {
        assert!(!detect(b"[tool.black]\nline-length = 100\n"));
        assert!(!detect(b"hello\n"));
    }
}

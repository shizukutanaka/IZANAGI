//! `.pylintrc` (INI) census.
//!
//! `[MASTER]`/`[MESSAGES CONTROL]`/`[REPORTS]`/`[FORMAT]`/`[BASIC]`/
//! `[DESIGN]`/`[TYPECHECK]`/`[SIMILARITIES]`/`[MISCELLANEOUS]`/
//! `[IMPORTS]`/`[CLASSES]`/`[VARIABLES]`/`[EXCEPTIONS]`/`[REFACTORING]`/
//! `[LOGGING]`/`[SPELLING]`/`[STRING]`/`[ELIF]`/`[MAIN]` +
//! `enable=`/`disable=` comma-separated message lists +
//! `max-*`/`good-names`/`ignore*`/`attr-rgx` settings.
//!
//! ```rust
//! let p = "[MASTER]\njobs=4\n[MESSAGES CONTROL]\ndisable=C0114,C0115\n[FORMAT]\nmax-line-length=120\n";
//! let c = izanagi_kit::pylintrc::Pylintrc::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.disabled, 2);
//! ```

/// .pylintrc census.
#[derive(Debug, Clone)]
pub struct Pylintrc {
    /// `[SECTION]` headers.
    pub sections: usize,
    /// `key=value`/`key=` entries.
    pub settings: usize,
    /// Message ids inside `disable=`.
    pub disabled: usize,
    /// Message ids inside `enable=`.
    pub enabled: usize,
    /// Recognised pylint option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "jobs",
    "suggestion-mode",
    "unsafe-load-any-extension",
    "extension-pkg-allow-list",
    "extension-pkg-whitelist",
    "load-plugins",
    "init-hook",
    "ignore",
    "ignore-paths",
    "ignore-patterns",
    "persistent",
    "py-version",
    "recursive",
    "source-roots",
    "fail-on",
    "fail-under",
    "exit-zero",
    "from-stdin",
    "disable",
    "enable",
    "confidence",
    "evaluation",
    "score",
    "msg-template",
    "output-format",
    "reports",
    "max-line-length",
    "max-module-lines",
    "indent-string",
    "expected-line-ending-format",
    "single-line-if-stmt",
    "single-line-class-stmt",
    "no-docstring-rgx",
    "docstring-min-length",
    "class-rgx",
    "class-attribute-rgx",
    "function-rgx",
    "method-rgx",
    "attr-rgx",
    "argument-rgx",
    "variable-rgx",
    "const-rgx",
    "inlinevar-rgx",
    "module-rgx",
    "typevar-rgx",
    "typealias-rgx",
    "good-names",
    "good-names-rgxs",
    "bad-names",
    "bad-names-rgxs",
    "name-group",
    "include-naming-hint",
    "property-classes",
    "max-args",
    "max-locals",
    "max-returns",
    "max-branches",
    "max-statements",
    "max-parents",
    "min-public-methods",
    "max-public-methods",
    "max-attributes",
    "max-bool-expr",
    "max-nested-blocks",
    "max-complexity",
    "min-similarity-lines",
    "ignore-comments",
    "ignore-docstrings",
    "ignore-imports",
    "ignore-signatures",
    "ignored-module-names",
    "logging-modules",
    "logging-format-style",
    "notes",
    "notes-rgx",
    "analyse-fallback-blocks",
    "ignored-classes",
    "ignored-modules",
    "analyse-implemented-blocks",
    "exclude-protected",
    "contextmanager-decorators",
    "generated-members",
    "known-plain-types",
    "ignored-argument-names",
    "mixin-class-rgx",
    "special-members",
    "signature-mutators",
    "invalid-metaclass",
    "valid-metaclass-classmethod-first-arg",
    "missing-kwoa",
    "dummy-variables-rgx",
    "additional-builtins",
    "callbacks",
    "redefining-builtins-modules",
    "allowed-redefined-builtins",
    "unnecessary-lambda-assignment",
    "preferred-modules",
    "spelling-dict",
    "spelling-ignore-words",
    "spelling-private-dict-file",
    "spelling-store-unknown-words",
    "spelling-ignore-comment-directives",
    "check-quote-consistency",
    "check-str-concat-over-line-jumps",
    "preferred-modules",
    "allow-global-unused-variables",
    "allow-any-export-level",
    "unknown-option-value",
    "overgeneral-exceptions",
    "clear-cache-post-run",
    "init-import",
];

/// Whether the buffer looks like a .pylintrc.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[MESSAGES CONTROL]")
        || t.contains("[MASTER]")
        || t.contains("[FORMAT]") && t.contains("max-line-length")
        || t.contains("disable=")
            && (t.contains("C0") || t.contains("R0") || t.contains("W0") || t.contains("E0"))
        || t.contains("[SIMILARITIES]")
        || t.contains("[TYPECHECK]")
}

impl Pylintrc {
    /// Parse a .pylintrc into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            disabled: 0,
            enabled: 0,
            named: 0,
        };
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with(['#', ';']) {
                continue;
            }
            let s = l.trim();
            if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
                continue;
            }
            let Some(eq) = s.find('=') else {
                continue;
            };
            c.settings += 1;
            let key = s[..eq].trim();
            let val = s[eq + 1..].trim();
            if KEYS.contains(&key) {
                c.named += 1;
            }
            if key == "disable" {
                c.disabled += val.split(',').filter(|v| !v.trim().is_empty()).count();
            } else if key == "enable" {
                c.enabled += val.split(',').filter(|v| !v.trim().is_empty()).count();
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rc() {
        let b = concat!(
            "[MASTER]\n",
            "jobs=4\n",
            "persistent=yes\n",
            "load-plugins=\n",
            "    pylint.extensions.docparams,\n",
            "    pylint.extensions.typing\n",
            "init-hook=\n",
            "    import sys; sys.path.append('src')\n",
            "ignore-paths=\n",
            "    ^vendor/\n",
            "py-version=3.12\n",
            "recursive=yes\n",
            "[MESSAGES CONTROL]\n",
            "disable=C0114,C0115,C0116,R0903,R0913,W0613\n",
            "enable=useless-suppression,use-symbolic-message-instead\n",
            "confidence=HIGH\n",
            "[FORMAT]\n",
            "max-line-length=120\n",
            "indent-string='    '\n",
            "[BASIC]\n",
            "good-names=i,j,k,ex,Run,_\n",
            "function-rgx=(([a-z][a-z0-9_]{2,})|(_.*))$\n",
            "docstring-min-length=-1\n",
            "[DESIGN]\n",
            "max-args=7\n",
            "max-locals=20\n",
            "max-branches=15\n",
            "max-statements=60\n",
            "max-parents=7\n",
            "min-public-methods=0\n",
            "max-attributes=10\n",
            "[TYPECHECK]\n",
            "ignored-modules=numpy,scipy.*\n",
            "generated-members=numpy.*,torch.*\n",
            "[SIMILARITIES]\n",
            "min-similarity-lines=6\n",
            "ignore-comments=yes\n",
            "ignore-docstrings=yes\n",
            "ignore-imports=yes\n",
            "[MISCELLANEOUS]\n",
            "notes=FIXME,XXX,TODO\n",
            "[IMPORTS]\n",
            "preferred-modules=yaml:ruamel.yaml\n",
        );
        let c = Pylintrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 9);
        assert_eq!(c.settings, 30);
        assert_eq!(c.disabled, 6);
        assert_eq!(c.enabled, 2);
        assert!(c.named >= 25);
    }

    #[test]
    fn rejects_other() {
        assert!(Pylintrc::parse(b"[foo]\na = 1").is_none());
    }
}

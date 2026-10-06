//! Clippy `clippy.toml`/`clippy.toml` 形式の検出と構造カウント。
//!
//! `msrv`/`cognitive-complexity-threshold`/`disallowed-names`/
//! `disallowed-methods`/`disallowed-macros`/`disallowed-types`/
//! `too-many-arguments-threshold`/`type-complexity-threshold` 等の
//! Clippy リント調整オプション行を識別する。
//!
//! ```
//! let b = b"msrv = \"1.78.0\"\ncognitive-complexity-threshold = 30\ndisallowed-methods = [\"std::mem::forget\"]\ntoo-many-arguments-threshold = 7\n";
//! assert!(izanagi_kit::clippytoml::detect(b));
//! let c = izanagi_kit::clippytoml::ClippyToml::parse(b).unwrap();
//! assert_eq!(c.keys, 4);
//! ```

/// Parsed clippy.toml summary.
#[derive(Debug, Clone)]
pub struct ClippyToml {
    /// Recognized option keys.
    pub keys: usize,
    /// `key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Clippy option names.
const KEYS: &[&str] = &[
    "absolute-paths-allowed-crates",
    "absolute-paths-max-segments",
    "accept-comment-above-attributes",
    "accept-comment-above-statement",
    "allow-dbg-in-tests",
    "allow-expect-in-tests",
    "allow-print-in-tests",
    "allow-unwrap-in-tests",
    "allowed-duplicate-crates",
    "allowed-idents-below-min-chars",
    "allow-mixed-uninlined-format-args",
    "allowed-internal-lints",
    "associated-constants-in-lint-allow-list",
    "avoid-breaking-exported-api",
    "check-reorder-imports",
    "check-private-items",
    "cognitive-complexity-threshold",
    "comment-line-threshold",
    "disallowed-macros",
    "disallowed-methods",
    "disallowed-names",
    "disallowed-types",
    "doc-valid-idents",
    "donotcheck-derive-macro-serde",
    "enable-integer-modules",
    "enforced-import-renames",
    "enum-variant-name-threshold",
    "enum-variant-size-threshold",
    "escaped-string-threshold",
    "excessive-nesting-threshold",
    "future-breaking-threshold",
    "large-error-threshold",
    "literal-fragment-restriction",
    "literal-representation",
    "lower-case-acronyms-aggressive",
    "max-fn-params-bools",
    "max-struct-bools",
    "max-trait-bounds",
    "missing-docs-in-crate-items",
    "module-item-sorted",
    "msrv",
    "non-const-literal-constant-attrs",
    "pass-by-value-size-limit",
    "preserve-source-format",
    "pub-underscore-fields-behavior",
    "single-char-binding-names-threshold",
    "standard-macro-braces",
    "struct-field-name-threshold",
    "suppress-restriction-lint-in-const",
    "too-large-for-stack",
    "too-many-arguments-threshold",
    "too-many-lines-threshold",
    "trivial-copy-size-limit",
    "type-complexity-threshold",
    "unnecessary-box-size",
    "upper-case-acronyms-aggressive",
    "vec-box-size-threshold",
    "wait-for-diagnostic-from-comment",
    "warn-on-all-wildcard-imports",
    "wildcards-imports-allowed-item-kinds",
];

fn key_present(t: &str, k: &str) -> bool {
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.starts_with('#') && tr.split('=').next().is_some_and(|p| p.trim() == k)
    })
}

/// Detect a Clippy config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| key_present(t, k)).count() >= 2
}

impl ClippyToml {
    /// Count categories. Returns `None` when the input does not look like
    /// a Clippy config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
                if let Some(k) = tr.split('=').next() {
                    if KEYS.contains(&k.trim()) {
                        c.keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

/// Convenience wrapper around [`ClippyToml::parse`].
#[must_use]
pub fn parse(b: &[u8]) -> Option<ClippyToml> {
    ClippyToml::parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# lints\nmsrv = \"1.78.0\"\ncognitive-complexity-threshold = 30\ndisallowed-names = [\"toto\", \"foo\"]\ndisallowed-methods = [\"std::mem::forget\"]\ndisallowed-macros = [\"println\"]\ndisallowed-types = [\"std::collections::LinkedList\"]\ntoo-many-arguments-threshold = 7\ntype-complexity-threshold = 250\nwarn-on-all-wildcard-imports = true\n";
        assert!(detect(b));
        let c = ClippyToml::parse(b).unwrap();
        assert_eq!(c.keys, 9);
        assert_eq!(c.assignments, 9);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_generic_toml() {
        assert!(!detect(b"[profile.dev]\nopt-level = 0\n"));
        assert!(!detect(b"msr = \"x\"\n"));
    }

    #[test]
    fn rejects_comment_only_mentions() {
        assert!(!detect(b"# msrv = \"1.78\"\n# disallowed-names = []\n"));
    }

    #[test]
    fn empty_and_binary_inputs_do_not_panic() {
        assert!(!detect(b""));
        assert!(ClippyToml::parse(b"").is_none());
        let mut b = vec![0xFF, 0x00];
        b.extend_from_slice(b"msrv");
        assert!(!detect(&b));
    }
}

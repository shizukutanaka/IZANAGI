//! Ace editor mode file (`mode-*.js`) parser.
//!
//! Detects Ace mode sources by `ace.define`/`define(function(require,
//! exports, module)` wrappers with `oop.inherits`, `Mode`/`HighlightRules`/
//! `Tokenizer`/`TextHighlightRules` prototypes, `this.$highlightRules`/
//! `this.createTokenizer`/`getNextLineIndent` hooks and `exports.Mode`
//! assignments.
//!
//! ```
//! let b = b"ace.define(\"ace/mode/demo\", function(require, exports, module) {\nvar oop = require(\"../lib/oop\");\nvar Mode = require(\"./text\").Mode;\nvar DemoHighlightRules = function() {\n  this.$rules = { start: [ { token: \"keyword\", regex: \"if|else\" } ] };\n};\noop.inherits(Demo, Mode);\nexports.Mode = Demo;\n});\n";
//! assert!(izanagi_kit::acemode::detect(b));
//! let c = izanagi_kit::acemode::Acemode::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed ace mode summary.
#[derive(Debug, Clone)]
pub struct Acemode {
    /// Recognized token occurrences.
    pub keys: usize,
    /// Define/require keys (`ace.define`/`define(`/`require(`/`module.exports`/`exports.`/`ace/mode/`).
    pub define_keys: usize,
    /// Mode inheritance keys (`oop.inherits`/`Mode`/`HighlightRules`/`TextHighlightRules`/`DocCommentHighlightRules`/`Behaviour`/`FoldMode`/`Tokenizer`/`MatchingBraceOutdent`/`WorkerClient`/`CstyleBehaviour`/`getNextLineIndent`/`toggleCommentLines`/`getMatching`/`createWorker`/`createModeDelegates`/`transformAction`).
    pub mode_keys: usize,
    /// Rule/object keys (`this.$rules`/`this.$highlightRules`/`this.createTokenizer`/`token:`/`regex:`/`next:`/`push:`/`defaultToken`/`caseInsensitive`/`onMatch`/`start:`/`state:`/`"ace/mode/`).
    pub rule_keys: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

/// Define/require keys.
const DEFINE_KEYS: &[&str] = &[
    "ace.define",
    "define(function",
    "require(",
    "module.exports",
    "exports.Mode",
    "ace/mode/",
];

/// Mode inheritance keys.
const MODE_KEYS: &[&str] = &[
    "oop.inherits",
    ".Mode",
    "HighlightRules",
    "TextHighlightRules",
    "DocCommentHighlightRules",
    "FoldMode",
    "Tokenizer",
    "MatchingBraceOutdent",
    "WorkerClient",
    "CstyleBehaviour",
    "getNextLineIndent",
    "toggleCommentLines",
    "createWorker",
    "createModeDelegates",
    "transformAction",
];

/// Rule keys.
const RULE_KEYS: &[&str] = &[
    "$rules",
    "$highlightRules",
    "createTokenizer",
    "token:",
    "regex:",
    "next:",
    "push:",
    "defaultToken",
    "caseInsensitive",
    "onMatch",
    "state:",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["ace/mode/", "HighlightRules", "ace.define"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "ace.define",
    "ace/mode/",
    "HighlightRules",
    "oop.inherits",
    "$rules",
    "exports.Mode",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect an ace mode file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Acemode {
    /// Count categories in a mode file. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            define_keys: 0,
            mode_keys: 0,
            rule_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("/*") {
                c.comments += 1;
            }
        }
        for k in DEFINE_KEYS {
            c.define_keys += t.matches(k).count();
        }
        for k in MODE_KEYS {
            c.mode_keys += t.matches(k).count();
        }
        for k in RULE_KEYS {
            c.rule_keys += t.matches(k).count();
        }
        c.keys = c.define_keys + c.mode_keys + c.rule_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"// mode\nace.define(\"ace/mode/demo\", function(require, exports, module) {\nvar oop = require(\"../lib/oop\");\nvar Mode = require(\"./text\").Mode;\nvar DemoHighlightRules = function() {\n  this.$rules = {\n    start: [ { token: \"keyword\", regex: \"if|else\", next: \"start\" } ],\n  };\n};\nvar Demo = function() { Mode.call(this); this.HighlightRules = DemoHighlightRules; };\noop.inherits(Demo, Mode);\nDemo.prototype.getNextLineIndent = function(state, line, tab) { return tab; };\nexports.Mode = Demo;\n});\n";
        assert!(detect(b));
        let c = Acemode::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.define_keys >= 3);
        assert!(c.mode_keys >= 3);
        assert!(c.rule_keys >= 3);
        assert!(c.keys >= 9);
    }

    #[test]
    fn rejects_js() {
        assert!(!detect(b"var x = 1;\nfunction g() {}\n"));
        assert!(Acemode::parse(b"a = b\n").is_none());
    }
}

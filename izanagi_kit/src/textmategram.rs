//! TextMate grammar (`.tmLanguage`/`.tmLanguage.json`/`syntaxes/*.json`) parser.
//!
//! Detects TextMate grammars by their characteristic keys (`scopeName`/
//! `patterns`/`repository`/`begin`/`end`/`match`/`captures`/`beginCaptures`/
//! `endCaptures`/`include`/`name`/`contentName`/`applyEndPatternLast`/
//! `fileTypes`/`firstLineMatch`/`foldingStartMarker`/`foldingStopMarker`
//! …) in JSON or plist XML form.
//!
//! ```
//! let b = b"{\n  \"scopeName\": \"source.demo\",\n  \"patterns\": [\n    { \"begin\": \"\\\\bif\\\\b\", \"end\": \"\\\\bend\\\\b\", \"captures\": { \"0\": { \"name\": \"keyword\" } } },\n    { \"include\": \"#strings\" }\n  ],\n  \"repository\": { \"strings\": { \"match\": \"\\\".*\\\"\", \"name\": \"string\" } }\n}\n";
//! assert!(izanagi_kit::textmategram::detect(b));
//! let c = izanagi_kit::textmategram::Tmgram::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

use crate::textutil::strip_xml_comments;
/// Parsed TextMate grammar summary.
#[derive(Debug, Clone)]
pub struct Tmgram {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Scope keys (`"scopeName"`/`"name"`/`"contentName"`/`"scope"`).
    pub scope_keys: usize,
    /// Pattern keys (`"patterns"`/`"begin"`/`"end"`/`"match"`/`"include"`/`"applyEndPatternLast"`/`"while"`).
    pub pattern_keys: usize,
    /// Capture/repo keys (`"captures"`/`"beginCaptures"`/`"endCaptures"`/`"repository"`/`"fileTypes"`/`"firstLineMatch"`/`"foldingStartMarker"`/`"foldingStopMarker"`/`"foldingEndMarker"`).
    pub repo_keys: usize,
    /// `//`/`<!-- -->` comment lines.
    pub comments: usize,
}

/// Scope keys.
const SCOPE_KEYS: &[&str] = &[
    "\"scopeName\"",
    "\"contentName\"",
    "<key>scopeName</key>",
    "<key>contentName</key>",
];

/// Pattern keys.
const PATTERN_KEYS: &[&str] = &[
    "\"patterns\"",
    "\"begin\"",
    "\"end\"",
    "\"match\"",
    "\"include\"",
    "\"while\"",
    "\"applyEndPatternLast\"",
    "<key>patterns</key>",
    "<key>begin</key>",
    "<key>end</key>",
    "<key>match</key>",
    "<key>include</key>",
];

/// Capture/repo keys.
const REPO_KEYS: &[&str] = &[
    "\"captures\"",
    "\"beginCaptures\"",
    "\"endCaptures\"",
    "\"repository\"",
    "\"fileTypes\"",
    "\"firstLineMatch\"",
    "\"foldingStartMarker\"",
    "\"foldingStopMarker\"",
    "\"foldingEndMarker\"",
    "<key>captures</key>",
    "<key>beginCaptures</key>",
    "<key>endCaptures</key>",
    "<key>repository</key>",
    "<key>fileTypes</key>",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["scopeName", "patterns", "repository", "source."];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "\"scopeName\"",
    "<key>scopeName</key>",
    "\"patterns\"",
    "<key>patterns</key>",
    "\"repository\"",
    "<key>repository</key>",
    "\"begin\"",
    "\"end\"",
    "\"include\"",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a TextMate grammar file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_xml_comments(t);
    let hits = ALL.iter().filter(|k| key_present(&t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Tmgram {
    /// Count key categories in a TextMate grammar. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = strip_xml_comments(std::str::from_utf8(b).ok()?);
        let mut c = Self {
            keys: 0,
            scope_keys: 0,
            pattern_keys: 0,
            repo_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("<!--") {
                c.comments += 1;
            }
        }
        for k in SCOPE_KEYS {
            c.scope_keys += t.matches(k).count();
        }
        for k in PATTERN_KEYS {
            c.pattern_keys += t.matches(k).count();
        }
        for k in REPO_KEYS {
            c.repo_keys += t.matches(k).count();
        }
        c.keys = c.scope_keys + c.pattern_keys + c.repo_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"// grammar\n{\n  \"scopeName\": \"source.demo\",\n  \"fileTypes\": [\"demo\"],\n  \"patterns\": [\n    { \"begin\": \"if\", \"end\": \"fi\", \"beginCaptures\": { \"0\": { \"name\": \"k\" } }, \"endCaptures\": {} },\n    { \"match\": \"[a-z]+\", \"name\": \"ident\" },\n    { \"include\": \"#main\" }\n  ],\n  \"repository\": { \"main\": { \"patterns\": [] } }\n}\n";
        assert!(detect(b));
        let c = Tmgram::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.scope_keys >= 1);
        assert!(c.pattern_keys >= 4);
        assert!(c.repo_keys >= 3);
        assert!(c.keys >= 8);
    }

    #[test]
    fn rejects_json() {
        assert!(!detect(b"{\"name\": \"x\", \"version\": 1}"));
        assert!(Tmgram::parse(b"a = b\n").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_xml_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_xml_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}

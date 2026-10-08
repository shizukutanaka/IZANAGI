//! cSpell `cspell.json`/`cSpell.json` の検出と構造カウント。
//!
//! `version`/`language`/`words`/`flagWords`/`ignoreWords`/`ignorePaths`/
//! `dictionaries`/`languageSettings`/`overrides`/`import`/`useGitignore` 等の
//! cSpell 設定キーを `"key":` 走査で識別する。
//!
//! ```
//! let c = izanagi_kit::cspell::parse(
//!     b"{\n  \"version\": \"0.2\",\n  \"language\": \"en\",\n  \"words\": [\"k8s\"],\n  \"ignorePaths\": [\"dist\"]\n}\n").unwrap();
//! assert!(c.options >= 4);
//! assert!(izanagi_kit::cspell::detect(
//!     b"{\"version\": \"0.2\", \"words\": [\"x\"], \"dictionaries\": []}\n"));
//! ```

use crate::textutil::strip_bom;
/// cSpell 設定既知キー。
const KEYS: &[&str] = &[
    "allowCompoundWords",
    "cache",
    "caseSensitive",
    "description",
    "dictionaries",
    "enableFiletypes",
    "enabled",
    "enabledLanguageIds",
    "enableGlobDot",
    "exclude",
    "files",
    "flagWords",
    "features",
    "ignorePaths",
    "ignoreRegExpList",
    "ignoreWords",
    "import",
    "include",
    "includeRegExpList",
    "language",
    "languageSettings",
    "maxDuplicateProblems",
    "maxNumberOfProblems",
    "minWordLength",
    "noConfigSearch",
    "numSuggestions",
    "overrides",
    "patterns",
    "readonly",
    "reporters",
    "showStatus",
    "spellCheckDelayMs",
    "suggestionsTimeout",
    "suggestionNumChanges",
    "userWords",
    "useGitignore",
    "usePnP",
    "version",
    "words",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー出現行数。
    pub options: usize,
    /// コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn key_hits(t: &str) -> usize {
    let t = t.strip_prefix("- ").map_or(t, |s| s.trim_start());
    let mut n = 0usize;
    for k in KEYS {
        if t.contains(&format!("\"{}\":", k)) || t.starts_with(&format!("{k}:")) {
            n += 1;
        }
    }
    n
}

/// `cspell.json` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        hits += key_hits(t);
        if hits >= 3 {
            return true;
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("//") || t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if key_hits(t) > 0 {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"version\": \"0.2\",\n  \"language\": \"en\",\n  \"words\": [\"k8s\", \"izanagi\"],\n  \"flagWords\": [\"teh\"],\n  \"ignorePaths\": [\"dist\", \"*.lock\"],\n  \"dictionaries\": [\"rust\", \"python\"],\n  \"useGitignore\": true,\n  \"languageSettings\": [\n    {\"languageId\": \"markdown\", \"locale\": \"en,ja\"}\n  ],\n  \"overrides\": []\n}\n";

    #[test]
    fn cspell() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 9);
        assert_eq!(c.misc, 4);
    }

    #[test]
    fn not_cspell() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

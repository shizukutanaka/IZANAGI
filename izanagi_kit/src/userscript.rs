//! ユーザスクリプトメタブロック(`// ==UserScript==`)の検出と構造カウント。
//!
//! `==UserScript==`/`==/UserScript==` マーカー間の `// @<directive> <value>` を
//! 識別する(Greasemonkey/Tampermonkey/Violentmonkey 共通)。
//!
//! ```
//! let c = izanagi_kit::userscript::parse(
//!     b"// ==UserScript==\n// @name test\n// @match https://*/*\n// @grant none\n// ==/UserScript==\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::userscript::detect(
//!     b"// ==UserScript==\n// @name a\n// @match *\n// ==/UserScript==\n"));
//! ```

/// 既知ディレクティブ。
const DIRECTIVES: &[&str] = &[
    "@antifeature",
    "@author",
    "@charset",
    "@compatible",
    "@connect",
    "@contributionAmount",
    "@contributionURL",
    "@contributor",
    "@copyright",
    "@description",
    "@domain",
    "@downloadURL",
    "@exclude",
    "@grant",
    "@homepage",
    "@homepageURL",
    "@icon",
    "@include",
    "@incompatible",
    "@inject-into",
    "@installURL",
    "@license",
    "@match",
    "@name",
    "@namespace",
    "@noframes",
    "@priority",
    "@require",
    "@resource",
    "@run-at",
    "@source",
    "@supportURL",
    "@unwrap",
    "@updateURL",
    "@version",
    "@webRequest",
    "@website",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// メタブロックマーカー行数(`==UserScript==`/`==/UserScript==`)。
    pub sections: usize,
    /// 既知 `@directive` 行数。
    pub options: usize,
    /// その他 `//` コメント行数。
    pub comments: usize,
    /// その他の行数(スクリプト本体を含む)。
    pub misc: usize,
}

fn directive(t: &str) -> Option<&str> {
    let rest = t.strip_prefix("//")?.trim_start();
    if !rest.starts_with('@') {
        return None;
    }
    let end = rest
        .find(|ch: char| ch.is_whitespace())
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// ユーザスクリプトらしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut in_block = false;
    let mut marker = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.contains("==UserScript==") {
            in_block = true;
            marker = true;
            continue;
        }
        if t.contains("==/UserScript==") {
            in_block = false;
            continue;
        }
        if in_block && directive(t).is_some_and(|d| DIRECTIVES.contains(&d)) {
            hits += 1;
            if marker && hits >= 2 {
                return true;
            }
        }
    }
    marker && hits >= 2
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
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
        if t.contains("==UserScript==") || t.contains("==/UserScript==") {
            c.sections += 1;
            continue;
        }
        match directive(t) {
            Some(d) if DIRECTIVES.contains(&d) => c.options += 1,
            Some(_) => c.comments += 1,
            None if t.starts_with("//") => c.comments += 1,
            _ => c.misc += 1,
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"// ==UserScript==\n// @name           My Script\n// @namespace      me\n// @version        1.0\n// @description    does things\n// @match          https://example.com/*\n// @match          https://*.example.org/*\n// @grant          GM_getValue\n// @grant          GM_setValue\n// @run-at         document-end\n// @require        https://cdn.example/lib.js\n// @noframes\n// ==/UserScript==\n// other comment\n(() => { const x = 1; })();\n";

    #[test]
    fn userscript() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.options, 11);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_userscript() {
        assert!(!detect(b"// just js\nconst a = 1;\n"));
        assert!(parse(b"text\n").is_none());
    }
}

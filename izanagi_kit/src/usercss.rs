//! UserCSS(ユーザスタイル)`/* ==UserStyle== */` メタブロックの検出と構造カウント。
//!
//! `==UserStyle==`/`==/UserStyle==` マーカー内の `@directive` と、ブロック外の
//! `@-moz-document` ルール、プリプロセッサ変数(`@var`/`@advanced`)を識別する。
//!
//! ```
//! let c = izanagi_kit::usercss::parse(
//!     b"/* ==UserStyle==\n@name test\n@namespace me\n@version 1.0\n@preprocessor uso\n==/UserStyle== */\n@-moz-document domain(example.com) { }\n").unwrap();
//! assert_eq!(c.options, 4);
//! assert_eq!(c.entries, 1);
//! assert!(izanagi_kit::usercss::detect(
//!     b"/* ==UserStyle==\n@name a\n@version 1\n==/UserStyle== */\n"));
//! ```

/// メタブロック内既知ディレクティブ。
const DIRECTIVES: &[&str] = &[
    "@advanced",
    "@author",
    "@contributionAmount",
    "@contributionURL",
    "@description",
    "@documentationURL",
    "@exclude",
    "@homepageURL",
    "@include",
    "@inject-into",
    "@license",
    "@match",
    "@name",
    "@namespace",
    "@noframes",
    "@preprocessor",
    "@supportURL",
    "@updateURL",
    "@var",
    "@version",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// メタブロックマーカー行数。
    pub sections: usize,
    /// 既知ディレクティブ行数。
    pub options: usize,
    /// `@-moz-document` ルール行数。
    pub entries: usize,
    /// その他コメント行数。
    pub comments: usize,
    /// その他の行数(CSS 本体を含む)。
    pub misc: usize,
}

fn directive(t: &str) -> Option<&str> {
    let t = t.trim_start_matches('*').trim_start();
    if !t.starts_with('@') {
        return None;
    }
    let end = t.find(|ch: char| ch.is_whitespace()).unwrap_or(t.len());
    Some(&t[..end])
}

/// UserCSS らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut in_block = false;
    let mut marker = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.contains("==UserStyle==") {
            in_block = true;
            marker = true;
            continue;
        }
        if t.contains("==/UserStyle==") {
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
        entries: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.contains("==UserStyle==") || t.contains("==/UserStyle==") {
            c.sections += 1;
            continue;
        }
        if t.starts_with("@-moz-document") {
            c.entries += 1;
            continue;
        }
        match directive(t) {
            Some(d) if DIRECTIVES.contains(&d) => c.options += 1,
            Some(_) => c.misc += 1,
            None if t.starts_with("/*") || t.starts_with('*') || t.starts_with("//") => {
                c.comments += 1;
            }
            _ => c.misc += 1,
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"/* ==UserStyle==\n@name         Dark Example\n@namespace    me\n@version      1.0\n@description  dark theme\n@author       me\n@license      MIT\n@preprocessor uso\n@var checkbox dark \"Dark\" 1\n@advanced color bg \"BG\" \"#111\"\n==/UserStyle== */\n@-moz-document domain(example.com) {\nbody { background: #000; }\n}\n@-moz-document url-prefix(https://a.b/) { }\n";

    #[test]
    fn usercss() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.options, 9);
        assert_eq!(c.entries, 2);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_usercss() {
        assert!(!detect(b"body { color: red; }\n"));
        assert!(parse(b"text\n").is_none());
    }
}

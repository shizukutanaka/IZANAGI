//! CodeClimate `.codeclimate.yml` / `.codeclimate.json` の検出・カウント。
//!
//! `engines`/`plugins`/`ratings`/`checks`/`exclude_paths`/`prepare`/`languages`
//! 等 CodeClimate 固有のセクションと、それらの下に並ぶエンジン・チェック名を分類する。
//!
//! ```
//! let cfg = b"version: \"2\"\nengines:\n  rubocop:\n    enabled: true\nratings:\n  paths:\n    - \"**.rb\"\nexclude_paths:\n  - vendor/\n";
//! assert!(izanagi_kit::codeclimate::detect(cfg));
//! let c = izanagi_kit::codeclimate::parse(cfg).unwrap();
//! assert_eq!(c.engines, 2);
//! ```

/// セクション名。
const SECTIONS: &[&str] = &[
    "version",
    "engines",
    "plugins",
    "ratings",
    "checks",
    "prepare",
    "languages",
    "exclude_paths",
    "exclude_patterns",
    "repository",
    "fetch",
    "meta",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// キー・リスト項目の総数。
    pub entries: usize,
    /// `engines`/`plugins` 配下の項目数。
    pub engines: usize,
    /// `checks` 配下の項目数。
    pub checks: usize,
    /// `ratings` 配下の項目数。
    pub ratings: usize,
    /// `exclude_paths`/`exclude_patterns` 配下の除外パターン数。
    pub excludes: usize,
    /// トップレベルセクションキー数 (上記配下を除く)。
    pub sections: usize,
    /// その他項目数。
    pub misc: usize,
}

/// `b` が CodeClimate 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| {
        c.engines + c.checks + c.ratings + c.excludes + c.sections >= 3 && c.sections >= 1
    })
}

/// `b` を `.codeclimate.yml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        engines: 0,
        checks: 0,
        ratings: 0,
        excludes: 0,
        sections: 0,
        misc: 0,
    };
    let mut section = "";
    let mut known_section = false;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if indent == 0 {
            let end = t.find([':', ' ']).unwrap_or(t.len());
            section = &t[..end];
            known_section = SECTIONS.contains(&section);
            if known_section {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let body = t.strip_prefix('-').map_or(t, |r| r.trim_start());
        let name = body
            .find([':', ' '])
            .map_or(body, |i| &body[..i])
            .trim_matches('"')
            .trim_matches('\'');
        match section {
            "engines" | "plugins" => c.engines += 1,
            "checks" => c.checks += 1,
            "ratings" => c.ratings += 1,
            "exclude_paths" | "exclude_patterns" => c.excludes += 1,
            _ => {
                if !known_section && !name.is_empty() {
                    c.misc += 1;
                }
            }
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn codeclimate() {
        let cfg = b"version: \"2\"\nengines:\n  rubocop:\n    enabled: true\n  eslint: {}\nchecks:\n  complexity:\n    enabled: true\nratings:\n  paths:\n    - \"**.rb\"\nexclude_paths:\n  - vendor/\n  - tmp/\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.engines, 3);
        assert_eq!(c.checks, 2);
        assert_eq!(c.ratings, 2);
        assert_eq!(c.excludes, 2);
    }

    #[test]
    fn not_codeclimate() {
        assert!(parse(b"foo: 1\n").is_none());
    }
}

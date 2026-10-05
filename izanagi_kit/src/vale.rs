//! Vale `.vale.ini` の検出と構造カウント。
//!
//! `StylesPath`/`MinAlertLevel`/`Packages`/`Vocab` グローバルキーと
//! `[*.md]`/`[formats]` セクション内の `BasedOnStyles`/`*.Rule = YES|NO`/
//! `TokenIgnores`/`BlockIgnores`/`Vale.Terms` 設定を識別する。
//!
//! ```
//! let c = izanagi_kit::vale::parse(
//!     b"StylesPath = styles\nMinAlertLevel = warning\n\n[*.md]\nBasedOnStyles = Vale, Microsoft\nVale.Terms = NO\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::vale::detect(
//!     b"StylesPath = s\n[*.md]\nBasedOnStyles = Vale\n"));
//! ```

/// グローバル(セクション無し)既知キー。
const GLOBAL_KEYS: &[&str] = &[
    "MinAlertLevel",
    "Packages",
    "RootINI",
    "StylesPath",
    "Vocab",
];

/// セクション内既知キー(プレフィックス一致も含む)。
const SECTION_KEYS: &[&str] = &[
    "BasedOnStyles",
    "BlockIgnores",
    "IgnoredScopes",
    "List",
    "MinAlertLevel",
    "Skips",
    "TokenIgnores",
    "Transform",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[*.md]` 等のセクション行数。
    pub sections: usize,
    /// 既知キー・`Style.Rule = YES|NO` 行数。
    pub options: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn is_rule_toggle(t: &str) -> bool {
    // `Style.Rule = YES|NO` 形(ドット含むキー + bool 値)。
    let Some(eq) = t.find('=') else { return false };
    let key = t[..eq].trim();
    let val = t[eq + 1..].trim().to_ascii_uppercase();
    key.contains('.')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        && matches!(
            val.as_str(),
            "YES" | "NO" | "SUGGESTION" | "WARNING" | "ERROR"
        )
}

fn known_key(t: &str) -> bool {
    let Some(eq) = t.find('=') else { return false };
    let key = t[..eq].trim();
    GLOBAL_KEYS.contains(&key) || SECTION_KEYS.contains(&key)
}

/// `.vale.ini` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut styles = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if t.starts_with("StylesPath") {
            styles = true;
        }
        if known_key(t) || is_rule_toggle(t) || t.starts_with("[*") || t.starts_with("[formats]") {
            hits += 1;
        }
        if styles && hits >= 3 {
            return true;
        }
    }
    styles && hits >= 2
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
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.contains(']') {
            c.sections += 1;
        } else if known_key(t) || is_rule_toggle(t) {
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

    const SAMPLE: &[u8] = b"; vale config\nStylesPath = styles\nMinAlertLevel = suggestion\nPackages = Microsoft, proselint\n\n[formats]\nmdx = md\n\n[*.md]\nBasedOnStyles = Vale, Microsoft\nMicrosoft.Acronyms = NO\nVale.Terms = YES\nTokenIgnores = (?s)```.*?\n\n[*.txt]\nBasedOnStyles = Vale\n";

    #[test]
    fn vale() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.options, 8);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_vale() {
        assert!(!detect(b"[section]\nkey = 1\n"));
        assert!(parse(b"text\n").is_none());
    }
}

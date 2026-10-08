//! alex `.alexrc`/`.alexrc.js`/`.alexrc.json` の検出と構造カウント。
//!
//! `allow`/`deny`/`profanitySureness`/`noBinary` 設定キー
//! (JS `module.exports = {...}` または JSON/YAML `key: value` 形)を識別する。
//!
//! ```
//! let c = izanagi_kit::alexrc::parse(
//!     b"module.exports = {\n  allow: [\"her-his\"],\n  profanitySureness: 1,\n  noBinary: true\n};\n").unwrap();
//! assert!(c.options >= 3);
//! assert!(izanagi_kit::alexrc::detect(
//!     b"allow:\n  - her-his\nprofanitySureness: 1\nnoBinary: true\n"));
//! ```

/// alex 既知設定キー。
const KEYS: &[&str] = &["allow", "deny", "noBinary", "profanitySureness"];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー行数。
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
        if t.starts_with(&format!("{k}:"))
            || t.contains(&format!("{k}:"))
            || t.contains(&format!("\"{k}\":"))
            || t.starts_with(&format!("{k} ="))
        {
            n += 1;
        }
    }
    n
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `.alexrc` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut hits = 0usize;
    let mut marker = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') || t.starts_with('*') {
            continue;
        }
        if t.starts_with("module.exports") {
            marker = true;
        }
        hits += key_hits(t);
        if hits >= 2 {
            return true;
        }
    }
    marker && hits >= 1
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
        if t.starts_with("//") || t.starts_with('#') || t.starts_with('*') || t.starts_with("/*") {
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

    const SAMPLE: &[u8] = b"module.exports = {\n  allow: [\n    \"her-his\",\n    \"he-she\"\n  ],\n  profanitySureness: 1,\n  noBinary: true\n};\n";

    #[test]
    fn alexrc() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 3);
        assert_eq!(c.misc, 5);
    }

    #[test]
    fn not_alexrc() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

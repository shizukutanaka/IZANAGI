//! Application Cache マニフェスト(`CACHE MANIFEST`)の検出と構造カウント。
//!
//! 先頭 `CACHE MANIFEST` 行と `CACHE:`/`NETWORK:`/`FALLBACK:` セクションヘッダ、
//! URI エントリ(`FALLBACK` では `url fallback` 2 トークン)、ワイルドカード `*` を
//! 識別する。
//!
//! ```
//! let c = izanagi_kit::appcache::parse(
//!     b"CACHE MANIFEST\n# v1\nCACHE:\n/app.js\nNETWORK:\n*\nFALLBACK:\n/ /offline.html\n").unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.entries, 3);
//! assert!(izanagi_kit::appcache::detect(b"CACHE MANIFEST\nCACHE:\n/a.css\n"));
//! ```

/// セクションヘッダ。
const HEADERS: &[&str] = &["CACHE:", "FALLBACK:", "NETWORK:", "SETTINGS:"];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクションヘッダ行数。
    pub sections: usize,
    /// URI/フォールバックエントリ行数(`*` を含む)。
    pub entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `CACHE MANIFEST` らしさを判定する(先頭マーカー必須)。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    for line in text.strip_prefix('\u{feff}').unwrap_or(text).lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        return t == "CACHE MANIFEST"
            || t == "CACHE MANIFEST "
            || t.starts_with("CACHE MANIFEST #");
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut c = Counts {
        sections: 0,
        entries: 0,
        comments: 0,
        misc: 0,
    };
    let mut first = true;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if first {
            first = false;
            if t.starts_with("CACHE MANIFEST") {
                c.sections += 1;
                continue;
            }
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if HEADERS.contains(&t) {
            c.sections += 1;
            continue;
        }
        if t.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '/' | '.'
                        | '_'
                        | '-'
                        | ':'
                        | '?'
                        | '&'
                        | '='
                        | '%'
                        | '~'
                        | '*'
                        | '+'
                        | '@'
                        | '!'
                        | '$'
                        | ','
                        | ';'
                        | '('
                        | ')'
                        | '\''
                        | ' '
                )
        }) {
            c.entries += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8_bom() {
        let mut v = b"\xef\xbb\xbf".to_vec();
        v.extend_from_slice(SAMPLE);
        assert!(detect(&v));
        assert!(parse(&v).is_some());
    }

    const SAMPLE: &[u8] = b"CACHE MANIFEST\n# 2024-01 rev\nCACHE:\n/index.html\n/css/app.css\n/js/app.js\nNETWORK:\napi/status\n*\nFALLBACK:\n/api /offline.json\n/ /offline.html\n";

    #[test]
    fn appcache() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.entries, 7);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_appcache() {
        assert!(!detect(b"<html></html>\n"));
        assert!(parse(b"text\n").is_none());
    }
}

//! RubyGems 設定ファイル(`.gemrc`)の検出と構造カウント。
//!
//! `:` プレフィックス付きシンボルキー(`:sources:`/`:backtrace`/
//! `:bulk_threshold:`/`:ssl_verify_mode:`/`:update_sources:`/`:verbose:` 等)、
//! `gem:` コマンドキー、サブコマンドキー(`install:`/`update:` 等)、
//! `gem_home`/`gem_path` キーを識別する。
//!
//! ```
//! let c = izanagi_kit::gemrc::parse(
//!     b"gem: --no-document\n:sources:\n  - https://rubygems.org\n:verbose: true\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert_eq!(c.items, 1);
//! assert!(izanagi_kit::gemrc::detect(b"gem: --no-document\n:backtrace: true\n"));
//! ```

use crate::textutil::strip_bom;
/// `:` プレフィックスの既知シンボルキー。
const SYMBOL_KEYS: &[&str] = &[
    ":backtrace",
    ":benchmark",
    ":bulk_threshold",
    ":concurrent_downloads",
    ":gemdeps",
    ":install_dir",
    ":local",
    ":remote",
    ":sources",
    ":ssl_ca_cert",
    ":ssl_client_cert",
    ":ssl_verify_mode",
    ":suggest_alternate",
    ":update_sources",
    ":verbose",
];

/// プレーンな既知キー。
const KEYS: &[&str] = &[
    "benchmark",
    "concurrent_downloads",
    "custom_shebang",
    "gem",
    "gem_home",
    "gem_path",
    "gemdeps",
    "install",
    "sources",
    "update",
];

/// gemrc 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key:`/`key: value` 既知オプション行。
    pub options: usize,
    /// `- value` リスト要素。
    pub items: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 未知キー/分類不能行。
    pub misc: usize,
}

/// `key:` または `key: value` のキー部分(`:` プレフィックス保持)。
fn yaml_key(t: &str) -> Option<&str> {
    // `:sym:` 形式: 終端 `:` がキーの一部。
    if let Some(rest) = t.strip_prefix(':') {
        if let Some(end) = rest.find(':') {
            let k = &t[..end + 2];
            if k.len() > 2
                && rest[..end]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Some(k);
            }
        }
        return None;
    }
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        None
    } else {
        Some(k)
    }
}

/// b が .gemrc かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with('#')
                && yaml_key(t).is_some_and(|k| {
                    SYMBOL_KEYS.contains(&k.strip_suffix(':').unwrap_or(k)) || KEYS.contains(&k)
                })
        })
        .count()
        >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        items: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('-') {
            c.items += 1;
            continue;
        }
        if let Some(k) = yaml_key(t) {
            if SYMBOL_KEYS.contains(&k.strip_suffix(':').unwrap_or(k)) || KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# gemrc\ngem: --no-document\n:sources:\n  - https://rubygems.org\n  - https://gems.example.com\n:backtrace: true\n:bulk_threshold: 1000\n:ssl_verify_mode: 0\n:verbose: true\nupdate: --no-document\ninstall: --no-document\ngem_home: ~/.gem/ruby/3.3.0\n";

    #[test]
    fn gemrc() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 9);
        assert_eq!(c.items, 2);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_gemrc() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"foo = bar\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

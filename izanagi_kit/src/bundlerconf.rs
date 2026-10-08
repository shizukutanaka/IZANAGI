//! Bundler 設定ファイル(`.bundle/config`)の検出と構造カウント。
//!
//! `BUNDLE_*` 環境変数風キー(`BUNDLE_PATH`/`BUNDLE_WITHOUT`/`BUNDLE_WITH`/
//! `BUNDLE_FROZEN`/`BUNDLE_JOBS`/`BUNDLE_RETRY`/`BUNDLE_BIN`/`BUNDLE_ONLY`/
//! `BUNDLE_GEMFILE`/`BUNDLE_MIRROR__*`/`BUNDLE_GLOBAL__*`/
//! `BUNDLE_LOCAL__*`/`BUNDLE_CACHE_PATH`/`BUNDLE_APP_CONFIG` 等)を識別する。
//!
//! ```
//! let c = izanagi_kit::bundlerconf::parse(
//!     b"BUNDLE_PATH: vendor/bundle\nBUNDLE_WITHOUT: \"development:test\"\nBUNDLE_JOBS: 4\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::bundlerconf::detect(b"BUNDLE_FROZEN: true\nBUNDLE_RETRY: 3\n"));
//! ```

use crate::textutil::strip_bom;
/// `BUNDLE_` 接頭辞なしで参照される既定名(bundle config set 名)。
const KEYS: &[&str] = &[
    "allow_multisource",
    "app_config",
    "auto_install",
    "bin",
    "cache_all",
    "cache_path",
    "clean",
    "default_install_uses_path",
    "deployment",
    "disable_checksum_validation",
    "disable_exec_load",
    "disable_shared_gems",
    "disable_version_check",
    "force_ruby_platform",
    "frozen",
    "gemfile",
    "ignore_messages",
    "jobs",
    "major_deprecations",
    "mirror",
    "no_install",
    "no_prune",
    "only",
    "path",
    "prefer_patch",
    "retry",
    "shebang",
    "silence_root_warning",
    "ssl_ca_cert",
    "ssl_client_cert",
    "ssl_verify_mode",
    "suppress_install_using_messages",
    "system_bindir",
    "timeout",
    "user_agent",
    "with",
    "without",
];

/// bundler 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `BUNDLE_*` または既定名の `key: value`/`key = value` 行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 未知キー/分類不能行。
    pub misc: usize,
}

/// `key: value` / `key = value` のキー部分。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find(':').or_else(|| t.find('='))?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}

/// キーが Bundler キーか(`BUNDLE_*` 接頭辞または既知既定名)。
fn is_bundle_key(k: &str) -> bool {
    k.starts_with("BUNDLE_")
        || KEYS.contains(&k)
        || k.strip_prefix("bundle.").is_some_and(|s| KEYS.contains(&s))
}

/// b が .bundle/config かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with('#') && kv_key(t).is_some_and(is_bundle_key)
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
        if let Some(k) = kv_key(t) {
            if is_bundle_key(k) {
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

    const SAMPLE: &[u8] = b"# bundler\nBUNDLE_PATH: vendor/bundle\nBUNDLE_WITHOUT: \"development:test\"\nBUNDLE_JOBS: 4\nBUNDLE_RETRY: 3\nBUNDLE_FROZEN: true\nBUNDLE_MIRROR__HTTPS://RUBYGEMS__ORG/: https://mirror.local\nBUNDLE_GLOBAL__PATH__SYSTEM__BUNDLE: false\n";

    #[test]
    fn bundlerconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 7);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_bundler() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"FOO=bar\nBAZ=qux\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

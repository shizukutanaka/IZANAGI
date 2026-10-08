//! Travis CI 設定(`.travis.yml`)の検出と構造カウント。
//!
//! `language`/`script`/`install`/`jobs`/`stages`/`deploy`/`env`/`cache`/
//! `addons`/`services`/`notifications`/`branches`/`matrix` 等のトップキーと、
//! `jobs:`/`matrix:` 内の `include:`/`exclude:`/`allow_failures:`/`fast_finish:`
//! エントリを識別する。
//!
//! ```
//! let c = izanagi_kit::travisci::parse(
//!     b"language: rust\nscript:\n  - cargo test\njobs:\n  include:\n    - name: lint\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.jobs, 1);
//! assert!(izanagi_kit::travisci::detect(b"language: rust\nscript: cargo test\n"));
//! ```

use crate::textutil::strip_bom;
/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "addons",
    "after_deploy",
    "after_failure",
    "after_script",
    "after_success",
    "arch",
    "assembly_info",
    "before_cache",
    "before_deploy",
    "before_install",
    "before_script",
    "branches",
    "bundler_args",
    "cache",
    "compiler",
    "composer_args",
    "deploy",
    "dist",
    "dotnet",
    "edge",
    "env",
    "filter_secrets",
    "gemfile",
    "git",
    "go",
    "group",
    "haskell",
    "if",
    "import",
    "install",
    "jdk",
    "jobs",
    "language",
    "matrix",
    "mono",
    "name",
    "node_js",
    "notifications",
    "nvm",
    "os",
    "osx_image",
    "php",
    "provider",
    "python",
    "r",
    "ruby",
    "rust",
    "rvm",
    "scala",
    "script",
    "services",
    "shard",
    "smalltalk",
    "solution",
    "sonarcloud",
    "stages",
    "sudo",
    "version",
    "workspace",
    "xcode_project",
];

/// `jobs:`/`matrix:` 内の既知サブキー。
const JOB_KEYS: &[&str] = &["allow_failures", "exclude", "fast_finish", "include"];

/// travisci 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルキー行。
    pub sections: usize,
    /// `jobs:`/`matrix:` 内の既知サブキー + `- ` ジョブ要素。
    pub jobs: usize,
    /// その他のネストした `key:` 行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行(リスト要素・継続スカラ・未知キー)。
    pub misc: usize,
}

/// `key:` 先頭のキー名(行末 `:` or `: value`)。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}

/// Travis 固有のトップキー(`env`/`script`/`install`/`cache`/`jobs`/
/// `services` 等は他の CI YAML でも現れるため除外)。最低1件要求。
const EXCLUSIVE_KEYS: &[&str] = &[
    "addons",
    "after_deploy",
    "after_failure",
    "after_script",
    "after_success",
    "before_cache",
    "before_deploy",
    "before_install",
    "before_script",
    "bundler_args",
    "composer_args",
    "dist",
    "dotnet",
    "edge",
    "filter_secrets",
    "gemfile",
    "language",
    "matrix",
    "mono",
    "node_js",
    "notifications",
    "nvm",
    "osx_image",
    "rvm",
    "sonarcloud",
    "sudo",
];

/// b が .travis.yml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0usize;
    let mut exclusive = 0usize;
    for l in text.lines() {
        let t = l.trim();
        if t.starts_with('#') || (l.len() - l.trim_start().len()) != 0 {
            continue;
        }
        if let Some(k) = yaml_key(t) {
            if TOP_KEYS.contains(&k) {
                hits += 1;
                if EXCLUSIVE_KEYS.contains(&k) {
                    exclusive += 1;
                }
            }
        }
    }
    hits >= 2 && exclusive >= 1
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        jobs: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_jobs = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_jobs = matches!(t, "jobs:" | "matrix:");
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let item = t.strip_prefix("- ").map_or(t, |s| s);
        if in_jobs && item.starts_with("- ") {
            c.jobs += 1;
            continue;
        }
        if let Some(k) = yaml_key(item) {
            if in_jobs && JOB_KEYS.contains(&k) {
                c.jobs += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# travis\nlanguage: rust\nos: linux\ndist: focal\ncache: cargo\nenv:\n  - RUST_BACKTRACE=1\njobs:\n  include:\n    - name: lint\n      script: cargo clippy\n  allow_failures:\n    - rust: nightly\n  fast_finish: true\nscript:\n  - cargo build\n  - cargo test\ndeploy:\n  provider: releases\nnotifications:\n  email: false\n";

    #[test]
    fn travisci() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 9);
        assert_eq!(c.jobs, 3);
        assert!(c.options >= 3);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 3);
    }

    #[test]
    fn not_travis() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"language: yaml\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

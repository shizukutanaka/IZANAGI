//! pre-commit 設定(`.pre-commit-config.yaml`)の検出と構造カウント。
//!
//! `repos:` リスト(`- repo:`/`rev:`/`hooks:`/`- id:`)とトップキー
//! (`default_stages`/`default_install_hook_types`/`default_language_version`/
//! `minimum_pre_commit_version`/`fail_fast`/`exclude`/`files`/`ci:`)を識別する。
//!
//! ```
//! let c = izanagi_kit::precommit::parse(
//!     b"repos:\n  - repo: https://github.com/pre-commit/pre-commit-hooks\n    rev: v4.5.0\n    hooks:\n      - id: trailing-whitespace\n      - id: end-of-file-fixer\n").unwrap();
//! assert_eq!(c.repos, 1);
//! assert_eq!(c.hooks, 2);
//! assert!(izanagi_kit::precommit::detect(
//!     b"repos:\n  - repo: local\n    hooks:\n      - id: x\n"));
//! ```

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `tr` が `- key:` リスト項目行かどうか。
fn is_item(tr: &str, key: &str) -> bool {
    let t = tr.trim_start_matches('-').trim_start();
    is_key(t, key)
}

/// `.pre-commit-config.yaml` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// `- repo:` 項目数。
    pub repos: usize,
    /// `- id:` フック項目数。
    pub hooks: usize,
    /// `rev:`/`language:` 等の補助キー行数。
    pub option_keys: usize,
    /// 既知トップレベルキー行数。
    pub top_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が `.pre-commit-config.yaml` に見えるかを判定する。
///
/// `repos:` キーと `- repo:` 項目を要求する — 単独で現れるキー名は
/// 汎用語のためリスト構造を併用する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let has_repos = t.lines().any(|l| is_key(l.trim(), "repos"));
    let has_repo_item = t.lines().any(|l| is_item(l.trim(), "repo"));
    has_repos && has_repo_item
}

/// `b` を `.pre-commit-config.yaml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        repos: 0,
        hooks: 0,
        option_keys: 0,
        top_keys: 0,
        comments: 0,
        misc: 0,
    };
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_item(tr, "repo") {
            c.repos += 1;
            continue;
        }
        if is_item(tr, "id") {
            c.hooks += 1;
            continue;
        }
        if is_key(tr, "rev") || is_key(tr, "language") || is_key(tr, "alias") {
            c.option_keys += 1;
            continue;
        }
        if TOP_KEYS.iter().any(|k| is_key(tr, k)) {
            c.top_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "ci",
    "default_install_hook_types",
    "default_language_version",
    "default_stages",
    "exclude",
    "exclude_types",
    "fail_fast",
    "files",
    "minimum_pre_commit_version",
];

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"repos:\n  - repo: https://github.com/pre-commit/pre-commit-hooks\n    rev: v4.5.0\n    hooks:\n      - id: trailing-whitespace\n      - id: end-of-file-fixer\n        args: [--fix=lf]\n  - repo: local\n    hooks:\n      - id: fmt\n        name: fmt\n        entry: cargo fmt\n        language: system\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(b"repos:\n  - repo: local\n"));
        assert!(!detect(
            b"# repos: listed only in comments\n#   - repo: x\n"
        ));
        assert!(!detect(b"repository: x\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.repos, 2);
        assert_eq!(c.hooks, 3);
        assert!(c.option_keys >= 2);
        assert!(parse(b"plain: yaml\n").is_none());
    }

    #[test]
    fn counts_comments() {
        let c = parse(b"# header\nrepos:\n  - repo: local\n").unwrap();
        assert_eq!(c.comments, 1);
        assert_eq!(c.repos, 1);
    }
}

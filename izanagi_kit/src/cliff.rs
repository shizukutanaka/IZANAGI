//! git-cliff(`cliff.toml`)の検出と構造カウント。
//!
//! `[changelog]` セクション(`header`/`body`/`footer`/`trim`)と `[git]`
//! セクション(`conventional_commits`/`commit_parsers`/`tag_pattern`/
//! `filter_commits`/`sort_commits`/`link_parsers`/`split_commits`/
//! `commit_preprocessors`/`protect_breaking_commits`/`filter_unconventional`)
//! の組合せを要求する。
//!
//! ```
//! let b = b"[changelog]\nheader = \"# Changelog\\n\"\nbody = \"{% for c in commits %}{{ c.message }}{% endfor %}\"\ntrim = true\n\n[git]\nconventional_commits = true\ntag_pattern = \"v[0-9].*\"\n";
//! assert!(izanagi_kit::cliff::detect(b));
//! let c = izanagi_kit::cliff::parse(b).unwrap();
//! assert!(c.git_keys >= 2);
//! ```

/// TOML セクションヘッダ `[name]`(配列表 `[[name]]` は別扱い)。
fn section_of(tr: &str) -> Option<&str> {
    tr.strip_prefix('[')?
        .strip_suffix(']')
        .map(|s| s.trim())
        .filter(|s| !s.starts_with('[') && !s.is_empty())
}

/// `key = value` 行のキー部。
fn toml_key(tr: &str) -> Option<&str> {
    tr.split_once('=')
        .map(|(k, _)| k.trim())
        .filter(|k| !k.is_empty())
}

/// `[changelog]` 配下の既知キー。
const CHANGELOG_KEYS: &[&str] = &[
    "body",
    "footer",
    "header",
    "postprocessors",
    "template",
    "trim",
];

/// `[git]` 配下の既知キー。
const GIT_KEYS: &[&str] = &[
    "commit_parsers",
    "commit_preprocessors",
    "conventional_commits",
    "filter_commits",
    "filter_unconventional",
    "ignore_tags",
    "link_parsers",
    "protect_breaking_commits",
    "sort_commits",
    "split_commits",
    "tag_pattern",
    "topo_order",
    "use_branch_tags",
];

/// `[changelog]` と `[git]`/固有キーの両方があるか。
fn shape(t: &str) -> (bool, bool) {
    let mut changelog = false;
    let mut git = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if let Some(s) = section_of(tr) {
            let top = s.split('.').next().unwrap_or(s);
            changelog |= top == "changelog";
            git |= top == "git"
                || top == "remote"
                || top == "bump"
                || top == "github"
                || top == "gitlab"
                || top == "bitbucket"
                || top == "gitea"
                || top == "codeberg"
                || top == "sourcehut";
            continue;
        }
        if let Some(k) = toml_key(tr) {
            git |= GIT_KEYS.contains(&k);
        }
    }
    (changelog, git)
}

/// `cliff.toml` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// 認識したセクションヘッダ数。
    pub sections: usize,
    /// `[changelog]` セクション内の既知キー行数。
    pub changelog_keys: usize,
    /// `[git]` セクション内の既知キー行数。
    pub git_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が `cliff.toml` に見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let (changelog, git) = shape(t);
    changelog && git
}

/// `b` を `cliff.toml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        sections: 0,
        changelog_keys: 0,
        git_keys: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_changelog = false;
    let mut in_git = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if let Some(s) = section_of(tr) {
            let top = s.split('.').next().unwrap_or(s);
            in_changelog = top == "changelog";
            in_git = top == "git";
            if in_changelog || in_git {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if in_changelog && toml_key(tr).is_some_and(|k| CHANGELOG_KEYS.contains(&k)) {
            c.changelog_keys += 1;
            continue;
        }
        if in_git && toml_key(tr).is_some_and(|k| GIT_KEYS.contains(&k)) {
            c.git_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[changelog]\nheader = \"# Changelog\\n\"\nbody = \"{% for c in commits %}{{ c.message }}{% endfor %}\"\ntrim = true\n\n[git]\nconventional_commits = true\ntag_pattern = \"v[0-9].*\"\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(b"[changelog]\ntrim = true\ncommit_parsers = []\n"));
        assert!(!detect(b"[package]\nname = \"x\"\nversion = \"1\"\n"));
        // [changelog] だけでは検出しない。
        assert!(!detect(b"[changelog]\ntrim = true\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert!(c.changelog_keys >= 3);
        assert!(c.git_keys >= 2);
        assert!(parse(b"[package]\nname = \"x\"\n").is_none());
    }
}

//! lefthook 設定(`lefthook.yml`)の検出と構造カウント。
//!
//! git フック名(`pre-commit`/`pre-push`/`commit-msg`/`post-merge`/…)を
//! トップキーに持ち、配下に `commands:`/`scripts:`/`run:`/`files:`/
//! `glob:`/`parallel:`/`piped:`/`skip:`/`only:`/`exclude:` を置く構成を識別する。
//!
//! ```
//! let c = izanagi_kit::lefthook::parse(
//!     b"pre-commit:\n  commands:\n    lint:\n      run: cargo clippy\npre-push:\n  commands:\n    test:\n      run: cargo test\n").unwrap();
//! assert_eq!(c.hooks, 2);
//! assert_eq!(c.commands, 2);
//! assert!(izanagi_kit::lefthook::detect(
//!     b"pre-commit:\n  commands:\n    fmt:\n      run: cargo fmt\n"));
//! ```

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// 認識する git フック名(トップレベルキー)。
const HOOK_NAMES: &[&str] = &[
    "applypatch-msg",
    "commit-msg",
    "fsmonitor-watchman",
    "p4-pre-submit",
    "post-applypatch",
    "post-checkout",
    "post-commit",
    "post-index-change",
    "post-merge",
    "post-receive",
    "post-rewrite",
    "post-update",
    "pre-applypatch",
    "pre-auto-gc",
    "pre-commit",
    "pre-merge-commit",
    "pre-push",
    "pre-rebase",
    "pre-receive",
    "prepare-commit-msg",
    "proc-receive",
    "push-to-checkout",
    "reference-transactions",
    "sendemail-validate",
    "update",
];

/// `lefthook.yml` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// フック名キー行数。
    pub hooks: usize,
    /// `commands:`/`scripts:` 配下のエントリ行数。
    pub commands: usize,
    /// `scripts:` 配下のエントリ行数。
    pub scripts: usize,
    /// `run:`/`files:`/`glob:` 等の補助キー行数。
    pub option_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が `lefthook.yml` に見えるかを判定する。
///
/// フック名キーが見つかり、`commands:`/`scripts:`/`run:`/`parallel:`/
/// `piped:` いずれかの構造キーが伴うこと — フック名だけでは
/// git 設定系ファイルと誤検出し得るため。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let hook_hit = t
        .lines()
        .any(|l| HOOK_NAMES.iter().any(|k| is_key(l.trim(), k)));
    let structural = t.lines().any(|l| {
        ["commands", "scripts", "run", "parallel", "piped"]
            .iter()
            .any(|k| is_key(l.trim(), k))
    });
    hook_hit && structural
}

/// `b` を `lefthook.yml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        hooks: 0,
        commands: 0,
        scripts: 0,
        option_keys: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_commands = false;
    let mut in_scripts = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let top = l.len() - l.trim_start().len() == 0;
        if top {
            in_commands = false;
            in_scripts = false;
        }
        if top && HOOK_NAMES.iter().any(|k| is_key(tr, k)) {
            c.hooks += 1;
            continue;
        }
        if is_key(tr, "commands") {
            in_commands = true;
            continue;
        }
        if is_key(tr, "scripts") {
            in_scripts = true;
            continue;
        }
        if in_commands && l.len() - l.trim_start().len() > 0 {
            if is_key(tr, "run") || is_key(tr, "files") || is_key(tr, "glob") {
                c.option_keys += 1;
            } else if !tr.starts_with('-') {
                c.commands += 1;
            }
            continue;
        }
        if in_scripts && l.len() - l.trim_start().len() > 0 {
            if is_key(tr, "runner") || is_key(tr, "interactive") {
                c.option_keys += 1;
            } else if !tr.starts_with('-') {
                c.scripts += 1;
            }
            continue;
        }
        if [
            "run", "files", "glob", "parallel", "piped", "skip", "only", "exclude",
        ]
        .iter()
        .any(|k| is_key(tr, k))
        {
            c.option_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"pre-commit:\n  parallel: true\n  commands:\n    lint:\n      run: cargo clippy\n    fmt:\n      glob: \"*.rs\"\n      run: cargo fmt\npost-merge:\n  scripts:\n    \"deps.sh\":\n      runner: bash\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"commit-msg:\n  commands:\n    lint:\n      run: commitlint -e\n"
        ));
        assert!(!detect(b"# pre-commit:\n#   commands:\n"));
        assert!(!detect(b"foo: bar\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.hooks, 2);
        assert_eq!(c.commands, 2);
        assert_eq!(c.scripts, 1);
        assert!(c.option_keys >= 3);
        assert!(parse(b"plain: yaml\n").is_none());
    }
}

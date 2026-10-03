//! Codecov `codecov.yml` の検出・カウント。
//!
//! `coverage`/`codecov`/`parsers`/`flag_management`/`comment`/`notify`
//! 等 Codecov 固有のトップレベルキーを持つ YAML 設定。
//!
//! ```
//! let cfg = b"coverage:\n  precision: 2\ncodecov:\n  bot: x\nnotify:\n  slack: {}\nignore:\n  - t/\n";
//! assert!(izanagi_kit::codecov::detect(cfg));
//! let c = izanagi_kit::codecov::parse(cfg).unwrap();
//! assert_eq!(c.entries, 4);
//! ```

/// カバレッジ系トップキー。
const COVERAGE: &[&str] = &[
    "coverage",
    "parsers",
    "status",
    "flag_management",
    "comment",
    "flags",
    "profiles",
];

/// 通知系トップキー。
const NOTIFY: &[&str] = &[
    "notify", "slack", "irc", "hipchat", "gitter", "flowdock", "webhook",
];

/// 除外系トップキー。
const IGNORES: &[&str] = &["ignore", "ignores", "fixes"];

/// 挙動系トップキー。
const BEHAVIOR: &[&str] = &[
    "codecov",
    "github_checks",
    "ai_pr_review",
    "archiving",
    "profiling",
    "meta",
    "bot",
    "enterprise",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルキー総数。
    pub entries: usize,
    /// `coverage`/`parsers`/`status`/`flag_management`/`comment` 等カバレッジ系キー数。
    pub coverage: usize,
    /// `notify`/`slack`/`webhook` 等通知系キー数。
    pub notify: usize,
    /// `ignore`/`fixes` 等除外系キー数。
    pub ignores: usize,
    /// `codecov`/`github_checks`/`ai_pr_review`/`meta` 等挙動系キー数。
    pub behavior: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が Codecov 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// トップレベルキー名を列挙。
fn top_keys(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with([' ', '\t', '-', '#']) || line.trim().is_empty() {
            continue;
        }
        if let Some(end) = line.find(':') {
            let key = line[..end].trim();
            if !key.is_empty() {
                out.push(key);
            }
        }
    }
    out
}

/// `b` を `codecov.yml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let keys = top_keys(text);
    if keys.is_empty() {
        return None;
    }
    let mut c = Counts {
        entries: keys.len(),
        coverage: 0,
        notify: 0,
        ignores: 0,
        behavior: 0,
        misc: 0,
    };
    for k in keys {
        if COVERAGE.contains(&k) {
            c.coverage += 1;
        } else if NOTIFY.contains(&k) {
            c.notify += 1;
        } else if IGNORES.contains(&k) {
            c.ignores += 1;
        } else if BEHAVIOR.contains(&k) {
            c.behavior += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.entries - c.misc >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn codecov() {
        let cfg = b"coverage:\n  precision: 2\nparsers:\n  javascript: {}\ncomment:\n  layout: x\nnotify:\n  slack: {}\nignore:\n  - t\nfixes:\n  - a::b\ncodecov:\n  bot: b\ngithub_checks:\n  annotations: true\nextra: 1\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.entries, 9);
        assert_eq!(c.coverage, 3);
        assert_eq!(c.notify, 1);
        assert_eq!(c.ignores, 2);
        assert_eq!(c.behavior, 2);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_codecov() {
        assert!(parse(b"foo: 1\nbar: 2\n").is_none());
    }
}

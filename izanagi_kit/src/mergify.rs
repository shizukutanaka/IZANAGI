//! Mergify `.mergify.yml` の検出・カウント。
//!
//! `pull_request_rules`/`queue_rules`/`merge_protections`/`commands_restrictions`
//! 等 Mergify 固有のトップレベルセクションと、その配下の `- name:` 等
//! ルール定義項目を分類する。
//!
//! ```
//! let cfg = b"pull_request_rules:\n  - name: merge\n    conditions:\n      - check-success=ci\nqueue_rules:\n  - name: default\n";
//! assert!(izanagi_kit::mergify::detect(cfg));
//! let c = izanagi_kit::mergify::parse(cfg).unwrap();
//! assert_eq!(c.items, 3);
//! ```

/// ルール系トップキー。
const RULES: &[&str] = &[
    "pull_request_rules",
    "queue_rules",
    "merge_protections",
    "priorities",
    "partition_rules",
];

/// コマンド・定義系トップキー。
const COMMANDS: &[&str] = &[
    "commands_restrictions",
    "definitions",
    "shared",
    "footer",
    "extends",
    "allow_mergify_commands",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// キー・リスト項目の総数。
    pub entries: usize,
    /// `pull_request_rules`/`queue_rules` 等ルール系セクションキー数。
    pub rules: usize,
    /// ルール配下の `- name:` 等リスト項目数。
    pub items: usize,
    /// `commands_restrictions`/`definitions`/`shared` 等コマンド系キー数。
    pub commands: usize,
    /// その他項目数。
    pub misc: usize,
}

/// `b` が Mergify 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.rules >= 1 && c.items >= 1)
}

/// `b` を `.mergify.yml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        rules: 0,
        items: 0,
        commands: 0,
        misc: 0,
    };
    let mut rule_depth = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if indent == 0 {
            rule_depth = None;
            let end = t.find([':', ' ']).unwrap_or(t.len());
            let key = &t[..end];
            if RULES.contains(&key) {
                c.rules += 1;
                rule_depth = Some(indent);
            } else if COMMANDS.contains(&key) {
                c.commands += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if rule_depth.is_some_and(|d| indent > d) && t.starts_with('-') {
            c.items += 1;
        }
    }
    (c.rules >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn mergify() {
        let cfg = b"pull_request_rules:\n  - name: merge\n    conditions:\n      - check-success=ci\n    actions:\n      merge: {}\n  - name: label\n    conditions: []\nqueue_rules:\n  - name: default\ncommands_restrictions:\n  rebase: {}\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.rules, 2);
        assert_eq!(c.commands, 1);
        assert_eq!(c.items, 4);
    }

    #[test]
    fn not_mergify() {
        assert!(parse(b"foo: 1\n").is_none());
    }
}

//! Dependabot `.github/dependabot.yml` の検出・カウント。
//!
//! `updates:` の下に `package-ecosystem`/`directory`/`schedule` を持つ
//! エントリリストを持つ YAML 設定。`registries`/`version`/`enable-beta-ecosystems`
//! トップキーも分類する。
//!
//! ```
//! let cfg = b"version: 2\nupdates:\n  - package-ecosystem: cargo\n    directory: /\n    schedule:\n      interval: weekly\n";
//! assert!(izanagi_kit::dependabot::detect(cfg));
//! let c = izanagi_kit::dependabot::parse(cfg).unwrap();
//! assert_eq!(c.ecosystems, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// キー・リスト項目の総数。
    pub entries: usize,
    /// `package-ecosystem` 宣言数 (更新エントリ数)。
    pub ecosystems: usize,
    /// `schedule`/`interval`/`day`/`time`/`timezone` 系キー数。
    pub schedules: usize,
    /// `allow`/`ignore`/`groups`/`versioning-strategy` 等許可・除外系キー数。
    pub allow: usize,
    /// `updates`/`registries`/`version`/`enable-beta-ecosystems` トップキー数。
    pub sections: usize,
    /// その他項目数。
    pub misc: usize,
}

/// `b` が dependabot.yml かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.ecosystems >= 1)
}

/// `b` を dependabot.yml として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        ecosystems: 0,
        schedules: 0,
        allow: 0,
        sections: 0,
        misc: 0,
    };
    let mut in_updates = false;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if indent == 0 {
            let end = t.find([':', ' ']).unwrap_or(t.len());
            let key = &t[..end];
            match key {
                "updates" => {
                    c.sections += 1;
                    in_updates = true;
                }
                "registries" | "version" | "enable-beta-ecosystems" | "ecosystems" => {
                    c.sections += 1;
                    in_updates = false;
                }
                _ => {
                    c.misc += 1;
                    in_updates = false;
                }
            }
            continue;
        }
        let body = t.strip_prefix('-').map_or(t, |r| r.trim_start());
        let name = body
            .find([':', ' '])
            .map_or(body, |i| &body[..i])
            .trim_matches('"')
            .trim_matches('\'');
        match name {
            "package-ecosystem" => c.ecosystems += 1,
            "schedule" | "interval" | "day" | "time" | "timezone" => {
                c.schedules += 1;
            }
            "allow" | "ignore" | "groups" | "versioning-strategy" => {
                c.allow += 1;
            }
            "directory"
            | "directories"
            | "target-branch"
            | "commit-message"
            | "assignees"
            | "reviewers"
            | "labels"
            | "milestone"
            | "pull-request-branch-name"
            | "open-pull-requests-limit"
            | "open-reviews-limit"
            | "rebase-strategy"
            | "vendor"
            | "insecure-external-code-execution"
            | "registries" => {
                if in_updates {
                    c.misc += 1;
                }
            }
            _ => {
                if !in_updates {
                    c.misc += 1;
                }
            }
        }
    }
    (c.ecosystems >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn dependabot() {
        let cfg = b"version: 2\nupdates:\n  - package-ecosystem: cargo\n    directory: /\n    schedule:\n      interval: weekly\n      day: monday\n      time: \"09:00\"\n      timezone: UTC\n    allow:\n      - dependency-name: x\n    ignore:\n      - dependency-name: y\n  - package-ecosystem: npm\n    directory: /web\nregistries:\n  npm:\n    type: registry\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.ecosystems, 2);
        assert_eq!(c.schedules, 5);
        assert_eq!(c.allow, 2);
        assert_eq!(c.sections, 3);
        assert!(c.misc >= 4);
    }

    #[test]
    fn not_dependabot() {
        assert!(parse(b"foo: 1\n").is_none());
    }
}

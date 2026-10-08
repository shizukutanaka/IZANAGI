//! Gitea/Forgejo Actions ワークフロー(`.gitea/workflows/*.yml`、
//! GitHub Actions 互換構文)の検出と構造カウント。
//!
//! `name:`/`run-name:`/`on:`/`env:`/`jobs:`/`defaults:`/`permissions:`/
//! `concurrency:` トップキー、`jobs:` 内のジョブ ID、`runs-on:`/`steps:`/
//! `needs:`/`strategy:`/`uses:`/`with:`/`if:` ジョブ内キー、および
//! `steps:` 内の `- name:`/`- uses:`/`- run:` ステップを識別する。
//!
//! ```
//! let c = izanagi_kit::giteaaction::parse(
//!     b"on: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo test\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.jobs, 1);
//! assert_eq!(c.steps, 1);
//! assert!(izanagi_kit::giteaaction::detect(b"on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: make\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "concurrency",
    "defaults",
    "env",
    "jobs",
    "name",
    "on",
    "permissions",
    "run-name",
];

/// ジョブブロック内の既知キー。
const JOB_KEYS: &[&str] = &[
    "concurrency",
    "container",
    "continue-on-error",
    "defaults",
    "env",
    "if",
    "name",
    "needs",
    "outputs",
    "permissions",
    "runs-on",
    "secrets",
    "services",
    "steps",
    "strategy",
    "timeout-minutes",
    "uses",
    "with",
    "working-directory",
];

/// giteaaction 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルキー行。
    pub sections: usize,
    /// `jobs:` 内のジョブ ID 行。
    pub jobs: usize,
    /// `steps:` 内の `- ` ステップ要素。
    pub steps: usize,
    /// その他のネスト `key:` 行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// `key:` 先頭のキー名。
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
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が Gitea Actions ワークフローかどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0;
    let mut in_jobs = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_jobs = t == "jobs:";
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                hits += 1;
            }
        } else if in_jobs && yaml_key(t).is_some_and(|k| JOB_KEYS.contains(&k)) {
            hits += 1;
        }
    }
    hits >= 3
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        jobs: 0,
        steps: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_jobs = false;
    let mut in_steps = false;
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
            in_jobs = t == "jobs:";
            in_steps = false;
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if in_jobs {
            if indent == 2 && yaml_key(t).is_some() {
                // ジョブ ID 行(例: `build:`)
                in_steps = false;
                c.jobs += 1;
                continue;
            }
            if t == "steps:" {
                in_steps = true;
                c.options += 1;
                continue;
            }
            if in_steps && t.starts_with("- ") {
                c.steps += 1;
                continue;
            }
        }
        let item = t.strip_prefix("- ").map_or(t, |s| s);
        if yaml_key(item).is_some() {
            c.options += 1;
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1 && c.jobs >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# gitea action\nname: CI\non: [push]\nenv:\n  RUST_BACKTRACE: \"1\"\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: test\n        run: cargo test\n  lint:\n    needs: build\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo clippy\n";

    #[test]
    fn giteaaction() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.jobs, 2);
        assert_eq!(c.steps, 3);
        assert!(c.options >= 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_giteaaction() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"on: push\njobs: {}\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

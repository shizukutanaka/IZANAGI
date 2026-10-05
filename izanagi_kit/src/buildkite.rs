//! Buildkite パイプライン(`pipeline.yml` / `.buildkite/*.yml`)の検出と構造カウント。
//!
//! `steps:`/`env:`/`notify:`/`agents:` トップキーと、ステップ種別
//! (`command`/`wait`/`block`/`input`/`select`/`trigger`/`group`)・
//! ステップ内キー(`label`/`key`/`depends_on`/`agents`/`plugins`/`retry`/
//! `soft_fail`/`timeout_in_minutes`/`parallelism`/`matrix`/`if`/`skip`)を識別する。
//!
//! ```
//! let c = izanagi_kit::buildkite::parse(
//!     b"steps:\n  - command: cargo test\n    label: test\n  - wait\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.steps, 2);
//! assert!(izanagi_kit::buildkite::detect(b"steps:\n  - command: make\n    label: build\n  - wait\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &["agents", "env", "image", "notify", "steps"];

/// ステップ種別(単独リスト要素として現れるもの)。
const STEP_KINDS: &[&str] = &[
    "block", "command", "group", "input", "select", "trigger", "wait",
];

/// ステップ内の既知キー。
const STEP_KEYS: &[&str] = &[
    "agents",
    "allow_dependency_failure",
    "artifact_paths",
    "branches",
    "cancel_on_build_failing",
    "command",
    "concurrency",
    "concurrency_group",
    "depends_on",
    "env",
    "fields",
    "if",
    "if_changed",
    "key",
    "label",
    "matrix",
    "notify",
    "parallelism",
    "plugins",
    "priority",
    "prompt",
    "retry",
    "signature",
    "skip",
    "soft_fail",
    "timeout_in_minutes",
    "trigger",
    "type",
];

/// buildkite 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルキー行。
    pub sections: usize,
    /// `steps:` 内のステップ要素(`- ` 行)。
    pub steps: usize,
    /// ステップ内/その他のネスト `key:` 行。
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

/// b が Buildkite パイプラインかどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    let mut in_steps = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_steps = t == "steps:";
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                hits += 1;
            }
        } else if in_steps && t.starts_with("- ") {
            let item = &t[2..];
            if STEP_KINDS.contains(&item) || yaml_key(item).is_some_and(|k| STEP_KEYS.contains(&k))
            {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        steps: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
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
            in_steps = t == "steps:";
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if t.starts_with("- ") {
            if in_steps {
                c.steps += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if yaml_key(t).is_some() {
            c.options += 1;
            continue;
        }
        c.misc += 1;
    }
    (c.sections + c.steps >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# buildkite\nenv:\n  RUST_BACKTRACE: \"1\"\nsteps:\n  - label: build\n    command: cargo build --release\n    key: build\n    agents:\n      queue: linux\n  - wait\n  - trigger: deploy\n    depends_on: build\n  - block: Release\nnotify:\n  - email: team@example.com\n";

    #[test]
    fn buildkite() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.steps, 4);
        assert!(c.options >= 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_buildkite() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"steps:\n  - run: something\n"));
    }
}

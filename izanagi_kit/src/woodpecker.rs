//! Woodpecker CI 設定(`.woodpecker.yml` / `.woodpecker/*.yml`)の検出と構造カウント。
//!
//! `steps:`/`pipeline:`/`services:`/`clone:`/`workspace:`/`platform:`/`branches:`/
//! `matrix:`/`depends_on:`/`when:` トップキーと、ステップ内キー
//! (`name`/`image`/`commands`/`settings`/`environment`/`secrets`/`volumes`/
//! `privileged`/`failure`/`group`/`detach`)を識別する。
//!
//! ```
//! let c = izanagi_kit::woodpecker::parse(
//!     b"steps:\n  build:\n    image: rust\n    commands:\n      - cargo test\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.steps, 1);
//! assert!(izanagi_kit::woodpecker::detect(b"steps:\n  test:\n    image: rust\n    commands: cargo test\n"));
//! ```

use crate::textutil::strip_bom;
/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "base",
    "branches",
    "clone",
    "depends_on",
    "dir",
    "environment",
    "event",
    "labels",
    "matrix",
    "network_mode",
    "pipeline",
    "platform",
    "runs_on",
    "services",
    "skip_clone",
    "steps",
    "volumes",
    "when",
    "workspace",
];

/// ステップ/サービスブロック内の既知キー。
const STEP_KEYS: &[&str] = &[
    "backend_options",
    "commands",
    "depends_on",
    "detach",
    "dns",
    "entrypoint",
    "environment",
    "failure",
    "group",
    "image",
    "network_mode",
    "ports",
    "privileged",
    "pull",
    "secrets",
    "settings",
    "shell",
    "tmpfs",
    "user",
    "volumes",
    "when",
];

/// woodpecker 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルキー行。
    pub sections: usize,
    /// ステップ名行(`steps:`/`pipeline:`/`services:` 直下の `name:` キー)。
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

/// b が .woodpecker.yml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k) || STEP_KEYS.contains(&k)) {
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
            in_steps = matches!(t, "steps:" | "pipeline:" | "services:");
            if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let item = t.strip_prefix("- ").map_or(t, |s| s);
        if let Some(k) = yaml_key(item) {
            if in_steps && indent == 2 && !STEP_KEYS.contains(&k) {
                // ステップ名行(例: `build:`)
                c.steps += 1;
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

    const SAMPLE: &[u8] = b"# woodpecker\nwhen:\n  event: push\nsteps:\n  build:\n    image: rust:latest\n    commands:\n      - cargo build\n      - cargo test\n    volumes:\n      - cargo-cache:/root/.cargo\n  lint:\n    image: rust:latest\n    group: check\n    failure: ignore\nservices:\n  db:\n    image: postgres:16\n    environment:\n      POSTGRES_PASSWORD: pass\n";

    #[test]
    fn woodpecker() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.steps, 3);
        assert!(c.options >= 4);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_woodpecker() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"steps:\n  - just-a-list\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

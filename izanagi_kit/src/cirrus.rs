//! Cirrus CI 設定(`.cirrus.yml`)の検出と構造カウント。
//!
//! `<name>_task:` タスク定義・`*_script:` フィールド・`env:`/`container:`/
//! `dockerfile:`/`depends_on:`/`only_if:`/`matrix:`/`alias:`/`trigger_type:`/
//! `auto_cancellation:`/`use_compute_credits:`/`persistent_worker:` 等の
//! 既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::cirrus::parse(
//!     b"test_task:\n  container:\n    image: rust:latest\n  test_script: cargo test\n").unwrap();
//! assert_eq!(c.tasks, 1);
//! assert!(izanagi_kit::cirrus::detect(b"test_task:\n  container:\n    image: rust\n  script: cargo test\n"));
//! ```

/// サフィックスでタスク/特殊ブロックを判定する語。
const TASK_SUFFIXES: &[&str] = &["_task", "_pipe", "_pipe_template", "_template"];

/// その他の既知トップ/ネストキー。
const KEYS: &[&str] = &[
    "alias",
    "allow_failures",
    "auto_cancellation",
    "background_script",
    "build_on_push",
    "cleanup_script",
    "compute_engine_instance",
    "container",
    "cpu",
    "depends_on",
    "docker_builder",
    "dockerfile",
    "ec2_instance",
    "eks_container",
    "env",
    "environment",
    "execution_lock",
    "experimental_features",
    "freebsd_instance",
    "gce_instance",
    "gke_container",
    "image",
    "image_family",
    "instance",
    "kubernetes_container",
    "macos_instance",
    "matrix",
    "memory",
    "name",
    "namespace",
    "only_if",
    "osx_image",
    "persistent_worker",
    "platform",
    "prebuilt_image",
    "prepull_script",
    "scheduler_timeout",
    "secrets",
    "skip",
    "stateful_timeout",
    "timeout_in",
    "trigger_type",
    "use_compute_credits",
    "use_in_memory_disk",
    "windows_container",
    "windows_instance",
    "worker",
];

/// サフィックスでスクリプト系フィールドを判定する語。
const SCRIPT_SUFFIXES: &[&str] = &["_script", "_artifacts", "_cache", "_instructions"];

/// Cirrus 固有キー(汎用の `container:`/`image:`/`env:` 等との区別に
/// 最低1件要求)。`*_task` 系サフィックスは常に固有として扱う。
const EXCLUSIVE_KEYS: &[&str] = &[
    "auto_cancellation",
    "build_on_push",
    "compute_engine_instance",
    "docker_builder",
    "ec2_instance",
    "eks_container",
    "execution_lock",
    "experimental_features",
    "freebsd_instance",
    "gce_instance",
    "gke_container",
    "only_if",
];

/// cirrus 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `*_task:`/`*_pipe:`/`*_template:` ブロック数。
    pub tasks: usize,
    /// `*_script:`/`*_cache:`/`*_artifacts:` 等フィールド行。
    pub fields: usize,
    /// その他の既知/未知 `key:` 行。
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

/// b が .cirrus.yml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0;
    let mut exclusive = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some(k) = yaml_key(t) {
            if TASK_SUFFIXES.iter().any(|s| k.ends_with(s)) {
                hits += 1;
                exclusive += 1;
            } else if SCRIPT_SUFFIXES.iter().any(|s| k.ends_with(s)) || KEYS.contains(&k) {
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
        tasks: 0,
        fields: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let item = t.strip_prefix("- ").map_or(t, |s| s);
        if let Some(k) = yaml_key(item) {
            if TASK_SUFFIXES.iter().any(|s| k.ends_with(s)) {
                c.tasks += 1;
            } else if SCRIPT_SUFFIXES.iter().any(|s| k.ends_with(s)) || KEYS.contains(&k) {
                c.fields += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.tasks >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# cirrus\nenv:\n  CARGO_TERM_COLOR: always\ntest_task:\n  container:\n    image: rust:latest\n    cpu: 4\n  build_script: cargo build\n  test_script: cargo test\n  depends_on:\n    - lint\nlint_task:\n  container:\n    image: rust:latest\n  lint_script: cargo clippy\n  only_if: $CIRRUS_BRANCH == \"main\"\n";

    #[test]
    fn cirrus() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tasks, 2);
        assert!(c.fields >= 3);
        assert!(c.options >= 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_cirrus() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"env:\n  A: 1\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

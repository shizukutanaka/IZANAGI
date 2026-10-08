//! Tekton CI パイプライン(`tekton.dev/`・`triggers.tekton.dev/`)の
//! 検出と構造カウント。
//!
//! `apiVersion:` の値に `tekton.dev/` を含み `kind:` が `Task`/`Pipeline`/
//! `PipelineRun`/`TaskRun`/`ClusterTask`/`TriggerTemplate`/`TriggerBinding`/
//! `ClusterTriggerBinding`/`EventListener`/`StepAction` のいずれか。
//!
//! ```
//! let b = b"apiVersion: tekton.dev/v1\nkind: Task\nmetadata:\n  name: build\nspec:\n  steps:\n    - name: compile\n      image: golang\n      script: go build ./...\n";
//! assert!(izanagi_kit::tekton::detect(b));
//! let c = izanagi_kit::tekton::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "Task");
//! assert_eq!(c.steps, 1);
//! ```

use crate::textutil::yaml_val;
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `- key:` リストアイテム形でも `key:` か。
fn is_item(tr: &str, key: &str) -> bool {
    is_key(tr.trim_start_matches('-').trim_start(), key)
}

/// `kind:` の値が Tekton リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Tekton `kind:` 値。
const KINDS: &[&str] = &[
    "ClusterTask",
    "ClusterTriggerBinding",
    "EventListener",
    "Pipeline",
    "PipelineResource",
    "PipelineRun",
    "StepAction",
    "Task",
    "TaskRun",
    "TriggerBinding",
    "TriggerTemplate",
];

/// `spec:` 配下の代表的な Tekton キー。
const SPEC_KEYS: &[&str] = &[
    "description",
    "displayName",
    "finally",
    "params",
    "pipelineRef",
    "pipelineSpec",
    "podTemplate",
    "resources",
    "results",
    "serviceAccountName",
    "sidecars",
    "steps",
    "taskRef",
    "taskSpec",
    "tasks",
    "timeout",
    "timeouts",
    "volumes",
    "workspaces",
];

/// Tekton マニフェストの構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// 検出された `kind:` 値(非検出時は空文字列)。
    pub kind: String,
    /// `spec:` 配下の既知キー行数。
    pub spec_keys: usize,
    /// `- name:` アイテムを伴う `steps:`/`tasks:` 数。
    pub steps: usize,
    /// `---` ドキュメント区切り数。
    pub documents: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が Tekton マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.contains("tekton.dev/")));
    api && kind_val(t).is_some()
}

/// `b` を Tekton マニフェストとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        kind: kind_val(t).unwrap_or("").to_string(),
        spec_keys: 0,
        steps: 0,
        documents: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_steps = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if tr == "---" {
            c.documents += 1;
            continue;
        }
        if is_key(tr, "steps") || is_key(tr, "tasks") {
            in_steps = true;
            c.spec_keys += 1;
            continue;
        }
        if in_steps && is_item(tr, "name") {
            c.steps += 1;
            continue;
        }
        if in_steps && tr.starts_with('-') && !is_item(tr, "name") {
            in_steps = false;
        }
        if SPEC_KEYS.iter().any(|k| is_key(tr, k)) {
            c.spec_keys += 1;
            continue;
        }
        c.misc += 1;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"apiVersion: tekton.dev/v1\nkind: Task\nmetadata:\n  name: build\nspec:\n  steps:\n    - name: compile\n      image: golang\n      script: go build ./...\n    - name: test\n      image: golang\n      script: go test ./...\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: tekton.dev/v1beta1\nkind: PipelineRun\nmetadata:\n  name: pr\n"
        ));
        assert!(detect(
            b"apiVersion: triggers.tekton.dev/v1beta1\nkind: EventListener\nmetadata:\n  name: el\n"
        ));
        assert!(!detect(
            b"apiVersion: v1\nkind: Task\nmetadata:\n  name: t\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "Task");
        assert_eq!(c.steps, 2);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

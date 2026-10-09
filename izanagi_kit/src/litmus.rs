//! LitmusChaos(`litmuschaos.io/`)カオスエンジニアリングマニフェストの
//! 検出と構造カウント。
//!
//! `kind:` が `ChaosEngine`/`ChaosExperiment`/`ChaosResult`。
//!
//! ```
//! let b = b"apiVersion: litmuschaos.io/v1alpha1\nkind: ChaosEngine\nmetadata:\n  name: engine\nspec:\n  appinfo:\n    appns: default\n    applabel: app=nginx\n  engineState: active\n  chaosServiceAccount: litmus\n  experiments:\n    - name: pod-delete\n";
//! assert!(izanagi_kit::litmus::detect(b));
//! let c = izanagi_kit::litmus::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "ChaosEngine");
//! ```

use crate::textutil::yaml_val;
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `- name:` リストアイテム形でも `key:` か。
fn is_item(tr: &str, key: &str) -> bool {
    is_key(tr.trim_start_matches('-').trim_start(), key)
}

/// `kind:` の値が Litmus リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Litmus `kind:` 値。
const KINDS: &[&str] = &["ChaosEngine", "ChaosExperiment", "ChaosResult"];

/// `spec:`/`spec.definition` 配下の代表的な Litmus キー。
const SPEC_KEYS: &[&str] = &[
    "annotationCheck",
    "appinfo",
    "applabel",
    "appns",
    "args",
    "chaosServiceAccount",
    "command",
    "components",
    "definition",
    "engineState",
    "env",
    "experiments",
    "jobCleanUpPolicy",
    "labels",
    "name",
    "permissions",
    "probe",
    "securityContext",
    "statusCheckTimeouts",
    "tolerations",
];

/// Litmus マニフェストの構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// 検出された `kind:` 値(非検出時は空文字列)。
    pub kind: String,
    /// `experiments:`/`probe:` 内の `- name:` アイテム数。
    pub experiments: usize,
    /// 既知キー行数。
    pub spec_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が Litmus マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t.lines().any(|l| {
        yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.starts_with("litmuschaos.io/"))
    });
    api && kind_val(t).is_some()
}

/// `b` を Litmus マニフェストとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        kind: kind_val(t).unwrap_or("").to_string(),
        experiments: 0,
        spec_keys: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_experiments = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_key(tr, "experiments") || is_key(tr, "probe") {
            in_experiments = true;
            c.spec_keys += 1;
            continue;
        }
        if in_experiments && is_item(tr, "name") {
            c.experiments += 1;
            continue;
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

    const SAMPLE: &[u8] = b"apiVersion: litmuschaos.io/v1alpha1\nkind: ChaosEngine\nmetadata:\n  name: engine\nspec:\n  appinfo:\n    appns: default\n    applabel: app=nginx\n  engineState: active\n  chaosServiceAccount: litmus\n  experiments:\n    - name: pod-delete\n      spec:\n        components:\n          env: []\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: litmuschaos.io/v1alpha1\nkind: ChaosExperiment\nmetadata:\n  name: e\n"
        ));
        assert!(!detect(
            b"apiVersion: litmuschaos.io/v1alpha1\nkind: Pod\nmetadata:\n  name: x\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "ChaosEngine");
        assert_eq!(c.experiments, 1);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

//! OPA Gatekeeper(`templates.gatekeeper.sh/`/`constraints.gatekeeper.sh/`/
//! `config.gatekeeper.sh/`/`mutations.gatekeeper.sh/`/
//! `expansion.gatekeeper.sh/`)マニフェストの検出と構造カウント。
//!
//! `kind:` が `ConstraintTemplate`/`Config`/`SyncSet`/`Assign`/`AssignMetadata`/
//! `AssignImage`/`ExpansionTemplate`/`ModifySet`/`MutatorPodStatus`、または
//! `constraints.gatekeeper.sh/` 配下の `K8s*` コミュニティ制約 kind。
//!
//! ```
//! let b = b"apiVersion: templates.gatekeeper.sh/v1\nkind: ConstraintTemplate\nmetadata:\n  name: k8srequiredlabels\nspec:\n  crd:\n    spec:\n      names:\n        kind: K8sRequiredLabels\n  targets:\n    - target: admission.k8s.gatekeeper.sh\n";
//! assert!(izanagi_kit::gatekeeper::detect(b));
//! let c = izanagi_kit::gatekeeper::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "ConstraintTemplate");
//! ```

use crate::textutil::yaml_val;
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `kind:` の値が Gatekeeper リソース種別かどうか
/// (`K8s*` は制約ライブラリの慣例命名)。
fn kind_val(t: &str) -> Option<String> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        if KINDS.contains(&v) || v.starts_with("K8s") {
            Some(v.to_string())
        } else {
            None
        }
    })
}

/// 認識する Gatekeeper `kind:` 値。
const KINDS: &[&str] = &[
    "Assign",
    "AssignImage",
    "AssignMetadata",
    "Config",
    "ConstraintTemplate",
    "ExpansionTemplate",
    "ExpansionTemplatePodStatus",
    "ModifySet",
    "MutatorPodStatus",
    "SyncSet",
];

/// `spec:` 配下の代表的な Gatekeeper キー。
const SPEC_KEYS: &[&str] = &[
    "applyTo",
    "crd",
    "enforcementAction",
    "location",
    "match",
    "mutator",
    "parameters",
    "rego",
    "source",
    "targets",
    "validation",
    "violations",
];

/// Gatekeeper マニフェストの構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// 検出された `kind:` 値(非検出時は空文字列)。
    pub kind: String,
    /// `spec:`/`targets:` 配下の既知キー行数。
    pub spec_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が Gatekeeper マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.contains("gatekeeper.sh/")));
    api && kind_val(t).is_some()
}

/// `b` を Gatekeeper マニフェストとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        kind: kind_val(t).unwrap_or_default(),
        spec_keys: 0,
        comments: 0,
        misc: 0,
    };
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
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

    const SAMPLE: &[u8] = b"apiVersion: templates.gatekeeper.sh/v1\nkind: ConstraintTemplate\nmetadata:\n  name: k8srequiredlabels\nspec:\n  crd:\n    spec:\n      names:\n        kind: K8sRequiredLabels\n  targets:\n    - target: admission.k8s.gatekeeper.sh\n      rego: |\n        package k8srequiredlabels\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        // 制約インスタンス(コミュニティ kind)。
        assert!(detect(
            b"apiVersion: constraints.gatekeeper.sh/v1beta1\nkind: K8sRequiredLabels\nmetadata:\n  name: req\n"
        ));
        assert!(!detect(
            b"apiVersion: templates.gatekeeper.sh/v1\nkind: Pod\nmetadata:\n  name: x\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "ConstraintTemplate");
        assert!(c.spec_keys >= 2);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

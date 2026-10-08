//! Chaos Mesh(`chaos-mesh.org/`)カオス実験マニフェストの検出と
//! 構造カウント。
//!
//! `kind:` が `PodChaos`/`NetworkChaos`/`StressChaos`/`IOChaos`/`TimeChaos`/
//! `DNSChaos`/`HTTPChaos`/`JVMChaos`/`KernelChaos`/`BlockChaos`/
//! `PhysicalMachineChaos`/`Schedule`/`Workflow`/`WorkflowNode`/
//! `ChaosSchedule`/`RemoteCluster`/`StatusCheck`。
//!
//! ```
//! let b = b"apiVersion: chaos-mesh.org/v1alpha1\nkind: PodChaos\nmetadata:\n  name: pod-failure\nspec:\n  action: pod-failure\n  mode: one\n  duration: 30s\n  selector:\n    namespaces: [default]\n";
//! assert!(izanagi_kit::chaosmesh::detect(b));
//! let c = izanagi_kit::chaosmesh::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "PodChaos");
//! ```

use crate::textutil::yaml_val;
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `kind:` の値が Chaos Mesh リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Chaos Mesh `kind:` 値。
const KINDS: &[&str] = &[
    "AWSChaos",
    "AzureChaos",
    "BlockChaos",
    "ChaosSchedule",
    "DNSChaos",
    "GCPChaos",
    "HTTPChaos",
    "IOChaos",
    "JVMChaos",
    "KernelChaos",
    "NetworkChaos",
    "PhysicalMachineChaos",
    "PodChaos",
    "PodHttpChaos",
    "PodIOChaos",
    "PodNetworkChaos",
    "RemoteCluster",
    "Schedule",
    "StatusCheck",
    "StressChaos",
    "TimeChaos",
    "Workflow",
    "WorkflowNode",
];

/// `spec:` 配下の代表的な Chaos Mesh キー。
const SPEC_KEYS: &[&str] = &[
    "action",
    "bandwidth",
    "concurrencyPolicy",
    "containerNames",
    "correlation",
    "delay",
    "direction",
    "duration",
    "externalTargets",
    "historyLimit",
    "jitter",
    "loss",
    "mode",
    "rate",
    "remoteCluster",
    "schedule",
    "selector",
    "stressors",
    "tasks",
    "templates",
    "type",
    "value",
];

/// Chaos Mesh マニフェストの構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// 検出された `kind:` 値(非検出時は空文字列)。
    pub kind: String,
    /// `spec:` 配下の既知キー行数。
    pub spec_keys: usize,
    /// `---` ドキュメント区切り数。
    pub documents: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が Chaos Mesh マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t.lines().any(|l| {
        yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.starts_with("chaos-mesh.org/"))
    });
    api && kind_val(t).is_some()
}

/// `b` を Chaos Mesh マニフェストとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        kind: kind_val(t).unwrap_or("").to_string(),
        spec_keys: 0,
        documents: 0,
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
        if tr == "---" {
            c.documents += 1;
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

    const SAMPLE: &[u8] = b"apiVersion: chaos-mesh.org/v1alpha1\nkind: PodChaos\nmetadata:\n  name: pod-failure\nspec:\n  action: pod-failure\n  mode: one\n  duration: 30s\n  selector:\n    namespaces: [default]\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: chaos-mesh.org/v1alpha1\nkind: NetworkChaos\nmetadata:\n  name: n\n"
        ));
        assert!(detect(
            b"apiVersion: chaos-mesh.org/v1alpha1\nkind: Schedule\nmetadata:\n  name: s\n"
        ));
        assert!(!detect(
            b"apiVersion: chaos-mesh.org/v1alpha1\nkind: Pod\nmetadata:\n  name: x\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "PodChaos");
        assert!(c.spec_keys >= 4);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

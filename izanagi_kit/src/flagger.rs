//! Flagger プログレッシブデリバリー(`flagger.app/`/`*.flagger.app/`)
//! マニフェストの検出と構造カウント。
//!
//! `kind:` が `Canary`/`MetricTemplate`/`AlertProvider`/`Metric`/`Controller`
//! のいずれか。
//!
//! ```
//! let b = b"apiVersion: flagger.app/v1beta1\nkind: Canary\nmetadata:\n  name: app\nspec:\n  targetRef:\n    apiVersion: apps/v1\n    kind: Deployment\n    name: app\n  service:\n    port: 80\n  analysis:\n    interval: 1m\n    threshold: 5\n    maxWeight: 50\n    stepWeight: 10\n";
//! assert!(izanagi_kit::flagger::detect(b));
//! let c = izanagi_kit::flagger::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "Canary");
//! ```

use crate::textutil::yaml_val;
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `kind:` の値が Flagger リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Flagger `kind:` 値。
const KINDS: &[&str] = &[
    "AlertProvider",
    "Canary",
    "Controller",
    "Metric",
    "MetricTemplate",
];

/// `spec:` 配下の代表的な Flagger キー。
const SPEC_KEYS: &[&str] = &[
    "analysis",
    "autoscalerRef",
    "canaryReadyThreshold",
    "finalizers",
    "ingressRef",
    "interval",
    "iterations",
    "maxWeight",
    "metrics",
    "port",
    "primaryReadyThreshold",
    "progressDeadlineSeconds",
    "provider",
    "revertOnDeletion",
    "service",
    "sessionAffinityCookie",
    "skipAnalysis",
    "stepWeight",
    "stepWeights",
    "targetRef",
    "threshold",
    "trafficPolicy",
    "webhooks",
];

/// Flagger マニフェストの構造カウント。
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

/// `b` が Flagger マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.contains("flagger.app/")));
    api && kind_val(t).is_some()
}

/// `b` を Flagger マニフェストとして解析する。
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

    const SAMPLE: &[u8] = b"apiVersion: flagger.app/v1beta1\nkind: Canary\nmetadata:\n  name: app\nspec:\n  targetRef:\n    apiVersion: apps/v1\n    kind: Deployment\n    name: app\n  service:\n    port: 80\n  analysis:\n    interval: 1m\n    threshold: 5\n    maxWeight: 50\n    stepWeight: 10\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: flagger.app/v1beta1\nkind: MetricTemplate\nmetadata:\n  name: m\n"
        ));
        assert!(!detect(
            b"apiVersion: flagger.app/v1beta1\nkind: Pod\nmetadata:\n  name: x\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "Canary");
        assert!(c.spec_keys >= 5);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

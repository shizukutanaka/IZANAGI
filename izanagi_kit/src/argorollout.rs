//! Argo Rollouts(`argoproj.io/v1alpha1` + `Rollout`/`AnalysisTemplate`/
//! `AnalysisRun`/`Experiment`/`ClusterAnalysisTemplate`)マニフェストの
//! 検出と構造カウント。
//!
//! ArgoCD(`Application`/`AppProject`/`ApplicationSet`)とは apiVersion を
//! 共有するが kind 集合が別のため相互に検出しない。
//!
//! ```
//! let b = b"apiVersion: argoproj.io/v1alpha1\nkind: Rollout\nmetadata:\n  name: app\nspec:\n  replicas: 2\n  strategy:\n    canary:\n      steps:\n        - setWeight: 20\n";
//! assert!(izanagi_kit::argorollout::detect(b));
//! let c = izanagi_kit::argorollout::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "Rollout");
//! ```

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `key` の値を `key: value` 行から取り出す。
fn yaml_val<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let l = line.trim_start_matches(['"', '\'']);
    let r = l
        .strip_prefix(key)?
        .trim_start_matches(['"', '\''])
        .trim_start();
    r.strip_prefix(':')
        .map(|v| v.trim().trim_matches('"').trim_matches('\''))
}

/// `kind:` の値が Argo Rollouts リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Rollouts `kind:` 値。
const KINDS: &[&str] = &[
    "AnalysisRun",
    "AnalysisTemplate",
    "ClusterAnalysisTemplate",
    "Experiment",
    "Rollout",
];

/// `spec:` 配下の代表的な Rollouts キー。
const SPEC_KEYS: &[&str] = &[
    "analysis",
    "args",
    "autoPromotionEnabled",
    "blueGreen",
    "canary",
    "dryRun",
    "duration",
    "metrics",
    "progressDeadlineSeconds",
    "replicas",
    "restartAt",
    "revisionHistoryLimit",
    "selector",
    "setWeight",
    "steps",
    "strategy",
    "template",
    "templates",
    "workloadRef",
];

/// Argo Rollouts マニフェストの構造カウント。
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

/// `b` が Argo Rollouts マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.starts_with("argoproj.io/")));
    api && kind_val(t).is_some()
}

/// `b` を Argo Rollouts マニフェストとして解析する。
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

    const SAMPLE: &[u8] = b"apiVersion: argoproj.io/v1alpha1\nkind: Rollout\nmetadata:\n  name: app\nspec:\n  replicas: 2\n  strategy:\n    canary:\n      steps:\n        - setWeight: 20\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: argoproj.io/v1alpha1\nkind: AnalysisTemplate\nmetadata:\n  name: a\n"
        ));
        // ArgoCD の kind 集合とは交差しない。
        assert!(!detect(
            b"apiVersion: argoproj.io/v1alpha1\nkind: Application\nmetadata:\n  name: a\n"
        ));
        assert!(!detect(
            b"apiVersion: v1\nkind: Deployment\nmetadata:\n  name: d\n"
        ));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "Rollout");
        assert!(c.spec_keys >= 3);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

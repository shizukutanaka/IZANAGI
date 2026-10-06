//! FluxCD(GitOps Toolkit)マニフェストの検出と構造カウント。
//!
//! `apiVersion:` の値に `fluxcd.io/` を含み(`source.toolkit.fluxcd.io/`、
//! `kustomize.toolkit.fluxcd.io/`、`helm.toolkit.fluxcd.io/`、
//! `notification.toolkit.fluxcd.io/`、`image.toolkit.fluxcd.io/`)、
//! `kind:` が既知の Flux リソースであることを要求する。
//!
//! ```
//! let b = b"apiVersion: kustomize.toolkit.fluxcd.io/v1\nkind: Kustomization\nmetadata:\n  name: app\n  namespace: flux-system\nspec:\n  interval: 10m\n  path: ./clusters/prod\n  prune: true\n  sourceRef:\n    kind: GitRepository\n    name: flux-system\n";
//! assert!(izanagi_kit::fluxcd::detect(b));
//! let c = izanagi_kit::fluxcd::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "Kustomization");
//! assert!(c.spec_keys >= 4);
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

/// `kind:` の値が Flux リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Flux `kind:` 値。
const KINDS: &[&str] = &[
    "Alert",
    "Bucket",
    "GitRepository",
    "HelmRelease",
    "HelmRepository",
    "ImagePolicy",
    "ImageRepository",
    "ImageUpdateAutomation",
    "Kustomization",
    "OCIRepository",
    "Provider",
    "Receiver",
    "Terraform",
];

/// `spec:` 配下の代表的な Flux キー。
const SPEC_KEYS: &[&str] = &[
    "chart",
    "dependsOn",
    "force",
    "healthChecks",
    "images",
    "interval",
    "kubeConfig",
    "path",
    "postBuild",
    "prune",
    "ref",
    "releaseName",
    "retryInterval",
    "serviceAccountName",
    "sourceRef",
    "suspend",
    "targetNamespace",
    "timeout",
    "url",
    "values",
    "valuesFrom",
    "wait",
];

/// Flux マニフェストの構造カウント。
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

/// `b` が Flux マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.contains("fluxcd.io/")));
    api && kind_val(t).is_some()
}

/// `b` を Flux マニフェストとして解析する。
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

    const SAMPLE: &[u8] = b"apiVersion: kustomize.toolkit.fluxcd.io/v1\nkind: Kustomization\nmetadata:\n  name: app\n  namespace: flux-system\nspec:\n  interval: 10m\n  path: ./clusters/prod\n  prune: true\n  sourceRef:\n    kind: GitRepository\n    name: flux-system\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: helm.toolkit.fluxcd.io/v2\nkind: HelmRelease\nmetadata:\n  name: r\n"
        ));
        // コメント内の言及だけでは検出しない。
        assert!(!detect(
            b"# apiVersion: kustomize.toolkit.fluxcd.io/v1\napiVersion: v1\nkind: Pod\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "Kustomization");
        assert!(c.spec_keys >= 4);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

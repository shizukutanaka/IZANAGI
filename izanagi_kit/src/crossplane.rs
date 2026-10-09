//! Crossplane(`*.crossplane.io/`)マニフェストの検出と構造カウント。
//!
//! `apiVersion:` の値に `crossplane.io/` を含み(`apiextensions.crossplane.io/`、
//! `pkg.crossplane.io/`、プロバイダ固有グループ)、`kind:` が
//! `CompositeResourceDefinition`/`Composition`/`Provider`/`Configuration`/
//! `Function`/`ProviderConfig`/`DeploymentRuntimeConfig` 等。
//!
//! ```
//! let b = b"apiVersion: apiextensions.crossplane.io/v1\nkind: CompositeResourceDefinition\nmetadata:\n  name: xbuckets.example.org\nspec:\n  group: example.org\n  names:\n    kind: XBucket\n    plural: xbuckets\n  versions:\n    - name: v1alpha1\n";
//! assert!(izanagi_kit::crossplane::detect(b));
//! let c = izanagi_kit::crossplane::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "CompositeResourceDefinition");
//! ```

use crate::textutil::yaml_val;
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `kind:` の値が Crossplane リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Crossplane `kind:` 値。
const KINDS: &[&str] = &[
    "CompositeResourceDefinition",
    "Composition",
    "CompositionRevision",
    "Configuration",
    "ConfigurationRevision",
    "DeploymentRuntimeConfig",
    "Function",
    "FunctionRevision",
    "ImageConfig",
    "Lock",
    "Provider",
    "ProviderConfig",
    "ProviderRevision",
    "Usage",
];

/// `spec:` 配下の代表的な Crossplane キー。
const SPEC_KEYS: &[&str] = &[
    "base",
    "claimNames",
    "compositeDeletePolicy",
    "connectionSecretKeys",
    "controller",
    "defaultCompositionRef",
    "enforceCompositionValidation",
    "forProvider",
    "functions",
    "group",
    "mode",
    "names",
    "package",
    "packagePullPolicy",
    "patchSets",
    "pipeline",
    "resources",
    "skipDependencyResolution",
    "versions",
    "writeConnectionSecretToRef",
];

/// Crossplane マニフェストの構造カウント。
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

/// `b` が Crossplane マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.contains("crossplane.io/")));
    api && kind_val(t).is_some()
}

/// `b` を Crossplane マニフェストとして解析する。
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

    const SAMPLE: &[u8] = b"apiVersion: apiextensions.crossplane.io/v1\nkind: CompositeResourceDefinition\nmetadata:\n  name: xbuckets.example.org\nspec:\n  group: example.org\n  names:\n    kind: XBucket\n    plural: xbuckets\n  claimNames:\n    kind: Bucket\n    plural: buckets\n  versions:\n    - name: v1alpha1\n      served: true\n      referenceable: true\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: pkg.crossplane.io/v1\nkind: Provider\nmetadata:\n  name: p\nspec:\n  package: xpkg.upbound.io/upbound/provider-aws:latest\n"
        ));
        assert!(!detect(
            b"apiVersion: apiextensions.crossplane.io/v1\nkind: Pod\nmetadata:\n  name: x\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "CompositeResourceDefinition");
        assert!(c.spec_keys >= 3);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

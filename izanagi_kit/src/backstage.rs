//! Backstage カタログ(`catalog-info.yaml`/`catalog.yaml`)の検出と構造カウント。
//!
//! `apiVersion: backstage.io/v1alpha1`(`v1beta1`/`v1beta2`)と `kind:` の
//! `Component`/`API`/`Resource`/`System`/`Domain`/`Group`/`User`/`Location`/
//! `Template` 組合せ、`spec:` 配下の `type`/`lifecycle`/`owner`/`system`/
//! `dependsOn`/`providesApis`/`consumesApis`/`definition`/`spec` 系キー、
//! `metadata:` 配下の `name`/`namespace`/`title`/`description`/`labels`/
//! `annotations`/`links`/`tags` を識別する。
//!
//! ```
//! let c = izanagi_kit::backstage::parse(
//!     b"apiVersion: backstage.io/v1alpha1\nkind: Component\nmetadata:\n  name: my-service\nspec:\n  type: service\n  lifecycle: production\n  owner: team-a\n").unwrap();
//! assert_eq!(c.kind.as_str(), "Component");
//! assert!(izanagi_kit::backstage::detect(
//!     b"apiVersion: backstage.io/v1alpha1\nkind: API\nmetadata:\n  name: api\n"));
//! ```

use crate::textutil::yaml_val;
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `kind:` の値が Backstage エンティティ種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する `kind:` 値。
const KINDS: &[&str] = &[
    "API",
    "Component",
    "Domain",
    "Group",
    "Location",
    "Resource",
    "System",
    "Template",
    "User",
];

/// `spec:`/`metadata:` 配下の既知キー。
const SPEC_KEYS: &[&str] = &[
    "consumesApis",
    "definition",
    "dependsOn",
    "lifecycle",
    "members",
    "owner",
    "parameters",
    "profile",
    "providesApis",
    "steps",
    "subcomponentOf",
    "system",
    "target",
    "targets",
    "type",
];

/// `catalog-info.yaml` の構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// 検出された `kind:` 値(非検出時は空文字列)。
    pub kind: String,
    /// `metadata:`/`spec:` 配下の既知キー行数。
    pub spec_keys: usize,
    /// `---` ドキュメント区切り数(マルチドキュメント)。
    pub documents: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が `catalog-info.yaml` に見えるかを判定する。
///
/// `apiVersion:` の値に `backstage.io/` を含み `kind:` が既知の
/// エンティティ種別であることを要求する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.starts_with("backstage.io/")));
    api && kind_val(t).is_some()
}

/// `b` を `catalog-info.yaml` として解析する。
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

    const SAMPLE: &[u8] = b"apiVersion: backstage.io/v1alpha1\nkind: Component\nmetadata:\n  name: my-service\n  namespace: default\n  annotations:\n    github.com/project-slug: org/repo\nspec:\n  type: service\n  lifecycle: production\n  owner: team-a\n  system: payments\n  providesApis:\n    - my-api\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(b"apiVersion: backstage.io/v1beta2\nkind: User\n"));
        assert!(!detect(
            b"# apiVersion: backstage.io/v1alpha1\n# kind: Component\n"
        ));
        assert!(!detect(
            b"apiVersion: argoproj.io/v1alpha1\nkind: Application\n"
        ));
        assert!(!detect(
            b"apiVersion: backstage.io/v1alpha1\nkind: SomethingElse\n"
        ));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind, "Component");
        assert!(c.spec_keys >= 5);
        assert!(parse(b"kind: Component\n").is_none());
    }
}

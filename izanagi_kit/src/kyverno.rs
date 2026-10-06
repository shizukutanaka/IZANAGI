//! Kyverno ポリシー(`kyverno.io/`/`wgpolicyk8s.io/`)マニフェストの
//! 検出と構造カウント。
//!
//! `apiVersion:` の値に `kyverno.io/` または `wgpolicyk8s.io/` を含み
//! `kind:` が `ClusterPolicy`/`Policy`/`PolicyException`/`CleanupPolicy`/
//! `ClusterCleanupPolicy`/`PolicyReport`/`ClusterPolicyReport` 等。
//!
//! ```
//! let b = b"apiVersion: kyverno.io/v1\nkind: ClusterPolicy\nmetadata:\n  name: require-label\nspec:\n  validationFailureAction: Enforce\n  rules:\n    - name: check\n      match:\n        any:\n        - resources:\n            kinds: [Pod]\n";
//! assert!(izanagi_kit::kyverno::detect(b));
//! let c = izanagi_kit::kyverno::parse(b).unwrap();
//! assert_eq!(c.rules, 1);
//! ```

fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

/// `- name:` アイテムか。
fn is_item(tr: &str, key: &str) -> bool {
    is_key(tr.trim_start_matches('-').trim_start(), key)
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

/// `kind:` の値が Kyverno リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Kyverno `kind:` 値。
const KINDS: &[&str] = &[
    "AdmissionReport",
    "BackgroundScanReport",
    "CleanupPolicy",
    "ClusterAdmissionReport",
    "ClusterBackgroundScanReport",
    "ClusterCleanupPolicy",
    "ClusterPolicy",
    "ClusterPolicyReport",
    "GlobalContextEntry",
    "Policy",
    "PolicyException",
    "PolicyReport",
    "UpdateRequest",
];

/// `spec:`/`rules:` 配下の代表的な Kyverno キー。
const SPEC_KEYS: &[&str] = &[
    "any",
    "all",
    "applyRules",
    "background",
    "context",
    "deny",
    "emitWarning",
    "exclude",
    "failurePolicy",
    "generate",
    "match",
    "mutate",
    "pattern",
    "preconditions",
    "resources",
    "rules",
    "validate",
    "validationFailureAction",
    "verifyImages",
    "webhookTimeoutSeconds",
];

/// Kyverno マニフェストの構造カウント。
#[derive(Clone, Debug)]
pub struct Counts {
    /// 検出された `kind:` 値(非検出時は空文字列)。
    pub kind: String,
    /// `rules:` 内の `- name:` ルール数。
    pub rules: usize,
    /// 既知キー行数。
    pub spec_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `b` が Kyverno マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t.lines().any(|l| {
        yaml_val(l.trim(), "apiVersion")
            .is_some_and(|v| v.contains("kyverno.io/") || v.contains("wgpolicyk8s.io/"))
    });
    api && kind_val(t).is_some()
}

/// `b` を Kyverno マニフェストとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let t = core::str::from_utf8(b).unwrap_or("");
    let mut c = Counts {
        kind: kind_val(t).unwrap_or("").to_string(),
        rules: 0,
        spec_keys: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_rules = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_key(tr, "rules") {
            in_rules = true;
            c.spec_keys += 1;
            continue;
        }
        if in_rules && is_item(tr, "name") {
            c.rules += 1;
            continue;
        }
        if SPEC_KEYS.iter().any(|k| is_key(tr, k)) || is_item(tr, "resources") {
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

    const SAMPLE: &[u8] = b"apiVersion: kyverno.io/v1\nkind: ClusterPolicy\nmetadata:\n  name: require-label\nspec:\n  validationFailureAction: Enforce\n  rules:\n    - name: check-label\n      match:\n        any:\n        - resources:\n            kinds: [Pod]\n      validate:\n        message: label required\n        pattern:\n          metadata:\n            labels:\n              app: \"?*\"\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: kyverno.io/v2beta1\nkind: Policy\nmetadata:\n  name: p\n"
        ));
        assert!(detect(
            b"apiVersion: wgpolicyk8s.io/v1alpha2\nkind: ClusterPolicyReport\nmetadata:\n  name: r\n"
        ));
        assert!(!detect(
            b"apiVersion: kyverno.io/v1\nkind: Pod\nmetadata:\n  name: x\n"
        ));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.rules, 1);
        assert!(c.spec_keys >= 3);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

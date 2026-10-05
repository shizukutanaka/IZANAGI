//! Harness CI/CD パイプライン(`pipeline.yaml` / `*.harness.yaml`)の
//! 検出と構造カウント。
//!
//! `pipeline:` ルート(`name`/`identifier`/`description`/`tags`/`properties`/
//! `variables`)と `stages:`/`steps:`/`stepGroups:`/`service:`/`infrastructure:`/
//! `environment:` ブロック、および `stage:`/`step:`/`stepGroup:`/`parallel:`/
//! `spec:`/`strategy:`/`repeat:`/`loopingStrategy:` 等の要素キーを識別する。
//!
//! ```
//! let c = izanagi_kit::harness::parse(
//!     b"pipeline:\n  name: ci\n  identifier: ci\n  stages:\n    - stage:\n        name: build\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.elements, 1);
//! assert!(izanagi_kit::harness::detect(b"pipeline:\n  name: ci\n  identifier: ci\n  stages: []\n"));
//! ```

/// ルートコンテナキー(直下に構造を持つ)。
const ROOT_KEYS: &[&str] = &[
    "pipeline",
    "service",
    "infrastructure",
    "environment",
    "inputs",
];

/// ブロック内の要素マーカキー。
const ELEMENT_KEYS: &[&str] = &["stage", "step", "stepGroup", "parallel", "matrix", "repeat"];

/// その他の既知キー。
const KEYS: &[&str] = &[
    "allowStageExecutions",
    "artifacts",
    "clonedFrom",
    "connectorRef",
    "delegateSelectors",
    "description",
    "environmentVariables",
    "failureStrategies",
    "gitClone",
    "identifier",
    "image",
    "infrastructure",
    "inputs",
    "loopingStrategy",
    "name",
    "namespace",
    "onError",
    "orgIdentifier",
    "outputs",
    "projectIdentifier",
    "properties",
    "runtime",
    "spe\u{63}",
    "stages",
    "steps",
    "stepGroups",
    "strategy",
    "tags",
    "template",
    "templateInputs",
    "timeout",
    "type",
    "variables",
    "when",
];

/// harness 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ルートコンテナキー行。
    pub sections: usize,
    /// `stage:`/`step:`/`stepGroup:`/`parallel:`/`matrix:`/`repeat:` 要素行。
    pub elements: usize,
    /// その他の `key:` 行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// `key:` 先頭のキー名。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        None
    } else {
        Some(k)
    }
}

/// b が Harness パイプラインかどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some(k) = yaml_key(t) {
            if ROOT_KEYS.contains(&k) || KEYS.contains(&k) || ELEMENT_KEYS.contains(&k) {
                hits += 1;
            }
        }
    }
    hits >= 3 && text.contains("pipeline:")
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        elements: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let item = t.strip_prefix("- ").map_or(t, |s| s);
        if let Some(k) = yaml_key(item) {
            if ROOT_KEYS.contains(&k) {
                c.sections += 1;
            } else if ELEMENT_KEYS.contains(&k) {
                c.elements += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# harness\npipeline:\n  name: ci\n  identifier: ci\n  projectIdentifier: demo\n  orgIdentifier: default\n  tags: {}\n  stages:\n    - stage:\n        name: build\n        identifier: build\n        type: CI\n        spec:\n          cloneCodebase: true\n          execution:\n            steps:\n              - step:\n                  name: test\n                  identifier: test\n                  type: Run\n                  spec:\n                    command: cargo test\n";

    #[test]
    fn harness() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.elements, 2);
        assert!(c.options >= 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_harness() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"name: x\nidentifier: y\n"));
    }
}

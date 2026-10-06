//! Knative(`serving.knative.dev/`/`eventing.knative.dev/`/
//! `sources.knative.dev/`/`flows.knative.dev/`/`messaging.knative.dev/`)
//! マニフェストの検出と構造カウント。
//!
//! ```
//! let b = b"apiVersion: serving.knative.dev/v1\nkind: Service\nmetadata:\n  name: hello\nspec:\n  template:\n    spec:\n      containers:\n        - image: gcr.io/x/hello\n";
//! assert!(izanagi_kit::knative::detect(b));
//! let c = izanagi_kit::knative::parse(b).unwrap();
//! assert_eq!(c.kind.as_str(), "Service");
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

/// `kind:` の値が Knative リソース種別かどうか。
fn kind_val(t: &str) -> Option<&'static str> {
    t.lines().find_map(|l| {
        let v = yaml_val(l.trim(), "kind")?;
        KINDS.iter().copied().find(|k| *k == v)
    })
}

/// 認識する Knative `kind:` 値。
const KINDS: &[&str] = &[
    "ApiServerSource",
    "Broker",
    "Channel",
    "Configuration",
    "ContainerSource",
    "EventType",
    "InMemoryChannel",
    "KafkaChannel",
    "Parallel",
    "PingSource",
    "Revision",
    "Route",
    "Sequence",
    "Service",
    "SinkBinding",
    "Subscription",
    "Trigger",
];

/// `spec:` 配下の代表的な Knative キー。
const SPEC_KEYS: &[&str] = &[
    "broker",
    "channel",
    "channelTemplate",
    "containers",
    "delivery",
    "filter",
    "port",
    "reply",
    "resources",
    "sink",
    "source",
    "steps",
    "subscriber",
    "template",
    "traffic",
];

/// Knative マニフェストの構造カウント。
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

/// `b` が Knative マニフェストに見えるかを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    let api = t
        .lines()
        .any(|l| yaml_val(l.trim(), "apiVersion").is_some_and(|v| v.contains("knative.dev/")));
    api && kind_val(t).is_some()
}

/// `b` を Knative マニフェストとして解析する。
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

    const SAMPLE: &[u8] = b"apiVersion: serving.knative.dev/v1\nkind: Service\nmetadata:\n  name: hello\nspec:\n  template:\n    spec:\n      containers:\n        - image: gcr.io/x/hello\n";

    #[test]
    fn detect_works() {
        assert!(detect(SAMPLE));
        assert!(detect(
            b"apiVersion: eventing.knative.dev/v1\nkind: Broker\nmetadata:\n  name: b\n"
        ));
        assert!(!detect(
            b"apiVersion: serving.knative.dev/v1\nkind: Pod\nmetadata:\n  name: x\n"
        ));
        assert!(!detect(
            b"apiVersion: v1\nkind: Service\nmetadata:\n  name: s\n"
        ));
    }

    #[test]
    fn parses() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.kind.as_str(), "Service");
        assert!(c.spec_keys >= 2);
        assert!(parse(b"apiVersion: v1\nkind: Pod\n").is_none());
    }
}

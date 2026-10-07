//! KubeMQ クラスタ設定(`kubemq.yaml` / `kubemq-cluster.yaml`)の検出と構造カウント。
//!
//! `apiVersion:`/`kind:`/`metadata:`/`data:`/`config:` 等の YAML 構造キーと、
//! KubeMQ 固有の `address`/`store`/`queues`/`queue`/`pubsub`/`events`/
//! `eventsStore`/`commands`/`queries`/`grpc`/`rest`/`license`/`cluster`/
//! `authentication`/`authorization`/`persistence`/`nodePort` 等のキーを識別する。
//!
//! ```
//! let c = izanagi_kit::kubemq::parse(
//!     b"config: |\n  address: kubemq-cluster-grpc.kubemq.svc:50000\n  store: queues\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.options, 2);
//! assert!(izanagi_kit::kubemq::detect(b"config:\n  address: kubemq-grpc:50000\n  store: queues\n  license: xyz\n"));
//! ```

/// コンテナ/構造キー。
const TOP_KEYS: &[&str] = &[
    "apiVersion",
    "config",
    "data",
    "kind",
    "metadata",
    "stringData",
];

/// KubeMQ 固有キー。
const KEYS: &[&str] = &[
    "address",
    "api",
    "authentication",
    "authorization",
    "binding",
    "certData",
    "cluster",
    "commands",
    "cpu",
    "events",
    "eventsStore",
    "exposeNodePort",
    concat!("grp", "\u{63}"),
    "grpcPort",
    "health",
    "host",
    "image",
    "keyData",
    "license",
    "logLevel",
    "memory",
    "metrics",
    "namespace",
    "nodePort",
    "nats",
    "nats_io",
    "persistence",
    "port",
    "ports",
    "properties",
    "pubsub",
    "queries",
    "queue",
    "queues",
    "replicas",
    "resources",
    "rest",
    "restPort",
    "store",
    "tag",
    "tls",
    "url",
    "volumeClaim",
];

/// kubemq 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 構造キー行(apiVersion/kind/metadata/data/config/stringData)。
    pub sections: usize,
    /// KubeMQ 固有キー行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行(未知キー・リスト要素・継続スカラ)。
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

/// Keys (or inline mentions) that appear only in KubeMQ resources —
/// `apiVersion`/`kind`/`metadata` alone are shared with every
/// Kubernetes manifest and prove nothing on their own.
const EXCLUSIVE_KEYS: &[&str] = &[
    "grpc",
    "grpcPort",
    "exposeNodePort",
    "eventsStore",
    "nats",
    "nats_io",
    "restPort",
    "license",
    "keyData",
    "certData",
];

/// KubemqCluster/Kubemq* カスタムリソースの検出。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    let mut exclusive = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k) || KEYS.contains(&k)) {
            hits += 1;
        }
        if yaml_key(t).is_some_and(|k| EXCLUSIVE_KEYS.contains(&k))
            || t.to_ascii_lowercase().contains("kubemq")
        {
            exclusive += 1;
        }
    }
    hits >= 3 && exclusive >= 1
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
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
            if TOP_KEYS.contains(&k) {
                c.sections += 1;
            } else if KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# kubemq\napiVersion: core.k8s.kubemq.io/v1alpha1\nkind: KubemqCluster\nmetadata:\n  name: kubemq-cluster\nspec:\n  replicas: 3\n  image:\n    repository: docker.io/kubemq/kubemq\n    tag: latest\n  license: |\n    <license data>\n  grpc:\n    port: 50000\n  rest:\n    port: 9090\n  persistence:\n    enabled: true\n  store: queues\n";

    #[test]
    fn kubemq() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert!(c.options >= 8);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 5);
    }

    #[test]
    fn not_kubemq() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }
}

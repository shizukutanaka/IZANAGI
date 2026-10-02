//! OpenSearch `opensearch.yml` の検出と構造カウント。
//!
//! `cluster.name`/`node.*`/`path.*`/`network.*`/`discovery.*`/`plugins.*` 等の
//! ドット付きフラットキーをプレフィックス族別に分類する。
//!
//! ```
//! let c = izanagi_kit::opensearch::parse(
//!     b"cluster.name: prod\nnode.name: n1\npath.data: /var/data\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::opensearch::detect(b"cluster.name: a\nnode.roles: [m]\npath.logs: /l\n"));
//! ```

/// ドット接頭辞の既知ファミリー。
const FAMILIES: &[&str] = &[
    "action",
    "bootstrap",
    "breaking",
    "cluster",
    "compatibility",
    "discovery",
    "gateway",
    "http",
    "indices",
    "ingest",
    "monitor",
    "network",
    "node",
    "path",
    "plugins",
    "processors",
    "repositories",
    "script",
    "search",
    "security",
    "slave",
    "snapshots",
    "thread_pool",
    "transport",
    "xpack",
];
/// 単体既知キー。
const SINGLES: &[&str] = &[
    "cluster.name",
    "cluster.routing.allocation.awareness.attributes",
    "compatibility.override_main_response_version",
    "discovery.seed_hosts",
    "discovery.seed_providers",
    "discovery.type",
    "gateway.expected_nodes",
    "http.compression",
    "http.cors.enabled",
    "http.host",
    "http.max_content_length",
    "http.port",
    "http.type",
    "indices.breaker.total.limit",
    "indices.fielddata.cache.size",
    "indices.memory.index_buffer_size",
    "indices.queries.cache.size",
    "indices.query.bool.max_clause_count",
    "indices.recovery.max_bytes_per_sec",
    "network.bind_host",
    "network.host",
    "network.publish_host",
    "node.attr",
    "node.data",
    "node.ingest",
    "node.master",
    "node.max_local_storage_nodes",
    "node.ml",
    "node.name",
    "node.remote_cluster_client",
    "node.roles",
    "node.store.allow_mmap",
    "path.data",
    "path.home",
    "path.logs",
    "path.plugins",
    "path.repo",
    "path.scripts",
    "path.shared_data",
    "plugins.security.disabled",
    "plugins.security.ssl.http.enabled",
    "reindex.remote.whitelist",
    "script.painless.regex.enabled",
    "search.max_buckets",
    "transport.host",
    "transport.port",
    "transport.type",
];

/// OpenSearch 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー行。
    pub options: usize,
    /// リスト項目 `- `。
    pub items: usize,
    /// ネストされたマップキー(単独 `key:` 行)。
    pub nested: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行の `:` までのキーを返す(値コロンでない最初のコロン)。
fn key_of(t: &str) -> Option<&str> {
    let colon = t.find(':')?;
    let k = t[..colon].trim();
    if k.is_empty() || k.starts_with('-') {
        None
    } else {
        Some(k)
    }
}

/// b が opensearch.yml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            key_of(l.trim()).is_some_and(|k| {
                SINGLES.contains(&k) || k.split('.').next().is_some_and(|f| FAMILIES.contains(&f))
            })
        })
        .count()
        >= 2
}

/// opensearch.yml の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        items: 0,
        nested: 0,
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
        if t.starts_with('-') {
            c.items += 1;
            continue;
        }
        let Some(key) = key_of(t) else {
            c.misc += 1;
            continue;
        };
        let first = key.split('.').next().unwrap_or("");
        if SINGLES.contains(&key) || FAMILIES.contains(&first) {
            if key.split(':').count() > 1 && t.ends_with(':') {
                c.nested += 1;
            } else {
                c.options += 1;
            }
        } else if t.ends_with(':') {
            c.nested += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options + c.nested >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# opensearch.yml\ncluster.name: production\nnode.name: node-1\nnode.roles: [remote_cluster_client]\npath.data: /var/lib/opensearch\npath.logs: /var/log/opensearch\nnetwork.host: 0.0.0.0\ndiscovery.seed_hosts:\n  - host1\n  - host2\nplugins.security.disabled: false\nplugins.security.ssl.http.enabled: true\nhttp.port: 9200\nindices.breaker.total.limit: 75%\nunknown.key: x\n";

    #[test]
    fn opensearch() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 11);
        assert_eq!(c.items, 2);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_opensearch() {
        assert!(!detect(b"key: value\nother: 1\n"));
        assert!(!detect(b"hello\n"));
    }
}

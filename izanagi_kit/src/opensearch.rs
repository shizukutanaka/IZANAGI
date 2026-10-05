//! OpenSearch `opensearch.yml` parser.
//!
//! Detects OpenSearch cluster YAML by dotted key prefixes (`cluster.*`/
//! `node.*`/`network.*`/`http.*`/`transport.*`/`discovery.*`/`path.*`/
//! `plugins.security.*`), and counts structure.
//!
//! ```
//! let b = b"cluster.name: demo\nnode.name: node-1\nnode.roles: [master, data]\nnetwork.host: 0.0.0.0\nhttp.port: 9200\ndiscovery.seed_hosts: [127.0.0.1]\npath.data: /var/lib/opensearch\npath.logs: /var/log/opensearch\n";
//! assert!(izanagi_kit::opensearch::detect(b));
//! let c = izanagi_kit::opensearch::Opensearch::parse(b).unwrap();
//! assert!(c.keys >= 8);
//! ```

/// Parsed opensearch.yml summary.
#[derive(Debug, Clone)]
pub struct Opensearch {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `cluster.*` keys.
    pub cluster_keys: usize,
    /// `node.*` keys.
    pub node_keys: usize,
    /// `network.*`/`http.*`/`transport.*` keys.
    pub network_keys: usize,
    /// `discovery.*` keys.
    pub discovery_keys: usize,
    /// `path.*` keys.
    pub path_keys: usize,
    /// `plugins.security.*` keys.
    pub security_keys: usize,
    /// `key: value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// `cluster.*` keys.
const CLUSTER_KEYS: &[&str] = &[
    "cluster.name",
    "cluster.initial_master_nodes",
    "cluster.routing.allocation.cluster_concurrent_rebalance",
    "cluster.routing.allocation.disk.threshold_enabled",
    "cluster.routing.allocation.disk.watermark.low",
    "cluster.routing.allocation.disk.watermark.high",
    "cluster.routing.allocation.disk.watermark.flood_stage",
    "cluster.routing.allocation.enable",
    "cluster.routing.allocation.awareness.attributes",
    "cluster.routing.rebalance.enable",
    "cluster.remote_cluster.connect",
    "cluster.remote.initial_mode",
];

/// `node.*` keys.
const NODE_KEYS: &[&str] = &[
    "node.name",
    "node.master",
    "node.data",
    "node.ingest",
    "node.roles",
    "node.attr",
    "node.max_local_storage_nodes",
    "node.ml",
    "node.search.cache.size",
    "node.store.allow_mmap",
    "node.processors",
];

/// `network.*`/`http.*`/`transport.*` keys.
const NETWORK_KEYS: &[&str] = &[
    "network.host",
    "network.bind_host",
    "network.publish_host",
    "network.breaker.inflight_requests.limit",
    "http.port",
    "http.bind_host",
    "http.publish_host",
    "http.compression",
    "http.max_content_length",
    "http.max_initial_line_length",
    "http.max_header_size",
    "http.cors.enabled",
    "http.cors.allow-origin",
    "http.cors.allow-methods",
    "http.cors.allow-headers",
    "http.cors.allow-credentials",
    "http.type",
    "transport.port",
    "transport.tcp.port",
    "transport.bind_host",
    "transport.publish_host",
    "transport.tcp.compress",
    "transport.ssl",
    "transport.type",
];

/// `discovery.*` keys.
const DISCOVERY_KEYS: &[&str] = &[
    "discovery.seed_hosts",
    "discovery.seed_providers",
    "discovery.zen.ping.unicast.hosts",
    "discovery.type",
    "discovery.ec2",
    "discovery.gce",
    "discovery.azure",
];

/// `path.*` keys.
const PATH_KEYS: &[&str] = &[
    "path.data",
    "path.logs",
    "path.repo",
    "path.shared_data",
    "path.scripts",
];

/// `plugins.security.*` keys.
const SECURITY_KEYS: &[&str] = &[
    "plugins.security.disabled",
    "plugins.security.ssl",
    "plugins.security.allow_default_init_securityindex",
    "plugins.security.allow_unsafe_democertificates",
    "plugins.security.audit",
    "plugins.security.restapi",
    "plugins.security.cert",
    "plugins.security.http",
    "plugins.security.nodes_dn",
    "plugins.security.authcz",
    "plugins.security.cache",
    "plugins.security.unsupported",
];

/// Additional singleton keys.
const MISC_KEYS: &[&str] = &[
    "action.auto_create_index",
    "action.destructive_requires_name",
    "bootstrap.memory_lock",
    "bootstrap.system_call_filter",
    "indices.recovery.max_bytes_per_se\u{63}",
    "indices.query.bool.max_clause_count",
    "indices.fielddata.cache.size",
    "indices.breaker.total.limit",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect an OpenSearch `opensearch.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = CLUSTER_KEYS
        .iter()
        .chain(NODE_KEYS.iter())
        .chain(NETWORK_KEYS.iter())
        .chain(DISCOVERY_KEYS.iter())
        .chain(PATH_KEYS.iter())
        .chain(SECURITY_KEYS.iter())
        .chain(MISC_KEYS.iter())
        .filter(|k| key_present(t, k))
        .count();
    hits >= 2
}

impl Opensearch {
    /// Count categories. Returns `None` when the input does not look like
    /// an opensearch.yml.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            cluster_keys: 0,
            node_keys: 0,
            network_keys: 0,
            discovery_keys: 0,
            path_keys: 0,
            security_keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.contains(':') {
                c.assignments += 1;
            }
        }
        for k in CLUSTER_KEYS {
            c.cluster_keys += t.matches(k).count();
        }
        for k in NODE_KEYS {
            c.node_keys += t.matches(k).count();
        }
        for k in NETWORK_KEYS {
            c.network_keys += t.matches(k).count();
        }
        for k in DISCOVERY_KEYS {
            c.discovery_keys += t.matches(k).count();
        }
        for k in PATH_KEYS {
            c.path_keys += t.matches(k).count();
        }
        for k in SECURITY_KEYS {
            c.security_keys += t.matches(k).count();
        }
        let mut misc = 0;
        for k in MISC_KEYS {
            misc += t.matches(k).count();
        }
        c.keys = c.cluster_keys
            + c.node_keys
            + c.network_keys
            + c.discovery_keys
            + c.path_keys
            + c.security_keys
            + misc;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"cluster.name: demo\nnode.name: node-1\nnode.roles: [master, data]\nnetwork.host: 0.0.0.0\nhttp.port: 9200\ntransport.port: 9300\ndiscovery.seed_hosts: [127.0.0.1]\ncluster.initial_master_nodes: [node-1]\npath.data: /var/lib/opensearch\npath.logs: /var/log/opensearch\nplugins.security.ssl.http.enabled: true\naction.auto_create_index: true\n";
        assert!(detect(b));
        let c = Opensearch::parse(b).unwrap();
        assert_eq!(c.cluster_keys, 2);
        assert_eq!(c.node_keys, 2);
        assert_eq!(c.network_keys, 3);
        assert_eq!(c.discovery_keys, 1);
        assert_eq!(c.path_keys, 2);
        assert_eq!(c.security_keys, 1);
        assert_eq!(c.assignments, 12);
        assert_eq!(c.keys, 12);
    }

    #[test]
    fn rejects_yaml() {
        assert!(!detect(b"foo: bar\nbaz: qux\n"));
        assert!(Opensearch::parse(b"").is_none());
    }
}

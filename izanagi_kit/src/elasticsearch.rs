//! Elasticsearch `elasticsearch.yml` config census.
//!
//! Elasticsearch top-level keys: `cluster.name`, `node.name`,
//! `node.roles`, `node.attr`, `path.data`, `path.logs`, `path.repo`,
//! `bootstrap.memory_lock`, `network.host`, `network.publish_host`,
//! `discovery.seed_hosts`, `discovery.seed_providers`,
//! `cluster.initial_master_nodes`, `transport.host`/`transport.port`,
//! `xpack.*`, `action.destructive_requires_name`, `thread_pool`,
//! `processors`, `ingest.*`, `search.*`, `indices.*`, `gateway.*`,
//! `cluster.routing.*`, `monitoring.*`, `reindex.*`, `script.*`.
//!
//! ```rust
//! let k = b"cluster.name: es-cluster\nnode.name: node-1\npath.data: /var/lib/elasticsearch\npath.logs: /var/log/elasticsearch\nnetwork.host: 0.0.0.0\ndiscovery.seed_hosts: [h1, h2]\ncluster.initial_master_nodes: [node-1]\n";
//! assert!(izanagi_kit::elasticsearch::detect(k));
//! ```

/// Elasticsearch config census.
#[derive(Debug, Clone)]
pub struct Elasticsearch {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "cluster.name",
    "cluster.initial_master_nodes",
    "discovery.seed_hosts",
    "discovery.seed_providers",
    "node.name",
    "node.roles",
    "node.attr",
    "path.data",
    "path.logs",
    "path.repo",
    "bootstrap.memory_lock",
    "network.host",
    "network.publish_host",
    "action.destructive_requires_name",
    "ingest.geoip.downloader.enabled",
    "indices.recovery.max_bytes_per_sec",
    "transport.host",
    "transport.port",
    "cluster.routing",
    "xpack",
];

const WEAK: &[&str] = &[
    "http.port",
    "http.host",
    "cluster",
    "node",
    "path",
    "network",
    "discovery",
    "thread_pool",
    "processors",
    "reindex",
    "script",
    "monitoring",
    "search",
    "gateway",
    "saml",
    "oidc",
    "ingest",
    "slm",
    "ilm",
    "logger",
    "plugins",
    "server",
    "pidfile",
    "ccr",
    "watcher",
    "vector",
    "inference",
    "profiling",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"').trim_matches('\'');
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect an `elasticsearch.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // vendor-exclusive keys carry the weight; shared keys only count
    // once a strong anchor is present.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Elasticsearch {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"cluster.name: es-cluster\nnode.name: node-1\npath.data: /var/lib/elasticsearch\npath.logs: /var/log/elasticsearch\nnetwork.host: 0.0.0.0\ndiscovery.seed_hosts: [h1, h2]\ncluster.initial_master_nodes: [node-1]\n";
        assert!(detect(b));
        let c = Elasticsearch::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"cluster:\n  name: x\npath:\n  data: /y\n"));
        assert!(!detect(
            b"# cluster.name: x\n# node.name: y\npath:\n  x: 1\n"
        ));
    }
}

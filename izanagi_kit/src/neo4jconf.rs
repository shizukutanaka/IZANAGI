//! `neo4j.conf` census.
//!
//! `key=value` lines with dot-namespaced keys (`dbms.*` in Neo4j 3.x/4.x,
//! `server.*`/`db.*`/`initial.*` in 5.x, `causal_clustering.*`, `fabric.*`,
//! `gds.*`). `#` comments.
//!
//! ```rust
//! let n = b"dbms.default_listen_address=0.0.0.0\nserver.bolt.enabled=true\nserver.http.listen_address=:7474\ndbms.security.auth_enabled=true\n";
//! assert!(izanagi_kit::neo4jconf::detect(n));
//! let c = izanagi_kit::neo4jconf::Neo4jconf::parse(n).unwrap();
//! assert_eq!(c.settings, 4);
//! ```

/// neo4j.conf census.
#[derive(Debug, Clone)]
pub struct Neo4jconf {
    /// `key=value` lines with a known `dbms.*`/`server.*`/`db.*` prefix.
    pub settings: usize,
    /// Other `key=value` lines.
    pub other: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Key prefixes covering both the `dbms.*` (3.x/4.x) and the `server.*`,
/// `db.*`, `initial.*` (5.x) namespaces.
const PREFIXES: &[&str] = &[
    "apoc.",
    "causal_clustering.",
    "db.",
    "dbms.",
    "dbms_memory_",
    "fabric.",
    "gds.",
    "ha.",
    "initial.",
    "metrics.",
    "server.",
];

fn assign_key(l: &str) -> Option<&str> {
    let t = l.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

fn is_neo4j_key(k: &str) -> bool {
    PREFIXES.iter().any(|p| k.starts_with(p))
}

/// Detect a `neo4j.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    for l in t.lines() {
        if let Some(k) = assign_key(l) {
            if is_neo4j_key(k) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Neo4jconf {
    /// Count keys. Returns `None` when the input does not look like a
    /// `neo4j.conf`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            other: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = assign_key(l) {
                if is_neo4j_key(k) {
                    c.settings += 1;
                } else {
                    c.other += 1;
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
        let b = b"# neo4j\ndbms.default_listen_address=0.0.0.0\ndbms.default_advertised_address=db1\ndbms.connector.bolt.enabled=true\ndbms.connector.http.enabled=true\ndbms.memory.heap.initial_size=512m\ndbms.memory.pagecache.size=1g\ndbms.security.auth_enabled=true\nserver.bolt.listen_address=:7687\nserver.http.listen_address=:7474\nserver.directories.data=/var/lib/neo4j/data\ndb.logs.query.enabled=INFO\ndb.checkpoint.interval.time=900s\ninitial.server.mode_constraint=PRIMARY\n";
        assert!(detect(b));
        let c = Neo4jconf::parse(b).unwrap();
        assert_eq!(c.settings, 13);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"port=6379\nbind=127\n"));
        assert!(!detect(b"foo=1\nbar=2\nbaz=3\n"));
        assert!(Neo4jconf::parse(b"").is_none());
    }
}

//! Census of a Feast `feature_store.yaml` file.
//!
//! Top-level keys: `project`, `provider`, `registry`/`registry_store_type`,
//! `online_store`/`offline_store`, `batch_engine`, `feature_server`,
//! `entity_key_serialization_version`, `flags`, `service`,
//! `auth`/`authz`, `materialization_engine`, `coalesce_batches`,
//! `all_together`/`concurrent_materialization` etc. Also legacy
//! `enable_features`/`featureview`/`feature_service` definitions.
//! Counts top keys, store blocks and comments.
//!
//! ```rust
//! let c = izanagi_kit::feast::Feast::parse(
//!     b"project: demo\nregistry: data/registry.db\nprovider: local\n\
//!       online_store:\n  type: sqlite\n",
//! ).unwrap();
//! assert_eq!(c.keys, 4);
//! ```
#![forbid(unsafe_code)]

/// feast feature_store.yaml census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feast {
    /// Recognised top-level keys.
    pub keys: usize,
    /// `online_store:`/`offline_store:`/`batch_engine:`/`registry:`/`feature_server:` nested blocks.
    pub store_blocks: usize,
    /// All other `key:` lines.
    pub entries: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Top-level feature_store keys.
const KEYS: &[&str] = &[
    "project",
    "provider",
    "registry",
    "registry_store_type",
    "online_store",
    "offline_store",
    "batch_engine",
    "feature_server",
    "entity_key_serialization_version",
    "flags",
    "service",
    "auth",
    "authz",
    "materialization_engine",
    "coalesce_batches",
    "all_together",
    "concurrent_materialization",
    "enable_features",
    "featureview",
    "feature_service",
    "project_location",
];

/// Nested store-section keys.
const STORES: &[&str] = &[
    "online_store",
    "offline_store",
    "batch_engine",
    "registry",
    "feature_server",
    "authz",
];

/// True if `b` looks like feature_store.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("online_store:")
        || t.contains("offline_store:")
        || t.contains("entity_key_serialization_version"))
        || (t.contains("provider:") && t.contains("registry:"))
        || t.contains("feature_server:")
}

impl Feast {
    /// Parse a feature_store.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            store_blocks: 0,
            entries: 0,
            comments: 0,
        };
        let mut in_store = false;
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if indent == 0 {
                let key = l.split(':').next().unwrap_or("").trim();
                if STORES.contains(&key) && l.ends_with(':') {
                    c.store_blocks += 1;
                    c.keys += 1;
                    in_store = true;
                } else if KEYS.contains(&key) {
                    c.keys += 1;
                    in_store = false;
                } else {
                    in_store = false;
                }
            } else if in_store || l.contains(':') {
                c.entries += 1;
            }
        }
        if c.keys == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# feast\n",
            "project: demo\n",
            "provider: local\n",
            "registry: data/registry.db\n",
            "online_store:\n",
            "  type: sqlite\n",
            "  path: data/online.db\n",
            "offline_store:\n",
            "  type: file\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Feast::parse(b.as_bytes()).unwrap();
        assert_eq!(c.keys, 5);
        assert_eq!(c.store_blocks, 2);
        assert_eq!(c.entries, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(Feast::parse(b"# none\n").is_none());
    }
}

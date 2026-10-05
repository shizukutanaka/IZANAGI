//! Milvus `milvus.yaml` parser.
//!
//! Detects the Milvus vector-DB config by its top-level sections
//! (`etcd:`/`minio:`/`pulsar:`/`rocksmq:`/`rootCoord:`/`proxy:`/
//! `queryCoord:`/`queryNode:`/`indexCoord:`/`indexNode:`/`dataCoord:`/
//! `dataNode:`/`localStorage:`/`log:`/`common:`/`grpc:`/`metrics:`), and
//! counts structure.
//!
//! ```
//! let b = b"etcd:\n  endpoints: [localhost:2379]\nminio:\n  address: localhost\nproxy:\n  port: 19530\nlog:\n  level: info\n";
//! assert!(izanagi_kit::milvusconf::detect(b));
//! let c = izanagi_kit::milvusconf::Milvus::parse(b).unwrap();
//! assert_eq!(c.sections, 4);
//! ```

/// Parsed milvus.yaml summary.
#[derive(Debug, Clone)]
pub struct Milvus {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Top-level known sections.
    pub sections: usize,
    /// Indented `key:` option lines.
    pub options: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Known top-level sections.
const SECTION_KEYS: &[&str] = &[
    "etcd:",
    "minio:",
    "pulsar:",
    "rocksmq:",
    "natsmq:",
    "kafka:",
    "rootCoord:",
    "proxy:",
    "queryCoord:",
    "queryNode:",
    "indexCoord:",
    "indexNode:",
    "dataCoord:",
    "dataNode:",
    "localStorage:",
    "log:",
    "msgChannel:",
    "common:",
    "quotaAndLimits:",
    "grpc:",
    "metrics:",
    "security:",
    "backup:",
    "metastore:",
    "mq:",
    "knowhere:",
];

/// Nested keys that strengthen detection.
const OPTION_KEYS: &[&str] = &[
    "endpoints:",
    "address:",
    "rootPath:",
    "bucketName:",
    "tlsEnabled:",
    "tlsMode:",
    "channel:",
    "subNamePrefix:",
    "defaultPartitionNum:",
    "maxTimeTickDelay:",
    "port:",
    "internalPort:",
    "trace:",
    "exporter:",
    "enableProtectedAuth:",
    "superUsers:",
    "rbac:",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Milvus config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = SECTION_KEYS
        .iter()
        .chain(OPTION_KEYS.iter())
        .filter(|k| key_present(t, k))
        .count();
    hits >= 2
}

impl Milvus {
    /// Count categories. Returns `None` when the input does not look like
    /// a milvus.yaml.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            options: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim_end();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if !l.starts_with(' ') && tr.ends_with(':') && !tr.starts_with('-') {
                c.sections += 1;
            } else if tr.contains(": ") || tr.ends_with(':') {
                c.options += 1;
            }
        }
        for k in SECTION_KEYS {
            c.keys += t.matches(k).count();
        }
        for k in OPTION_KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"etcd:\n  endpoints: [localhost:2379]\n  rootPath: by-dev\nminio:\n  address: localhost\n  port: 9000\n  bucketName: milvus-bucket\npulsar:\n  address: pulsar://localhost:6650\nproxy:\n  port: 19530\nlog:\n  level: info\n";
        assert!(detect(b));
        let c = Milvus::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.options, 8);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_yaml() {
        assert!(!detect(b"foo: bar\nbaz: qux\n"));
        assert!(Milvus::parse(b"").is_none());
    }
}

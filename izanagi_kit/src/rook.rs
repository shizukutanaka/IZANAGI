//! Rook Ceph manifest census (`ceph.rook.io` API group).
//!
//! `apiVersion` in the `ceph.rook.io` API group(s) together with a
//! `kind` in `CephCluster, CephBlockPool, CephFilesystem, CephObjectStore, CephObjectStoreUser, CephObjectRealm, CephObjectZone, CephObjectZoneGroup, CephNFS, CephClient, CephRBDMirror, CephFilesystemMirror, CephFilesystemSubVolumeGroup, CephBlockPoolRadosNamespace, CephCOSIDriver, CephReplicationSource, CephBucketTopic, CephBucketNotification, CephMirroredPeer`.
//!
//! ```rust
//! let k = b"apiVersion: ceph.rook.io/v1\nkind: CephCluster\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
//! assert!(izanagi_kit::rook::detect(k));
//! ```

use crate::textutil::{kind_val, value_of};
/// Rook Ceph manifest census.
#[derive(Debug, Clone)]
pub struct Rook {
    /// `kind` value, when present.
    pub kind: Option<String>,
    /// `apiVersion`/`kind`/`metadata`/`spec` envelope lines.
    pub envelope: usize,
    /// `- ` list items.
    pub items: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const GROUPS: &[&str] = &["ceph.rook.io"];

const KINDS: &[&str] = &[
    "CephCluster",
    "CephBlockPool",
    "CephFilesystem",
    "CephObjectStore",
    "CephObjectStoreUser",
    "CephObjectRealm",
    "CephObjectZone",
    "CephObjectZoneGroup",
    "CephNFS",
    "CephClient",
    "CephRBDMirror",
    "CephFilesystemMirror",
    "CephFilesystemSubVolumeGroup",
    "CephBlockPoolRadosNamespace",
    "CephCOSIDriver",
    "CephReplicationSource",
    "CephBucketTopic",
    "CephBucketNotification",
    "CephMirroredPeer",
];

fn api_ok(t: &str) -> bool {
    t.lines().any(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("apiVersion") {
            return false;
        }
        let v = value_of(s);
        GROUPS.iter().any(|g| v.starts_with(g))
    })
}

/// Detect a Rook Ceph manifest.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    if !api_ok(t) {
        return false;
    }
    match kind_val(t) {
        Some(k) => KINDS.contains(&k.as_str()),
        None => false,
    }
}

impl Rook {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kind: kind_val(t),
            envelope: 0,
            items: 0,
            settings: 0,
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
                let k = s[..s.find(':').unwrap_or(s.len())].trim();
                if matches!(k, "apiVersion" | "kind" | "metadata" | "spec") {
                    c.envelope += 1;
                }
                if !k.is_empty() {
                    c.settings += 1;
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
        let b = b"apiVersion: ceph.rook.io/v1\nkind: CephCluster\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
        assert!(detect(b));
        let c = Rook::parse(b).unwrap();
        assert_eq!(c.kind.as_deref(), Some("CephCluster"));
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"apiVersion: v1\nkind: CephCluster\n"));
        assert!(!detect(b"apiVersion: ceph.rook.io/v1\nkind: Deployment\n"));
        assert!(!detect(
            b"# apiVersion: ceph.rook.io/v1\nkind: CephCluster\n"
        ));
    }
}

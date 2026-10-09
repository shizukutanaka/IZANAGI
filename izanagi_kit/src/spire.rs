//! SPIRE/SPIFFE manifest census (`spire.spiffe.io`/`spiffeid.spiffe.io` groups).
//!
//! `apiVersion` in the `spire.spiffe.io, spiffeid.spiffe.io` API group(s) together with a
//! `kind` in `ClusterSPIFFEID, ClusterFederatedTrustDomain, SPIFFECSIDriver, SpiffeID, ClusterStaticEntry, ControllermanagerConfig, UpstreamAuthorityDisk, UpstreamAuthorityAWS, UpstreamAuthorityAwsPCA, UpstreamAuthorityAwsSecret, UpstreamAuthorityCertManager, UpstreamAuthorityGCPIIT, UpstreamAuthorityVault, UpstreamAuthorityspire, UpstreamAuthorityKeyManager`.
//!
//! ```rust
//! let k = b"apiVersion: spire.spiffe.io/v1alpha1\nkind: ClusterSPIFFEID\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
//! assert!(izanagi_kit::spire::detect(k));
//! ```

use crate::textutil::{kind_val, value_of};
/// SPIRE manifest census.
#[derive(Debug, Clone)]
pub struct Spire {
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

const GROUPS: &[&str] = &["spire.spiffe.io", "spiffeid.spiffe.io"];

const KINDS: &[&str] = &[
    "ClusterSPIFFEID",
    "ClusterFederatedTrustDomain",
    "SPIFFECSIDriver",
    "SpiffeID",
    "ClusterStaticEntry",
    "ControllermanagerConfig",
    "UpstreamAuthorityDisk",
    "UpstreamAuthorityAWS",
    "UpstreamAuthorityAwsPCA",
    "UpstreamAuthorityAwsSecret",
    "UpstreamAuthorityCertManager",
    "UpstreamAuthorityGCPIIT",
    "UpstreamAuthorityVault",
    "UpstreamAuthorityspire",
    "UpstreamAuthorityKeyManager",
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

/// Detect a SPIRE manifest.
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

impl Spire {
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
        let b = b"apiVersion: spire.spiffe.io/v1alpha1\nkind: ClusterSPIFFEID\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
        assert!(detect(b));
        let c = Spire::parse(b).unwrap();
        assert_eq!(c.kind.as_deref(), Some("ClusterSPIFFEID"));
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"apiVersion: v1\nkind: ClusterSPIFFEID\n"));
        assert!(!detect(
            b"apiVersion: spire.spiffe.io/v1alpha1\nkind: Deployment\n"
        ));
        assert!(!detect(
            b"# apiVersion: spire.spiffe.io/v1alpha1\nkind: ClusterSPIFFEID\n"
        ));
    }
}

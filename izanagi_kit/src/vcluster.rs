//! vCluster `vcluster.yaml` / `values.yaml` config census.
//!
//! vCluster top-level keys: `vcluster`, `controlPlane`, `sync`,
//! `syncer` (legacy), `storage`, `rbac`, `isolation`, `telemetry`,
//! `exportKubeConfig`, `proxy`, `plugin`, `coredns`, `openshift`,
//! `init`, `policies`, `experimental`, `distro`, `embedded`,
//! `networking`, `ingress`, `serviceAccount`, `fallbackHostDns`.
//!
//! ```rust
//! let k = b"vcluster:\n  image: rancher/k3s\ncontrolPlane:\n  distro:\n    k3s:\n      enabled: true\nsync:\n  services:\n    enabled: true\n";
//! assert!(izanagi_kit::vcluster::detect(k));
//! ```

/// vCluster config census.
#[derive(Debug, Clone)]
pub struct Vcluster {
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
    "vcluster",
    "controlPlane",
    "syncer",
    "telemetry",
    "exportKubeConfig",
    "isolation",
    "openshift",
    "fallbackHostDns",
    "embedded",
    "distro",
];

const WEAK: &[&str] = &[
    "sync",
    "storage",
    "rbac",
    "proxy",
    "plugin",
    "coredns",
    "init",
    "policies",
    "experimental",
    "networking",
    "ingress",
    "serviceAccount",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect a vCluster config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
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

impl Vcluster {
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
        let b = b"vcluster:\n  image: rancher/k3s\ncontrolPlane:\n  distro:\n    k3s:\n      enabled: true\nsync:\n  services:\n    enabled: true\n";
        assert!(detect(b));
        let c = Vcluster::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"sync:\n  services:\n    enabled: true\n"));
        assert!(!detect(b"# vcluster:\ncontrolPlane:\n"));
    }
}

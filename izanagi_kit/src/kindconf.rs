//! kind `kind-config.yaml` cluster config census.
//!
//! `kind: Cluster` + `apiVersion: kind.x-k8s.io/v1alpha4`
//! envelope; `nodes:` entries (`- role: control-plane|worker`
//! with `image:`/`kubeadmConfigPatches`/`extraMounts`/
//! `extraPortMappings`/`labels:`/`registration`), `networking:`
//! (`apiServerAddress`/`apiServerPort`/`podSubnet`/
//! `serviceSubnet`/`disableDefaultCNI`/`kubeProxyMode`/
//! `ipFamily`/`listenAddress`/`kubeProxyVerbosity`),
//! `kubeadmConfigPatches`/`kubeadmConfigPatchesJSON6902`/
//! `containerdConfigPatches`, `featureGates`, `runtimeConfig`,
//! `name`, `nodes`, `containerdConfigPatches`,
//! `dnsOptions`, `labels`.
//!
//! ```rust
//! let k = "kind: Cluster\napiVersion: kind.x-k8s.io/v1alpha4\nnodes:\n- role: control-plane\n- role: worker\n- role: worker\n";
//! let c = izanagi_kit::kindconf::Kindconf::parse(k.as_bytes()).unwrap();
//! assert_eq!(c.nodes, 3);
//! ```

/// kind config census.
#[derive(Debug, Clone)]
pub struct Kindconf {
    /// `- role:` node entries.
    pub nodes: usize,
    /// Recognised top-level/section keys.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items (all).
    pub items: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "kind",
    "apiVersion",
    "name",
    "nodes",
    "networking",
    "kubeadmConfigPatches",
    "kubeadmConfigPatchesJSON6902",
    "containerdConfigPatches",
    "featureGates",
    "runtimeConfig",
    "dnsOptions",
    "labels",
    "apiServerAddress",
    "apiServerPort",
    "podSubnet",
    "serviceSubnet",
    "disableDefaultCNI",
    "kubeProxyMode",
    "ipFamily",
    "listenAddress",
    "kubeProxyVerbosity",
    "role",
    "image",
    "extraMounts",
    "extraPortMappings",
    "kubeadmConfigPatches",
    "registration",
    "hostPath",
    "containerPath",
    "readOnly",
    "selinuxRelabel",
    "propagation",
    "containerPort",
    "hostPort",
    "protocol",
    "version",
    "control-plane",
    "worker",
    "key",
    "value",
    "effect",
    "taints",
];

/// Detect kind config content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    t.contains("kind.x-k8s.io") || (t.contains("kind: Cluster") && t.contains("- role:"))
}

impl Kindconf {
    /// Census a kind config buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            nodes: 0,
            sections: 0,
            settings: 0,
            items: 0,
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
            if let Some(rest) = s.strip_prefix("- ") {
                c.items += 1;
                if rest.trim().starts_with("role:") {
                    c.nodes += 1;
                }
                continue;
            }
            if let Some(colon) = s.find(':') {
                let key = s[..colon].trim();
                if !key.is_empty() {
                    c.settings += 1;
                    if KEYS.contains(&key) {
                        c.sections += 1;
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
    fn detects_kind() {
        let b = b"kind: Cluster\napiVersion: kind.x-k8s.io/v1alpha4\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_kind() {
        let b = concat!(
            "# kind\n",
            "kind: Cluster\n",
            "apiVersion: kind.x-k8s.io/v1alpha4\n",
            "name: dev\n",
            "networking:\n",
            "  apiServerAddress: 127.0.0.1\n",
            "  apiServerPort: 6443\n",
            "  podSubnet: 10.244.0.0/16\n",
            "  serviceSubnet: 10.96.0.0/12\n",
            "  disableDefaultCNI: false\n",
            "  kubeProxyMode: iptables\n",
            "  ipFamily: ipv4\n",
            "nodes:\n",
            "- role: control-plane\n",
            "  image: kindest/node:v1.30.0\n",
            "  extraPortMappings:\n",
            "  - containerPort: 80\n",
            "    hostPort: 8080\n",
            "  extraMounts:\n",
            "  - hostPath: /data\n",
            "    containerPath: /data\n",
            "  kubeadmConfigPatches:\n",
            "  - |\n",
            "    kind: ClusterConfiguration\n",
            "- role: worker\n",
            "  image: kindest/node:v1.30.0\n",
            "- role: worker\n",
            "featureGates:\n",
            "  EphemeralContainers: true\n",
            "runtimeConfig:\n",
            "  api/alpha: \"true\"\n",
        );
        let c = Kindconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.nodes, 3);
        assert!(c.sections >= 18);
        assert!(c.settings >= 20);
        assert!(c.items >= 5);
        assert_eq!(c.comments, 1);
    }
}

//! k3d cluster config (`cluster.yaml`) census.
//!
//! `apiVersion: k3d.io/v1alpha4|v1alpha5` + `kind: Simple`/
//! `Cluster` envelope; top keys: `servers`, `agents`,
//! `image`, `ports`, `volumes`, `env`, `registries`,
//! `options`, `kubeAPI`, `network`, `subnet`, `token`,
//! `images`, `k3s`, `hostAliases`, `runtimeLabels`,
//! `nodeFilters`, `k3sargs`, `noLoadbalancer`,
//! `exposeAPI`, `host`, `hostPort`, `hostIP`, `hostNetwork`,
//! `loadbalancer`, `configOverrides`, `globalEnv`,
//! `registries`/`create`/`use`, `mirror`.
//!
//! ```rust
//! let k = "apiVersion: k3d.io/v1alpha5\nkind: Simple\nservers: 1\nagents: 2\nimage: rancher/k3s:latest\n";
//! let c = izanagi_kit::k3d::K3d::parse(k.as_bytes()).unwrap();
//! assert_eq!(c.sections, 5);
//! ```

/// k3d config census.
#[derive(Debug, Clone)]
pub struct K3d {
    /// Recognised top-level/section keys.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "apiVersion",
    "kind",
    "metadata",
    "name",
    "servers",
    "agents",
    "image",
    "ports",
    "volumes",
    "env",
    "registries",
    "options",
    "kubeAPI",
    "network",
    "subnet",
    "token",
    "images",
    "k3s",
    "hostAliases",
    "runtimeLabels",
    "nodeFilters",
    "k3sargs",
    "noLoadbalancer",
    "exposeAPI",
    "host",
    "hostPort",
    "hostIP",
    "hostNetwork",
    "loadbalancer",
    "configOverrides",
    "globalEnv",
    "create",
    "use",
    "mirror",
    "endpoint",
    "aliases",
    "volumesMatch",
    "disable",
    "datastore",
    "servicelb",
    "traefik",
    "metrics",
    "runtime",
    "k3sVersion",
    "containerRuntime",
    "labels",
    "args",
    "extraArgs",
    "extraMounts",
    "ulimits",
    "gpus",
    "dns",
    "dnsServers",
    "searchDomains",
    "wireguard",
    "encryption",
    "snapshot",
];

/// Detect k3d config content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    t.contains("k3d.io/") || (t.contains("kind: Simple") && t.contains("servers:"))
}

impl K3d {
    /// Census a k3d config buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
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
            if s.starts_with("- ") {
                c.items += 1;
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
    fn detects_k3d() {
        let b = b"apiVersion: k3d.io/v1alpha5\nkind: Simple\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_k3d() {
        let b = concat!(
            "# k3d\n",
            "apiVersion: k3d.io/v1alpha5\n",
            "kind: Simple\n",
            "metadata:\n",
            "  name: dev\n",
            "servers: 1\n",
            "agents: 2\n",
            "image: rancher/k3s:latest\n",
            "kubeAPI:\n",
            "  host: \"myhost.my.domain\"\n",
            "  hostIP: \"127.0.0.1\"\n",
            "  hostPort: \"6445\"\n",
            "ports:\n",
            "- port: 8080:80\n",
            "  nodeFilters:\n",
            "  - loadbalancer\n",
            "- port: 8443:443\n",
            "  nodeFilters:\n",
            "  - loadbalancer\n",
            "env:\n",
            "- envVar: FOO=bar\n",
            "  nodeFilters:\n",
            "  - server:*\n",
            "volumes:\n",
            "- volume: /data:/data\n",
            "  nodeFilters:\n",
            "  - all\n",
            "registries:\n",
            "  create:\n",
            "    name: my-reg\n",
            "    host: \"0.0.0.0\"\n",
            "    hostPort: \"5000\"\n",
            "  use:\n",
            "  - k3d-my-reg:5000\n",
            "options:\n",
            "  k3d:\n",
            "    wait: true\n",
            "    timeout: \"60s\"\n",
            "    disableLoadbalancer: false\n",
            "  k3s:\n",
            "    extraArgs:\n",
            "    - arg: --tls-san=127.0.0.1\n",
            "      nodeFilters:\n",
            "      - server:*\n",
            "  kubeconfig:\n",
            "    updateDefaultKubeconfig: true\n",
            "    switchCurrentContext: true\n",
        );
        let c = K3d::parse(b.as_bytes()).unwrap();
        assert!(c.sections >= 20);
        assert!(c.settings >= 25);
        assert!(c.items >= 10);
        assert_eq!(c.comments, 1);
    }
}

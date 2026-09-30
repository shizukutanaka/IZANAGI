//! containerd `config.toml` census.
//!
//! TOML with root keys `version`/`root`/`state`/`oom_score`/
//! `subreaper`/`imports`/`disabled_plugins`/`required_plugins`/
//! `plugin_dirs`/`metrics`/`timeouts`/`debug`/`ttrpc`/`grpc`,
//! and `[plugins]` subtrees like
//! `[plugins."io.containerd.grpc.v1.cri"]`,
//! `[plugins."io.containerd.grpc.v1.cri".containerd]`,
//! `[plugins."io.containerd.grpc.v1.cri".containerd.runtimes]`,
//! `[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.runc]`,
//! `[plugins."io.containerd.grpc.v1.cri".registry.mirrors]`,
//! `[plugins."io.containerd.grpc.v1.cri".registry.configs."x".tls]`,
//! `[plugins."io.containerd.grpc.v1.cri".x509_key_pair_stream]`,
//! `[plugins."io.containerd.internal.v1.opt"]`,
//! `[plugins."io.containerd.service.v1.tasks-service"]`,
//! `[plugins."io.containerd.metadata.v1.bolt"]`,
//! `[plugins."io.containerd.gc.v1.scheduler"]`,
//! `[plugins."io.containerd.monitor.v1.cgroups"]`,
//! `[plugins."io.containerd.differ.v1.walking"]`,
//! `[plugins."io.containerd.snapshotter.v1.*"]`,
//! `[plugins."io.containerd.transfer.v1.local"]`,
//! `[plugins."io.containerd.lease.v1.manager"]`,
//! `[plugins."io.containerd.nri.v1.nri"]`,
//! `[plugins."io.containerd.tracing.processor.v1.otlp"]`,
//! `[plugins."io.containerd.tracing.v1"]`,
//! `[metrics]`/`[grpc]`/`[cni]`/`[debug]`/`[timeouts]`/
//! `[proxy_plugins.*]`.
//!
//! ```rust
//! let c = "[plugins.\"io.containerd.grpc.v1.cri\"]\n  sandbox_image = \"registry.k8s.io/pause:3.9\"\n  [plugins.\"io.containerd.grpc.v1.cri\".containerd]\n    default_runtime_name = \"runc\"\n";
//! let r = izanagi_kit::containerd::Containerd::parse(c.as_bytes()).unwrap();
//! assert_eq!(r.sections, 2);
//! ```

/// containerd config census.
#[derive(Debug, Clone)]
pub struct Containerd {
    /// `[…]`/`[[…]]` table headers.
    pub sections: usize,
    /// `[plugins.*]` subtrees.
    pub plugin_sections: usize,
    /// `key = value` pairs.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Detect containerd config content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    t.contains("io.containerd.")
        || (t.contains("[plugins]")
            && (t.contains("sandbox_image") || t.contains("default_runtime_name")))
}

impl Containerd {
    /// Census a containerd config buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            plugin_sections: 0,
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
            if s.starts_with('[') {
                c.sections += 1;
                if s.contains("plugins") && s.contains("io.containerd") {
                    c.plugin_sections += 1;
                }
                continue;
            }
            if let Some(eq) = s.find('=') {
                let key = s[..eq].trim();
                if !key.is_empty() {
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
    fn detects_conf() {
        let b = b"[plugins.\"io.containerd.grpc.v1.cri\"]\nsandbox_image = \"x\"\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_conf() {
        let b = concat!(
            "# containerd\n",
            "version = 2\n",
            "root = \"/var/lib/containerd\"\n",
            "state = \"/run/containerd\"\n",
            "oom_score = 0\n",
            "imports = [\"/etc/containerd/conf.d/*.toml\"]\n",
            "disabled_plugins = []\n",
            "[grpc]\n",
            "  address = \"/run/containerd/containerd.sock\"\n",
            "  tcp_address = \"\"\n",
            "  uid = 0\n",
            "  gid = 0\n",
            "  max_recv_message_size = 16777216\n",
            "[plugins]\n",
            "  [plugins.\"io.containerd.internal.v1.opt\"]\n",
            "    path = \"/opt/containerd\"\n",
            "  [plugins.\"io.containerd.grpc.v1.cri\"]\n",
            "    sandbox_image = \"registry.k8s.io/pause:3.9\"\n",
            "    enable_selinux = false\n",
            "    tolerate_missing_hugepages = true\n",
            "    [plugins.\"io.containerd.grpc.v1.cri\".containerd]\n",
            "      default_runtime_name = \"runc\"\n",
            "      snapshotter = \"overlayfs\"\n",
            "      discard_unpacked_layers = true\n",
            "      [plugins.\"io.containerd.grpc.v1.cri\".containerd.runtimes]\n",
            "        [plugins.\"io.containerd.grpc.v1.cri\".containerd.runtimes.runc]\n",
            "          runtime_type = \"io.containerd.runc.v2\"\n",
            "          [plugins.\"io.containerd.grpc.v1.cri\".containerd.runtimes.runc.options]\n",
            "            SystemdCgroup = true\n",
            "            BinaryName = \"runc\"\n",
            "    [plugins.\"io.containerd.grpc.v1.cri\".cni]\n",
            "      bin_dir = \"/opt/cni/bin\"\n",
            "      conf_dir = \"/etc/cni/net.d\"\n",
            "      max_conf_num = 1\n",
            "    [plugins.\"io.containerd.grpc.v1.cri\".registry]\n",
            "      config_path = \"\"\n",
            "      [plugins.\"io.containerd.grpc.v1.cri\".registry.mirrors]\n",
            "        [plugins.\"io.containerd.grpc.v1.cri\".registry.mirrors.\"docker.io\"]\n",
            "          endpoint = [\"https://registry-1.docker.io\"]\n",
            "[metrics]\n",
            "  address = \"127.0.0.1:1338\"\n",
            "  grpc_histogram = false\n",
            "[debug]\n",
            "  level = \"info\"\n",
            "  format = \"json\"\n",
        );
        let c = Containerd::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 14);
        assert_eq!(c.plugin_sections, 10);
        assert!(c.settings >= 20);
        assert_eq!(c.comments, 1);
    }
}

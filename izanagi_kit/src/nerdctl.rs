//! nerdctl `nerdctl.toml` の検出と構造カウント。
//!
//! `address`/`namespace`/`snapshotter`/`cgroup_manager`/`cni_path`/
//! `cni_netconfpath`/`bridge`/`debug`/`debug_full`/`hosts`/`host_gateway_ip`/
//! `systemd`/`insecure_registry`/`experimental`/`pull_unpack_speed` 等のキーを
//! TOML `key = value` 形で識別する。
//!
//! ```
//! let c = izanagi_kit::nerdctl::parse(
//!     b"debug = false\naddress = \"unix:///run/containerd/containerd.sock\"\nnamespace = \"k8s.io\"\ncgroup_manager = \"systemd\"\n").unwrap();
//! assert!(c.options >= 4);
//! assert!(izanagi_kit::nerdctl::detect(
//!     b"namespace = \"k8s.io\"\nsnapshotter = \"stargz\"\ncgroup_manager = \"systemd\"\n"));
//! ```

/// nerdctl.toml 既知キー。
const KEYS: &[&str] = &[
    "address",
    "bridge",
    "buildkit_host",
    "cgroup_manager",
    "cni_netconfpath",
    "cni_path",
    "data_root",
    "debug",
    "debug_full",
    "docker_compat",
    "experimental",
    "grpc_max_recv_msg_size",
    "grpc_max_send_msg_size",
    "host_gateway_ip",
    "hosts",
    "host_addrs",
    "insecure_registry",
    "ipfs_address",
    "kube_hide_duplicates",
    "namespace",
    "oci_hooks",
    "pull_unpack_speed",
    "rootless",
    "snapshotter",
    "systemd",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[table]` 行数。
    pub sections: usize,
    /// 既知キー行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn known_key(t: &str) -> bool {
    let Some(eq) = t.find('=') else { return false };
    KEYS.contains(&t[..eq].trim())
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `nerdctl.toml` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if known_key(t) {
            hits += 1;
            if hits >= 2 {
                return true;
            }
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') {
            c.sections += 1;
        } else if known_key(t) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# nerdctl.toml\ndebug = false\ndebug_full = false\naddress = \"unix:///run/containerd/containerd.sock\"\nnamespace = \"k8s.io\"\nsnapshotter = \"stargz\"\ncgroup_manager = \"systemd\"\ncni_path = \"/opt/cni/bin\"\ncni_netconfpath = \"/etc/cni/net.d\"\nhost_gateway_ip = \"192.168.5.2\"\nexperimental = true\ninsecure_registry = false\n\n[debug]\nlevel = \"info\"\n";

    #[test]
    fn nerdctl() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.options, 11);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_nerdctl() {
        assert!(!detect(b"key = 1\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

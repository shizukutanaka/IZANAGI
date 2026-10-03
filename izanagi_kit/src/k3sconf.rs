//! k3s `/etc/rancher/k3s/config.yaml` の検出と構造カウント。
//!
//! `server`/`agent`/`kubelet-arg`/`kube-apiserver-arg`/`cluster-cidr`/
//! `service-cidr`/`cluster-dns`/`node-name`/`node-ip`/`node-external-ip`/
//! `tls-san`/`data-dir`/`token`/`etcd-snapshot-*`/`flannel-backend`/
//! `disable`/`disable-network-policy`/`prefer-bundled-bin`/`secrets-encryption`
//! 等の既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::k3sconf::parse(
//!     b"server: https://k3s:6443\ntoken: secret\nnode-name: node1\ncluster-cidr: 10.42.0.0/16\ntls-san: [k3s.local]\n").unwrap();
//! assert!(c.options >= 5);
//! assert!(izanagi_kit::k3sconf::detect(
//!     b"server: https://x:6443\ntoken: s\nnode-name: n1\nflannel-backend: wireguard-native\n"));
//! ```

/// k3s 設定キー(server/agent 共通 + サブコンポーネント引数族)。
const KEYS: &[&str] = &[
    "agent-token",
    "bind-address",
    "bundle",
    "cluster-cidr",
    "cluster-dns",
    "cluster-domain",
    "data-dir",
    "default-local-storage-path",
    "disable",
    "disable-apiserver",
    "disable-cloud-controller",
    "disable-etcd",
    "disable-helm-controller",
    "disable-kube-proxy",
    "disable-network-policy",
    "disable-scheduler",
    "disable-servicelb",
    "docker",
    "egress-selector-mode",
    "embedded-registry",
    "enable-pprof",
    "etcd-arg",
    "etcd-disable-snapshots",
    "etcd-expose-metrics",
    "etcd-snapshot-compress",
    "etcd-snapshot-dir",
    "etcd-snapshot-name",
    "etcd-snapshot-retention",
    "etcd-snapshot-schedule-cron",
    "etcd-s3",
    "flannel-backend",
    "flannel-conf",
    "flannel-iface",
    "flannel-ipv6-masq",
    "kube-apiserver-arg",
    "kube-cloud-controller-manager-arg",
    "kube-controller-manager-arg",
    "kube-proxy-arg",
    "kube-scheduler-arg",
    "kubelet-arg",
    "lb-server-port",
    "node-external-ip",
    "node-ip",
    "node-label",
    "node-name",
    "node-taint",
    "prefer-bundled-bin",
    "private-registry",
    "profile",
    "protect-kernel-defaults",
    "resolv-conf",
    "secrets-encryption",
    "selinux",
    "server",
    "service-cidr",
    "snapshotter",
    "tls-san",
    "tls-san-security",
    "token",
    "token-file",
    "vpn-auth",
    "vpn-auth-file",
    "write-kubeconfig",
    "write-kubeconfig-mode",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn key_of(t: &str) -> &str {
    match t.find(':') {
        Some(i) => t[..i].trim(),
        None => "",
    }
}

fn known_key(t: &str) -> bool {
    KEYS.contains(&key_of(t))
}

/// `config.yaml` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with('-') {
            continue;
        }
        if known_key(t) {
            hits += 1;
            if hits >= 3 {
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
    let mut c = Counts {
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
        if t.starts_with('-') {
            continue;
        }
        if known_key(t) {
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

    const SAMPLE: &[u8] = b"# k3s config\nserver: https://server.k3s.local:6443\ntoken: K10a3b\ndata-dir: /var/lib/rancher/k3s\nnode-name: worker-1\nnode-ip: 192.168.0.11\nnode-external-ip: 203.0.113.11\ncluster-cidr: 10.42.0.0/16\nservice-cidr: 10.43.0.0/16\ncluster-dns: 10.43.0.10\nflannel-backend: wireguard-native\ntls-san:\n  - k3s.local\n  - api.k3s.local\ndisable:\n  - traefik\n  - servicelb\nkubelet-arg:\n  - \"max-pods=110\"\netcd-snapshot-schedule-cron: \"0 */12 * * *\"\netcd-snapshot-retention: 10\nsecrets-encryption: true\n";

    #[test]
    fn k3sconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 16);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_k3sconf() {
        assert!(!detect(b"foo: 1\nbar: 2\nbaz: 3\n"));
        assert!(parse(b"text\n").is_none());
    }
}

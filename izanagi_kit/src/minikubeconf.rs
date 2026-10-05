//! Minikube `config.json`(`~/.minikube/config`)の検出と構造カウント。
//!
//! `driver`/`cpus`/`memory`/`disk-size`/`kubernetes-version`/`container-runtime`/
//! `network`/`nodes`/`addons`/`vm-driver`/`iso-url`/`registry-mirror`/
//! `insecure-registry`/`image-repository`/`extra-config`/`embed-certs`/
//! `mount`/`mount-string`/`share` 等の既知キーを JSON `"key":` 走査で識別する。
//!
//! ```
//! let c = izanagi_kit::minikubeconf::parse(
//!     b"{\n  \"driver\": \"docker\",\n  \"cpus\": 4,\n  \"memory\": 8192,\n  \"kubernetes-version\": \"v1.28\",\n  \"container-runtime\": \"containerd\"\n}\n").unwrap();
//! assert!(c.options >= 5);
//! assert!(izanagi_kit::minikubeconf::detect(
//!     b"{\"driver\": \"docker\", \"cpus\": 2, \"memory\": 2048}\n"));
//! ```

/// Minikube 設定キー。
const KEYS: &[&str] = &[
    "addons",
    "apiserver-ips",
    "apiserver-name",
    "apiserver-names",
    "apiserver-port",
    "base-image",
    "cache",
    "cert_expiration",
    "container-runtime",
    "container-runtime-version",
    "containerd-shim-path",
    "cpus",
    "cri-socket",
    "crio",
    "delete-on-failure",
    "disk-size",
    "dns-domain",
    "docker-env",
    "docker-opt",
    "driver",
    "embed-certs",
    "enable-default-cni",
    "extra-config",
    "extra-options",
    "force",
    "fx-privileged",
    "gpus",
    "host-dns-resolver",
    "host-only-cidr",
    "host-only-nic-type",
    "hyperkit-vpnkit-sock",
    "hyperkit-vsock-ports",
    "hyperv-virtual-switch",
    "image-repository",
    "insecure-registry",
    "install-addons",
    "iso-url",
    "keep-context",
    "kubernetes-version",
    "kvm-gpu",
    "kvm-hidden",
    "kvm-network",
    "kvm-numa-count",
    "kvm-qemu-uri",
    "listen-address",
    "memory",
    "mount",
    "mount-string",
    "mount-uid",
    "nat-nic-type",
    "network",
    "network-plugin",
    "nfs-ip",
    "nfs-share",
    "nfs-shares-root",
    "nodes",
    "no-kubernetes",
    "no-vtx-check",
    "output",
    "ports",
    "preload",
    "profile",
    "registry-mirror",
    "rootless",
    "service-cluster-ip-range",
    "share",
    "ssh-ip-address",
    "ssh-key",
    "ssh-port",
    "ssh-user",
    "subnet",
    "uuid",
    "vm-driver",
    "vm-filter",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー行数。
    pub options: usize,
    /// `//` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// `"key"` の直後に(空白を挟んでも)`:` が来るか。JSON は `"key" : value`
/// のようにコロン前の空白を許す。
fn has_key(t: &str, key: &str) -> bool {
    let quoted = format!("\"{}\"", key);
    let mut rest = t;
    while let Some(i) = rest.find(&quoted) {
        let after = &rest[i + quoted.len()..];
        if after.trim_start().starts_with(':') {
            return true;
        }
        rest = &rest[i + 1..];
    }
    false
}

fn key_hits(t: &str) -> usize {
    KEYS.iter().filter(|k| has_key(t, k)).count()
}

/// `config.json` らしさを判定する。
///
/// 非コメント行を連結してから走査するので、`"key"` と `:` が
/// 改行で分かれた記法(`"key"\n:`)にも耐える。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut joined = String::with_capacity(text.len());
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        joined.push_str(t);
        joined.push('\n');
    }
    key_hits(&joined) >= 2
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
        if t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if key_hits(t) > 0 {
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

    const SAMPLE: &[u8] = b"{\n    \"addons\": {\n        \"dashboard\": true,\n        \"metrics-server\": false\n    },\n    \"container-runtime\": \"containerd\",\n    \"cpus\": 4,\n    \"disk-size\": \"50g\",\n    \"driver\": \"docker\",\n    \"embed-certs\": true,\n    \"extra-config\": \"kubelet.housekeeping-interval=10s\",\n    \"kubernetes-version\": \"v1.28.3\",\n    \"memory\": 8192,\n    \"mount\": false,\n    \"network\": \"\",\n    \"nodes\": [\n        {\"name\": \"minikube\", \"control-plane\": true}\n    ],\n    \"registry-mirror\": [\"https://mirror.local\"],\n    \"vm-driver\": \"kvm2\"\n}\n";

    #[test]
    fn minikubeconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 14);
        assert_eq!(c.misc, 7);
    }

    #[test]
    fn not_minikubeconf() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn spaced_colon_still_detects() {
        // JSON permits whitespace between the quoted key and the colon.
        assert!(detect(
            b"{\n  \"driver\" : \"docker\",\n  \"cpus\" : 2,\n  \"memory\" : 2048\n}\n"
        ));
        let c = parse(b"{\n  \"driver\"\t:\t\"docker\",\n  \"cpus\" : 2\n}\n").unwrap();
        assert_eq!(c.options, 2);
    }

    #[test]
    fn newline_before_colon_still_detects() {
        // JSON permits a line break between the quoted key and the colon.
        assert!(detect(
            b"{\n  \"driver\"\n: \"docker\",\n  \"cpus\"\n: 2\n}\n"
        ));
        // Keys mentioned only inside string values do not count.
        assert!(!detect(b"{\"note\": \"driver cpus\"}\n"));
    }
}

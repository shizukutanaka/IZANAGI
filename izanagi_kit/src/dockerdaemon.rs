//! Census of a `dockerd` `daemon.json` file.
//!
//! The Docker daemon configuration is a JSON object of ~60 known
//! keys (`"data-root"`, `"storage-driver"`, `"registry-mirrors"`,
//! `"insecure-registries"`, `"log-driver"`, `"log-opts"`, `"features"`,
//! `"experimental"`, `"bip"`, `"default-address-pools"` …).
//! Counts keys, array-valued keys, object-valued keys and comments.
//!
//! ```rust
//! let c = izanagi_kit::dockerdaemon::DockerDaemon::parse(
//!     b"{\"data-root\":\"/var/lib/docker\",\"storage-driver\":\"overlay2\",\
//!       \"insecure-registries\":[\"localhost:5000\"],\"log-driver\":\"json-file\"}",
//! ).unwrap();
//! assert_eq!(c.keys, 4);
//! assert_eq!(c.array_keys, 1);
//! ```
#![forbid(unsafe_code)]

use crate::textutil::strip_bom;
/// daemon.json census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockerDaemon {
    /// Recognised `"key":` entries.
    pub keys: usize,
    /// Keys whose value is an array.
    pub array_keys: usize,
    /// Keys whose value is an object.
    pub object_keys: usize,
    /// `//`/`#` comment lines (JSON is strict but tolerated).
    pub comments: usize,
}

/// Known daemon.json keys.
const KEYS: &[&str] = &[
    "data-root",
    "exec-root",
    "storage-driver",
    "storage-opts",
    "registry-mirrors",
    "insecure-registries",
    "log-driver",
    "log-opts",
    "default-address-pools",
    "bip",
    "fixed-cidr",
    "fixed-cidr-v6",
    "dns",
    "dns-opts",
    "dns-search",
    "default-gateway",
    "default-gateway-v6",
    "ipv6",
    "iptables",
    "ip-forward",
    "ip-masq",
    "icc",
    "bridge",
    "pidfile",
    "tls",
    "tlsverify",
    "tlscacert",
    "tlscert",
    "tlskey",
    "api-cors-header",
    "hosts",
    "group",
    "exec-opts",
    "features",
    "experimental",
    "metrics-addr",
    "swarm-default-advertise-addr",
    "node-generic-resources",
    "authorization-plugins",
    "allow-nondistributable-artifacts",
    "shutdown-timeout",
    "max-download-attempts",
    "max-concurrent-downloads",
    "max-concurrent-uploads",
    "userland-proxy",
    "live-restore",
    "no-new-privileges",
    "seccomp-profile",
    "selinux-enabled",
    "userns-remap",
    "cgroup-parent",
    "default-ulimits",
    "oom-score-adjust",
    "containerd-namespace",
    "default-runtime",
    "runtimes",
    "proxies",
    "http-proxy",
    "https-proxy",
    "no-proxy",
];

/// True if `b` looks like daemon.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.trim_start().starts_with('{') && KEYS.iter().filter(|k| t.contains(**k)).count() >= 2
}

/// Count `"key"` occurrences whose value starts with `lead`.
fn count_key(t: &str, lead: u8) -> usize {
    let mut n = 0usize;
    let mut rest = t;
    while let Some(q) = rest.find('"') {
        let Some(end) = rest[q + 1..].find('"') else {
            break;
        };
        let key = &rest[q + 1..q + 1 + end];
        if KEYS.contains(&key) {
            let after = rest[q + 1 + end + 1..].trim_start();
            if let Some(val) = after.strip_prefix(':') {
                if val.trim_start().starts_with(char::from(lead)) {
                    n += 1;
                }
            }
        }
        rest = &rest[q + 1 + end + 1..];
    }
    n
}

impl DockerDaemon {
    /// Parse daemon.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        if !t.trim_start().starts_with('{') {
            return None;
        }
        let comments = t
            .lines()
            .filter(|l| l.trim_start().starts_with("//") || l.trim_start().starts_with('#'))
            .count();
        let mut keys = 0usize;
        let mut rest = t;
        while let Some(q) = rest.find('"') {
            let Some(end) = rest[q + 1..].find('"') else {
                break;
            };
            let key = &rest[q + 1..q + 1 + end];
            let after = rest[q + 1 + end + 1..].trim_start();
            if KEYS.contains(&key) && after.starts_with(':') {
                keys += 1;
            }
            rest = &rest[q + 1 + end + 1..];
        }
        let array_keys = count_key(t, b'[');
        let object_keys = count_key(t, b'{');
        if keys == 0 {
            return None;
        }
        Some(Self {
            keys,
            array_keys,
            object_keys,
            comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        concat!(
            "{\"data-root\":\"/var/lib/docker\",",
            "\"storage-driver\":\"overlay2\",",
            "\"insecure-registries\":[\"localhost:5000\"],",
            "\"registry-mirrors\":[\"https://m.local\"],",
            "\"log-driver\":\"json-file\",",
            "\"log-opts\":{\"max-size\":\"10m\"},",
            "\"experimental\":true}",
        )
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = DockerDaemon::parse(b.as_bytes()).unwrap();
        assert_eq!(c.keys, 7);
        assert_eq!(c.array_keys, 2);
        assert_eq!(c.object_keys, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"port\":8080}"));
        assert!(DockerDaemon::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

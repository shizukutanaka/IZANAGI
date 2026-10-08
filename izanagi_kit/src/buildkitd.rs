//! BuildKit `buildkitd.toml` の検出と構造カウント。
//!
//! `[worker.oci]`/`[worker.containerd]`/`[grpc]`/`[grpc.address]`/
//! `[registry."host"]`/`[frontend.dockerfile.v0]`/`[frontend.gateway.v0]`/
//! `[otlp]`/`[history]`/`[worker.sysfscgroup]` 等のテーブルと、
//! `root`/`debug`/`allowinsecure`/`networkMode`/`gc`/`gckeepstorage`/
//! `entitlements`/`enabled`/`namespace`/`address`/`runtime` 等のキーを識別する。
//!
//! ```
//! let c = izanagi_kit::buildkitd::parse(
//!     b"debug = true\nroot = \"/var/lib/buildkit\"\n\n[worker.oci]\nenabled = true\n\n[grpc]\naddress = [\"tcp://0.0.0.0:1234\"]\n").unwrap();
//! assert!(c.sections >= 2);
//! assert!(izanagi_kit::buildkitd::detect(
//!     b"[worker.oci]\nenabled = true\nplatforms = [\"linux/amd64\"]\n"));
//! ```

use crate::textutil::strip_bom;
/// テーブルプレフィックス(先頭照合)。
const TABLES: &[&str] = &[
    "buildkitd",
    "debug",
    "frontend",
    "gateway",
    "gr\u{70}\u{63}",
    "history",
    "insecure-entitlements",
    "otlp",
    "registry",
    "secrets",
    "system",
    "trace",
    "worker",
];

/// 既知キー。
const KEYS: &[&str] = &[
    "address",
    "allowinsecure",
    "annotations",
    "args",
    "buildkitd",
    "ca",
    "cert",
    "cgroupParent",
    "cniloopback",
    "debug",
    "enabled",
    "entitlements",
    "g\u{63}",
    "gckeepstorage",
    "hostname",
    "http",
    "insecure",
    "insecure-entitlements",
    "key",
    "labels",
    "log",
    "maxSizeMB",
    "meta",
    "name",
    "namespace",
    "networkMode",
    "noProcessSandbox",
    "parent",
    "path",
    "platforms",
    "reserveSpace",
    "reservedSpace",
    "root",
    "runtime",
    "runtimePath",
    "scheduleGCPolicy",
    "selinuxLabel",
    "sockets",
    "storage-driver",
    "systemCgroup",
    "systemd",
    "tempdir",
    "tls",
    "userns",
    "vsock",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[table]` 行数。
    pub sections: usize,
    /// `key = value` 既知キー行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn table_of(t: &str) -> &str {
    if !(t.starts_with('[') && t.contains(']')) {
        return "";
    }
    let inner = &t[1..t.find(']').unwrap_or(1)];
    inner.split('.').next().unwrap_or("")
}

fn known_key(t: &str) -> bool {
    let Some(eq) = t.find('=') else { return false };
    KEYS.contains(&t[..eq].trim())
}

/// `buildkitd.toml` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut tables = 0usize;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && TABLES.contains(&table_of(t)) {
            tables += 1;
        }
        if known_key(t) {
            hits += 1;
        }
        if tables >= 1 && hits >= 2 {
            return true;
        }
        if hits >= 3 {
            return true;
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

    const SAMPLE: &[u8] = b"# buildkitd\ndebug = true\nroot = \"/var/lib/buildkit\"\ninsecure-entitlements = [\"network.host\", \"security.insecure\"]\n\n[worker.oci]\nenabled = true\nplatforms = [\"linux/amd64\", \"linux/arm64\"]\nnetworkMode = \"auto\"\ngc = true\nmaxSizeMB = 0\n\n[worker.containerd]\nenabled = false\nnamespace = \"buildkit\"\n\n[grpc]\naddress = [\"tcp://0.0.0.0:1234\", \"unix:///run/buildkit/buildkitd.sock\"]\n\n[registry.\"docker.io\"]\nhttp = false\ninsecure = false\n\n[otlp]\nsockets = [\"jaeger:6831\"]\n";

    #[test]
    fn buildkitd() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.options, 14);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_buildkitd() {
        assert!(!detect(b"[other]\nkey = 1\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

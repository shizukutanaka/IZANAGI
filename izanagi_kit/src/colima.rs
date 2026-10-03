//! Colima `colima.yaml`/`colima.default.yaml` の検出と構造カウント。
//!
//! `cpu`/`memory`/`disk`/`arch`/`autoActivate`/`forwardAgent`/`kubernetes`/
//! `env`/`network`/`mountType`/`sshConfig`/`vmType`/`runtime`/`hostname`/
//! `mounts`/`provision`/`rosetta`/`layers`/`diskImage` 等の既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::colima::parse(
//!     b"cpu: 4\nmemory: 8\ndisk: 60\nkubernetes:\n  enabled: true\nruntime: docker\n").unwrap();
//! assert!(c.options >= 5);
//! assert!(izanagi_kit::colima::detect(
//!     b"cpu: 2\ndisk: 20\nautoActivate: true\nvmType: vz\n"));
//! ```

/// Colima 既知設定キー(トップ + kubernetes/network ネスト)。
const KEYS: &[&str] = &[
    "address",
    "arch",
    "autoActivate",
    "cpu",
    "cpuType",
    "daemon",
    "deactivateRuntime",
    "deactivated",
    "disk",
    "diskImage",
    "docker",
    "driver",
    "enable",
    "enabled",
    "env",
    "forwardAgent",
    "forwardSSA",
    "hostname",
    "image",
    "interface",
    "k3s",
    "k3sArgs",
    "kubernetes",
    "kubernetesVersion",
    "layers",
    "limaInstance",
    "location",
    "memory",
    "mounts",
    "mountType",
    "nestedVirtualization",
    "network",
    "networkAddress",
    "number",
    "provision",
    "rosetta",
    "runtime",
    "serviceAccount",
    "socket_vmnet_path",
    "ssh",
    "sshPort",
    "sshConfig",
    "timer",
    "version",
    "vmType",
    "vtDaemon",
    "vtWarn",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key:` ブロック行数。
    pub sections: usize,
    /// `- ` リスト要素行数。
    pub entries: usize,
    /// 既知 `key:` 行数。
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

/// `colima.yaml` らしさを判定する。
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
        sections: 0,
        entries: 0,
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
        if t.starts_with("- ") {
            c.entries += 1;
            continue;
        }
        if t.ends_with(':') {
            c.sections += 1;
            if known_key(t) {
                c.options += 1;
            }
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

    const SAMPLE: &[u8] = b"# colima\ncpu: 4\nmemory: 8\ndisk: 100\narch: aarch64\nautoActivate: true\nforwardAgent: false\n\nkubernetes:\n  enabled: true\n  version: v1.28.0+k3s1\n\nnetwork:\n  address: true\n  dns:\n    - 8.8.8.8\n    - 1.1.1.1\n\nruntime: docker\nvmType: vz\nrosetta: true\nmountType: sshfs\nmounts: []\nenv: {}\n";

    #[test]
    fn colima() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 2);
        assert!(c.options >= 15);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_colima() {
        assert!(!detect(b"foo: 1\nbar: 2\nbaz: 3\n"));
        assert!(parse(b"text\n").is_none());
    }
}

//! k0s `ClusterConfig`(`k0s.yaml`/`cluster.yaml`)の検出と構造カウント。
//!
//! `apiVersion: k0s.k0sproject.io/v1beta1` + `kind: ClusterConfig` 必須と、
//! `spec:` 内 `api`/`controllerManager`/`scheduler`/`network`/`storage`/
//! `konnectivity`/`extensions`/`telemetry`/`workerProfiles`/`installConfig`
//! 等の既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::k0sconf::parse(
//!     b"apiVersion: k0s.k0sproject.io/v1beta1\nkind: ClusterConfig\nmetadata:\n  name: k0s\nspec:\n  api:\n    address: 192.168.0.1\n").unwrap();
//! assert!(c.options >= 2);
//! assert!(izanagi_kit::k0sconf::detect(
//!     b"apiVersion: k0s.k0sproject.io/v1beta1\nkind: ClusterConfig\n"));
//! ```

/// k0s spec 内既知キー。
const KEYS: &[&str] = &[
    "address",
    "api",
    "args",
    "bindAddress",
    "ca",
    "clusterDNS",
    "clusterDomain",
    "controllerManager",
    "dns",
    "dualStack",
    "enabled",
    "eventRateLimit",
    "etcd",
    "extraArgs",
    "featureGates",
    "hairpinMode",
    "images",
    "installConfig",
    "ips",
    "kine",
    "konnectivity",
    "kubeProxy",
    "loadBalancer",
    "metricsServer",
    "network",
    "nodeLocalLoadBalancing",
    "nodePortRange",
    "peerAddress",
    "podCIDR",
    "port",
    "provider",
    "sans",
    "scheduler",
    "serviceCIDR",
    "storage",
    "telemetry",
    "type",
    "workerProfiles",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `apiVersion`/`kind`/`metadata`/`spec`/`status` 行数。
    pub sections: usize,
    /// ネストマップ(`key:`)行数。
    pub entries: usize,
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

/// `key : value` 行のコメント手前までの値部分を返す。
///
/// キーと `:` の間の空白(`apiVersion :`)は許容し、` #` 以降の
/// インラインコメントは値から除く。
fn value_of<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix(':')?;
    let value = rest.split(" #").next().unwrap_or(rest);
    Some(value.trim().trim_matches(|c| c == '"' || c == '\''))
}

/// `ClusterConfig` らしさを判定する(`k0s.k0sproject.io` + `ClusterConfig`)。
///
/// 両マーカーがトップレベルの `apiVersion:`/`kind:` キー行の
/// 値位置にあることだけを見る — コメントや文字列値の中に
/// 同じ語が現れても検出しない。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut api = false;
    let mut kind = false;
    for line in text.lines() {
        if value_of(line, "apiVersion").is_some_and(|v| v.starts_with("k0s.k0sproject.io")) {
            api = true;
        }
        if value_of(line, "kind").is_some_and(|v| v.starts_with("ClusterConfig")) {
            kind = true;
        }
        if api && kind {
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
        let k = key_of(t);
        if matches!(
            k,
            "apiVersion" | "kind" | "metadata" | "spe\u{63}" | "status"
        ) && t.starts_with(k)
            && line.chars().take_while(|c| c.is_whitespace()).count() == 0
        {
            c.sections += 1;
            continue;
        }
        if t.starts_with("- ") {
            c.entries += 1;
            continue;
        }
        if t.ends_with(':') {
            c.entries += 1;
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

    const SAMPLE: &[u8] = b"apiVersion: k0s.k0sproject.io/v1beta1\nkind: ClusterConfig\nmetadata:\n  name: k0s\nspec:\n  api:\n    address: 192.168.0.10\n    port: 6443\n    sans:\n      - 192.168.0.10\n      - k0s.local\n    extraArgs:\n      - feature-gates: \"\"\n  controllerManager:\n    extraArgs:\n      - \"--bind-address=0.0.0.0\"\n  scheduler:\n    extraArgs: {}\n  network:\n    provider: calico\n    podCIDR: 10.244.0.0/16\n    serviceCIDR: 10.96.0.0/12\n    clusterDomain: cluster.local\n    kubeProxy:\n      mode: ipvs\n  storage:\n    type: kine\n    kine:\n      dataSource: sqlite:///var/lib/k0s/db/state.db\n  konnectivity:\n    agentPort: 8132\n    adminPort: 8133\n  telemetry:\n    enabled: true\n";

    #[test]
    fn k0sconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.entries, 16);
        assert!(c.options >= 16);
        assert_eq!(c.misc, 5);
    }

    #[test]
    fn not_k0sconf() {
        assert!(!detect(b"apiVersion: v1\nkind: ConfigMap\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn commented_markers_do_not_detect() {
        // Both markers present but only inside comments / string values.
        assert!(!detect(
            b"# apiVersion: k0s.k0sproject.io/v1beta1\n# kind: ClusterConfig\n"
        ));
        assert!(!detect(
            b"apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: \"k0s.k0sproject.io ClusterConfig\"\n"
        ));
        // kind is real but apiVersion is comment-only -> must not detect.
        assert!(!detect(
            b"#   apiVersion: k0s.k0sproject.io/v1beta1\nkind: ClusterConfig\n"
        ));
    }

    #[test]
    fn markers_need_top_level_keys() {
        assert!(detect(
            b"apiVersion: k0s.k0sproject.io/v1beta1\nkind: ClusterConfig\n"
        ));
        // Indented (non-top-level) occurrences do not qualify.
        assert!(!detect(
            b"spec:\n  apiVersion: k0s.k0sproject.io/v1beta1\n  kind: ClusterConfig\n"
        ));
    }

    #[test]
    fn spaced_colon_and_inline_comments() {
        // `apiVersion :` / `kind :` (space before colon) still counts.
        assert!(detect(
            b"apiVersion : k0s.k0sproject.io/v1beta1\nkind : ClusterConfig\n"
        ));
        // Markers inside an inline comment do not count as the value.
        assert!(!detect(
            b"apiVersion: v1 # k0s.k0sproject.io\nkind: ConfigMap # ClusterConfig\n"
        ));
        // Quoted values are accepted.
        assert!(detect(
            b"apiVersion: \"k0s.k0sproject.io/v1beta1\"\nkind: 'ClusterConfig'\n"
        ));
    }
}

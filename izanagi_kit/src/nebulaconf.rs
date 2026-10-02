//! Nebula `config.yml` の検出と構造カウント。
//!
//! `pki`/`static_host_map`/`lighthouse`/`tun`/`firewall`/`listen`/`logging`/
//! `ciphers`/`local_range`/`ssh`/`relay`/`preferred_ranges`/`unsafe_routes` 等の
//! 既知トップレベルセクションと `am_lighthouse`/`hosts`/`inbound`/`outbound`/
//! `port`/`action`/`proto`/`groups` 等の既知サブキーを識別する。
//!
//! ```
//! let c = izanagi_kit::nebulaconf::parse(
//!     b"pki:\n  ca: ca.crt\nlighthouse:\n  am_lighthouse: true\nfirewall:\n  inbound:\n    - port: any\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert!(izanagi_kit::nebulaconf::detect(b"lighthouse:\n  am_lighthouse: true\ntun:\n  dev: nebula1\n"));
//! ```

/// 既知トップレベルセクション。
const TOP_KEYS: &[&str] = &[
    "ciphers",
    "conntrack",
    "firewall",
    "handshakes",
    "ip",
    "is_lighthouse",
    "lighthouse",
    "listen",
    "local_ips",
    "local_range",
    "logging",
    "pki",
    "preferred_ranges",
    "punchy",
    "relay",
    "ssh",
    "sshd",
    "stats",
    "static_host_map",
    "tun",
    "unsafe_routes",
];
/// ネストされた既知サブキー。
const SUBKEYS: &[&str] = &[
    "action",
    "am_lighthouse",
    "authkey",
    "blocklist",
    "ca",
    "cert",
    "cidr",
    "cipher",
    "code",
    "connected",
    "default_queue",
    "dev",
    "drop_local_broadcast",
    "drop_multicast",
    "duration",
    "enabled",
    "gateways",
    "groups",
    "host",
    "hosts",
    "inbound",
    "interface",
    "interval",
    "ip",
    "ipv6",
    "level",
    "listen",
    "mtu",
    "nebula",
    "outbound",
    "port",
    "proto",
    "prometheus",
    "queue",
    "remote_ips",
    "retries",
    "route",
    "routes",
    "serve_dns",
    "ssh",
    "subnet",
    "tcp_queue",
    "timeout",
    "trustee",
    "tx_queue",
    "udp",
    "use_remote",
    "users",
    "version",
];

/// Nebula 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルセクション(非インデント `key:`)。
    pub sections: usize,
    /// ネスト `key: value` 既知代入。
    pub options: usize,
    /// `- ` リスト項目。
    pub items: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// YAML の `key:` 行のキーを返す。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        None
    } else {
        Some(k)
    }
}

/// b が Nebula config.yml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        let indent = line.len() - line.trim_start().len();
        if let Some(k) = yaml_key(t) {
            if indent == 0 && TOP_KEYS.contains(&k) || indent > 0 && SUBKEYS.contains(&k) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        items: 0,
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
            c.items += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if yaml_key(t).is_some() {
            if indent == 0 {
                c.sections += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# nebula\npki:\n  ca: /etc/nebula/ca.crt\n  cert: /etc/nebula/host.crt\n  key: /etc/nebula/host.key\nlighthouse:\n  am_lighthouse: true\n  interval: 60\ntun:\n  dev: nebula1\nfirewall:\n  inbound:\n    - port: any\n      proto: any\n      host: any\n";

    #[test]
    fn nebulaconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.items, 1);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_nebula() {
        assert!(!detect(b"foo:\n  bar: baz\n"));
        assert!(!detect(b"hello\n"));
    }
}

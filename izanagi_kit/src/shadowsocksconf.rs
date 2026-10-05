//! Shadowsocks `config.json` (server/client/manager JSON).
//!
//! ```
//! let b = b"{\n  \"server\": \"0.0.0.0\",\n  \"server_port\": 8388,\n  \"local_port\": 1080,\n  \"password\": \"secret\",\n  \"method\": \"aes-256-gcm\",\n  \"timeout\": 300,\n  \"fast_open\": true,\n  \"workers\": 4,\n  \"mode\": \"tcp_and_udp\"\n}\n";
//! assert!(izanagi_kit::shadowsocksconf::detect(b));
//! let c = izanagi_kit::shadowsocksconf::Shadowsocks::parse(b).unwrap();
//! assert!(c.keys >= 8);
//! ```
const KEYS: &[&str] = &[
    "server",
    "server_port",
    "local_port",
    "local_address",
    "password",
    "method",
    "timeout",
    "fast_open",
    "workers",
    "nameserver",
    "mode",
    "plugin",
    "plugin_opts",
    "plugin_args",
    "reuse_port",
    "no_delay",
    "mptcp",
    "nofile",
    "acl",
    "address",
    "port_password",
    "key",
    "port",
    "remarks",
    "auth",
    "obfs",
    "obfs_param",
    "protocol",
    "protocol_param",
    "redirect",
    "dns_ipv6",
    "connect_verbose_info",
    "manager_address",
    "out_bindaddr",
    "key_path",
    "cert_path",
    "total_time",
    "total_bytes",
    "key_dn",
    "key_cn",
    "key_sn",
    "key_issuer",
    "shadowsocks",
    "ipv6_first",
    "prefer_ipv6",
    "disable_hit_cache",
    "latency",
    "throughput",
    "weight",
    "group",
    "strategy",
    "health_check",
    "check_interval",
    "check_timeout",
    "check_healthy",
    "max_stream",
];
const METHODS: &[&str] = &[
    "aes-128-gcm",
    "aes-192-gcm",
    "aes-256-gcm",
    "aes-128-cfb",
    "aes-192-cfb",
    "aes-256-cfb",
    "aes-128-ctr",
    "aes-192-ctr",
    "aes-256-ctr",
    "aes-128-ofb",
    "aes-192-ofb",
    "aes-256-ofb",
    "bf-cfb",
    "camellia-128-cfb",
    "camellia-192-cfb",
    "camellia-256-cfb",
    "chacha20",
    "chacha20-ietf",
    "chacha20-ietf-poly1305",
    "salsa20",
    "xchacha20-ietf-poly1305",
    "rc4-md5",
    "rc4-md5-6",
    "seed-cfb",
    "2022-blake3-aes-128-gcm",
    "2022-blake3-aes-256-gcm",
    "2022-blake3-chacha8-poly1305",
    "2022-blake3-chacha20-poly1305",
    "none",
    "plain",
];

/// Detect a Shadowsocks config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if !t.contains('{') {
        return false;
    }
    let mut hits = 0usize;
    for k in KEYS {
        if t.contains(k) {
            hits += 1;
        }
    }
    let method = METHODS.iter().any(|m| t.contains(m));
    hits >= 4 || (method && hits >= 2)
}

/// Structural counts for a Shadowsocks config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shadowsocks {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `"key":` JSON members (all lines).
    pub members: usize,
    /// Known cipher method strings.
    pub methods: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

impl Shadowsocks {
    /// Parse structural counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            members: 0,
            methods: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with('#') {
                c.comments += 1;
            }
            c.members += tr.matches("\":").count();
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        for m in METHODS {
            c.methods += t.matches(m).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"{\n  \"server\": \"0.0.0.0\",\n  \"server_port\": 8388,\n  \"local_port\": 1080,\n  \"password\": \"secret\",\n  \"method\": \"aes-256-gcm\",\n  \"timeout\": 300,\n  \"fast_open\": true,\n  \"mode\": \"tcp_and_udp\"\n}\n";
        assert!(detect(b));
        let c = Shadowsocks::parse(b).unwrap();
        assert_eq!(c.members, 8);
        assert_eq!(c.methods, 1);
        assert!(c.keys >= 8);
    }

    #[test]
    fn rejects_json() {
        let b = b"{\"foo\": 1}\n";
        assert!(!detect(b));
        assert!(Shadowsocks::parse(b).is_none());
    }
}

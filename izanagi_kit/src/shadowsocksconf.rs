//! Shadowsocks `config.json`(ssserver/sslocal/ssurl 共通形式)の検出と構造カウント。
//!
//! `"server"`/`"server_port"`/`"local_port"`/`"local_address"`/`"password"`/
//! `"method"`/`"timeout"`/`"fast_open"`/`"reuse_port"`/`"mode"`/`"plugin"`/
//! `"plugin_opts"`/`"nameserver"`/`"acl"`/`"manager_address"` 等の既知キー +
//! 暗号メソッド値(`"aes-256-gcm"`/`"chacha20-ietf-poly1305"`/`"2022-blake3-*"` 等)を識別する。
//!
//! ```
//! let c = izanagi_kit::shadowsocksconf::parse(
//!     b"{\n  \"server\": \"0.0.0.0\",\n  \"server_port\": 8388,\n  \"password\": \"x\",\n  \"method\": \"aes-256-gcm\"\n}\n").unwrap();
//! assert_eq!(c.keys, 3);
//! assert!(izanagi_kit::shadowsocksconf::detect(
//!     b"{\n  \"server\": \"0.0.0.0\",\n  \"method\": \"chacha20-ietf-poly1305\",\n  \"password\": \"x\"\n}\n"));
//! ```

/// 既知キー。
const KEYS: &[&str] = &[
    "acl",
    "allowed_users",
    "auth_threads",
    "balancer",
    "dns",
    "external_dns",
    "fast_open",
    "forwarding",
    "ipv6_first",
    "key",
    "local_address",
    "local_dns_address",
    "local_port",
    "local_path",
    "local_socket",
    "manager_address",
    "max_request_size",
    "method",
    "mode",
    "nameserver",
    "no_delay",
    "outbound_bind_addr",
    "outbound_bind_interface",
    "outbound_fwnat",
    "outbound_v6_fwmark",
    "password",
    "plugin",
    "plugin_args",
    "plugin_mode",
    "plugin_opts",
    "port",
    "protocol",
    "remotes",
    "replay_attack_policy",
    "server",
    "server_port",
    "servers",
    "socks5_auth_config_path",
    "timeout",
    "tunnel_address",
    "udp_max_associations",
    "udp_relay_policy",
    "udp_timeout",
    "unsafe_request_max_clients",
];
/// 既知暗号メソッド値。
const METHODS: &[&str] = &[
    "2022-blake3-aes-128-gcm",
    "2022-blake3-aes-256-gcm",
    "2022-blake3-chacha20-poly1305",
    "aes-128-cfb",
    "aes-128-ctr",
    "aes-128-gcm",
    "aes-128-ofb",
    "aes-192-cfb",
    "aes-192-ctr",
    "aes-192-gcm",
    "aes-192-ofb",
    "aes-256-cfb",
    "aes-256-ctr",
    "aes-256-gcm",
    "aes-256-ofb",
    "bf-cfb",
    "camellia-128-cfb",
    "camellia-192-cfb",
    "camellia-256-cfb",
    "chacha20",
    "chacha20-ietf",
    "chacha20-ietf-poly1305",
    "chacha20-poly1305",
    "des-cfb",
    "idea-cfb",
    "none",
    "plain",
    "rc4",
    "rc4-md5",
    "salsa20",
    "seed-cfb",
    "xchacha20-ietf-poly1305",
];

/// Shadowsocks 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"key":` 既知キー出現。
    pub keys: usize,
    /// `"method": "<cipher>"` 既知暗号メソッド。
    pub ciphers: usize,
    /// `servers` 内のサーバエントリ行(`{`/`}`/`server` 再出現)。
    pub server_entries: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行内の全 `"key":` 出現を列挙。
fn keys_in_line<'a>(t: &'a str, out: &mut Vec<&'a str>) {
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j < bytes.len() && j > start {
                let mut k = j + 1;
                while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b':' {
                    out.push(&t[start..j]);
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
}

/// 行の `"method": "..."` 値を抽出。
fn method_value(t: &str) -> Option<&str> {
    let p = t.find("\"method\"")?;
    let rest = &t[p + 8..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// b が shadowsocks config.json かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    let mut cipher = false;
    let mut keys = Vec::new();
    for line in text.lines() {
        keys.clear();
        keys_in_line(line, &mut keys);
        for k in &keys {
            if KEYS.contains(k) {
                hits += 1;
            }
            if *k == "method" && method_value(line).is_some_and(|v| METHODS.contains(&v)) {
                cipher = true;
            }
        }
    }
    hits >= 2 && cipher || hits >= 3
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        keys: 0,
        ciphers: 0,
        server_entries: 0,
        misc: 0,
    };
    let mut keys = Vec::new();
    let mut in_servers = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        keys.clear();
        keys_in_line(line, &mut keys);
        if keys.is_empty() {
            if in_servers && (t == "{" || t.starts_with('{') && t.contains("\"server\"")) {
                c.server_entries += 1;
            }
            continue;
        }
        for k in &keys {
            if *k == "servers" {
                in_servers = true;
                c.keys += 1;
            } else if *k == "method" {
                if method_value(line).is_some_and(|v| METHODS.contains(&v)) {
                    c.ciphers += 1;
                } else {
                    c.keys += 1;
                }
            } else if KEYS.contains(k) {
                c.keys += 1;
            } else {
                c.misc += 1;
            }
        }
    }
    (c.keys + c.ciphers >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"server\": \"0.0.0.0\",\n  \"server_port\": 8388,\n  \"local_address\": \"127.0.0.1\",\n  \"local_port\": 1080,\n  \"password\": \"secret\",\n  \"method\": \"aes-256-gcm\",\n  \"timeout\": 300,\n  \"fast_open\": true,\n  \"mode\": \"tcp_and_udp\"\n}\n";

    #[test]
    fn shadowsocksconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.keys, 8);
        assert_eq!(c.ciphers, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_shadowsocks() {
        assert!(!detect(b"{\"foo\": 1}\n"));
        assert!(!detect(b"hello\n"));
    }
}

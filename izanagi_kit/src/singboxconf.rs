//! sing-box `config.json` (log/dns/ntp/inbounds/outbounds/route/endpoints/experimental).
//!
//! ```
//! let b = b"{\n  \"log\": {\"level\": \"info\", \"timestamp\": true},\n  \"dns\": {\"servers\": [{\"tag\": \"remote\", \"address\": \"https://dns.google/dns-query\"}]},\n  \"inbounds\": [{\"type\": \"mixed\", \"tag\": \"in\", \"listen\": \"127.0.0.1\", \"listen_port\": 2080}],\n  \"outbounds\": [{\"type\": \"vless\", \"tag\": \"out\", \"server\": \"example.com\", \"server_port\": 443, \"uuid\": \"uuid\", \"tls\": {\"enabled\": true}}],\n  \"route\": {\"rules\": [{\"geosite\": \"cn\", \"outbound\": \"direct\"}], \"final\": \"out\"}\n}\n";
//! assert!(izanagi_kit::singboxconf::detect(b));
//! let c = izanagi_kit::singboxconf::Singbox::parse(b).unwrap();
//! assert!(c.sections >= 4);
//! ```
const SECTION_KEYS: &[&str] = &[
    "\"log\"",
    "\"dns\"",
    "\"ntp\"",
    "\"inbounds\"",
    "\"outbounds\"",
    "\"route\"",
    "\"endpoints\"",
    "\"services\"",
    "\"experimental\"",
    "\"certificate\"",
];
const TYPE_VALUES: &[&str] = &[
    "direct",
    "block",
    "dns",
    "socks",
    "http",
    "mixed",
    "tun",
    "tuntap",
    "redirect",
    "tproxy",
    "vmess",
    "vless",
    "trojan",
    "shadowsocks",
    "shadowtls",
    "hysteria",
    "hysteria2",
    "tui\u{63}",
    "wireguard",
    "tor",
    "ssh",
    "selector",
    "urltest",
    "anytls",
    "juicity",
    "mieru",
    "naive",
    "reality",
    "ws",
    "httpupgrade",
    "grp\u{63}",
    "qui\u{63}",
    "h2",
    "h3",
];
const KEYS: &[&str] = &[
    "type",
    "tag",
    "listen",
    "listen_port",
    "server",
    "server_port",
    "local_address",
    "local_port",
    "uuid",
    "password",
    "method",
    "tls",
    "transport",
    "enabled",
    "reality",
    "handshake",
    "public_key",
    "short_id",
    "utls",
    "flow",
    "network",
    "security",
    "alpn",
    "server_name",
    "certificate",
    "private_key",
    "insecure",
    "fragment",
    "multiplex",
    "udp_over_tcp",
    "udp_fragment",
    "detour",
    "domain_strategy",
    "fallback_delay",
    "route",
    "rules",
    "final",
    "geoip",
    "geosite",
    "rule_set",
    "rule-sets",
    "default_domain_resolver",
    "auto_detect_interface",
    "override_android_vpn",
    "strict_route",
    "outbound",
    "inbound",
    "action",
    "process_name",
    "process_path",
    "package_name",
    "user",
    "user_id",
    "clash_mode",
    "wifi_ssid",
    "wifi_bssid",
    "protocol",
    "port",
    "port_range",
    "source_ip_cidr",
    "ip_cidr",
    "source_port",
    "source_port_range",
    "network_type",
    "network_is_expensive",
    "network_is_constrained",
    "domain",
    "domain_suffix",
    "domain_keyword",
    "domain_regex",
    "ip_version",
    "source_ip_is_private",
    "ip_is_private",
    "invert",
    "bypass",
    "hijack-dns",
    "address",
    "address_resolver",
    "address_strategy",
    "strategy",
    "client_subnet",
    "fakeip",
    "filter_aaaa",
    "independent_cache",
    "reverse_mapping",
    "mapping",
    "ttl",
    "rcode",
    "rewrite_ttl",
    "predefined",
    "hosts",
    "path",
    "headers",
    "max_early_data",
    "early_data_header_name",
    "service_name",
    "idle_timeout",
    "ping_timeout",
    "host",
    "masquerade",
    "system",
    "name_servers",
    "dialer",
    "prefix",
    "peer_public_key",
    "pre_shared_key",
    "reserved",
    "mtu",
    "workers",
    "keepalive",
    "gso",
    "endpoint_independent_nat",
];

/// Detect a sing-box config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if !t.contains('{') {
        return false;
    }
    let mut sec = 0usize;
    for k in SECTION_KEYS {
        if t.contains(k) {
            sec += 1;
        }
    }
    let mut hits = 0usize;
    for v in TYPE_VALUES {
        if t.contains(&format!("\"{v}\"")) {
            hits += 1;
        }
    }
    sec >= 2 || hits >= 3
}

/// Structural counts for a sing-box config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Singbox {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `"name":` top-level section keys.
    pub sections: usize,
    /// Known `"type"`/`"network"`/`"security"` values.
    pub type_values: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

impl Singbox {
    /// Parse structural counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            sections: 0,
            type_values: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in SECTION_KEYS {
            c.sections += t.matches(k).count();
        }
        for v in TYPE_VALUES {
            c.type_values += t.matches(&format!("\"{v}\"")).count();
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"{\n  \"log\": {\"level\": \"info\"},\n  \"dns\": {},\n  \"inbounds\": [{\"type\": \"mixed\", \"listen_port\": 2080}],\n  \"outbounds\": [{\"type\": \"vless\"}, {\"type\": \"direct\"}],\n  \"route\": {\"final\": \"out\"}\n}\n";
        assert!(detect(b));
        let c = Singbox::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert!(c.type_values >= 3);
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_json() {
        let b = b"{\"name\": \"foo\"}\n";
        assert!(!detect(b));
        assert!(Singbox::parse(b).is_none());
    }
}

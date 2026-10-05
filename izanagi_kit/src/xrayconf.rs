//! Xray-core `config.json` (JSON inbounds/outbounds/routing/dns).
//!
//! ```
//! let b = b"{\n  \"log\": {\"loglevel\": \"warning\"},\n  \"inbounds\": [{\"port\": 1080, \"protocol\": \"socks\", \"listen\": \"127.0.0.1\"}],\n  \"outbounds\": [{\"protocol\": \"vless\", \"settings\": {\"vnext\": [{\"address\": \"example.com\", \"port\": 443, \"users\": [{\"id\": \"uuid\", \"encryption\": \"none\"}]}]}, \"streamSettings\": {\"network\": \"ws\", \"security\": \"tls\"}}],\n  \"routing\": {\"rules\": [{\"type\": \"field\", \"ip\": [\"geoip:private\"], \"outboundTag\": \"block\"}]}\n}\n";
//! assert!(izanagi_kit::xrayconf::detect(b));
//! let c = izanagi_kit::xrayconf::Xray::parse(b).unwrap();
//! assert!(c.sections >= 3);
//! ```
const SECTION_KEYS: &[&str] = &[
    "\"log\"",
    "\"api\"",
    "\"dns\"",
    "\"routing\"",
    "\"policy\"",
    "\"inbounds\"",
    "\"outbounds\"",
    "\"transport\"",
    "\"stats\"",
    "\"reverse\"",
    "\"fakedns\"",
    "\"browser\"",
    "\"observatory\"",
    "\"burstObservatory\"",
    "\"metrics\"",
];
const PROTOCOL_VALUES: &[&str] = &[
    "vmess",
    "vless",
    "trojan",
    "shadowsocks",
    "socks",
    "http",
    "dokodemo-door",
    "freedom",
    "blackhole",
    "dns",
    "fakedns",
    "wireguard",
    "loopback",
];
const KEYS: &[&str] = &[
    "protocol",
    "port",
    "listen",
    "address",
    "settings",
    "streamSettings",
    "network",
    "security",
    "sniffing",
    "tag",
    "outboundTag",
    "inboundTag",
    "rules",
    "domain",
    "ip",
    "geoip",
    "geosite",
    "type",
    "field",
    "balancerTag",
    "strategy",
    "domainStrategy",
    "destOverride",
    "enabled",
    "sendThrough",
    "dokodemo",
    "vnext",
    "users",
    "id",
    "level",
    "alterId",
    "encryption",
    "flow",
    "clients",
    "servers",
    "serverName",
    "publicKey",
    "shortId",
    "fingerprint",
    "alpn",
    "minVersion",
    "maxVersion",
    "cipherSuites",
    "certificates",
    "certificateFile",
    "keyFile",
    "rejectUnknownSNI",
    "allowInsecure",
    "allowInsecureCiphers",
    "disableSystemRoot",
    "enableSessionResumption",
    "peerName",
    "pinnedPeerCertChainSha256",
    "wsSettings",
    "grpcSettings",
    "httpSettings",
    "tcpSettings",
    "kcpSettings",
    "quicSettings",
    "dsSettings",
    "httpupgradeSettings",
    "splithttpSettings",
    "sockopt",
    "mark",
    "tcpFastOpen",
    "tproxy",
    "tcpMptcp",
    "tcpNoDelay",
    "acceptProxyProtocol",
    "path",
    "host",
    "headers",
    "serviceName",
    "multiMode",
    "idle_timeout",
    "health_check_timeout",
    "permit_without_stream",
    "initial_windows_size",
    "method",
    "password",
    "email",
    "uot",
    "UoT",
    "mux",
    "concurrency",
    "xver",
    "version",
    "request",
    "response",
    "mtu",
    "reserved",
    "secretKey",
    "peers",
    "endpoint",
    "keepAlive",
    "kernelMode",
    "domainMatcher",
    "balancers",
    "rule",
    "rules_balancer",
];

/// Detect an Xray config.json.
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
    for v in PROTOCOL_VALUES {
        if t.contains(&format!("\"{v}\"")) {
            hits += 1;
        }
    }
    sec >= 2 || (sec >= 1 && hits >= 1) || hits >= 2
}

/// Structural counts for an Xray config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xray {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `"name":` top-level section keys.
    pub sections: usize,
    /// Known `"protocol"`/`"network"`/`"security"`/`"type"` values.
    pub protocol_values: usize,
    /// `//`/`#` comment lines (Xray allows them).
    pub comments: usize,
}

impl Xray {
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
            protocol_values: 0,
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
        for v in PROTOCOL_VALUES {
            c.protocol_values += t.matches(&format!("\"{v}\"")).count();
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
        let b = b"{\n  \"inbounds\": [{\"port\": 1080, \"protocol\": \"socks\"}],\n  \"outbounds\": [{\"protocol\": \"vless\"}, {\"protocol\": \"freedom\"}],\n  \"routing\": {\"rules\": []}\n}\n";
        assert!(detect(b));
        let c = Xray::parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.protocol_values, 3);
        assert!(c.keys >= 3);
    }

    #[test]
    fn rejects_json() {
        let b = b"{\"name\": \"foo\", \"version\": 1}\n";
        assert!(!detect(b));
        assert!(Xray::parse(b).is_none());
    }
}

//! Clash/Clash Meta `config.yaml` (proxies + proxy-groups + rules).
//!
//! ```
//! let b = b"port: 7890\nsocks-port: 7891\nmixed-port: 7893\nallow-lan: false\nmode: rule\nlog-level: info\nproxies:\n  - name: \"node1\"\n    type: vmess\n    server: example.com\n    port: 443\nproxy-groups:\n  - name: PROXY\n    type: select\n    proxies: [\"node1\", \"DIRECT\"]\nrules:\n  - DOMAIN-SUFFIX,google.com,PROXY\n  - IP-CIDR,10.0.0.0/8,DIRECT\n  - MATCH,PROXY\n";
//! assert!(izanagi_kit::clashconf::detect(b));
//! let c = izanagi_kit::clashconf::Clash::parse(b).unwrap();
//! assert_eq!(c.rules, 3);
//! ```
const TOP_KEYS: &[&str] = &[
    "port:",
    "socks-port:",
    "redir-port:",
    "tproxy-port:",
    "mixed-port:",
    "allow-lan:",
    "bind-address:",
    "mode:",
    "log-level:",
    "ipv6:",
    "external-controller:",
    "external-ui:",
    "external-ui-url:",
    "secret:",
    "experimental:",
    "dns:",
    "hosts:",
    "proxies:",
    "proxy-groups:",
    "proxy-providers:",
    "rule-providers:",
    "rules:",
    "tunnels:",
    "tun:",
    "interface-name:",
    "routing-mark:",
    "geodata-mode:",
    "geodata-loader:",
    "geo-auto-update:",
    "geo-update-interval:",
    "geox-url:",
    "mmdb:",
    "cfw-bypass:",
    "cfw-latency-timeout:",
    "cfw-conn-break-strategy:",
    "profile:",
    "authentication:",
    "skip-auth-prefixes:",
    "lan-allowed-ips:",
    "lan-disallowed-ips:",
    "find-process-mode:",
    "keep-alive-interval:",
    "keep-alive-idle:",
    "disable-keep-alive:",
    "sniffer:",
    "global-client-fingerprint:",
    "global-ua:",
    "tcp-concurrent:",
    "unified-delay:",
    "geosite-matcher:",
    "fallback-filter:",
    "fake-ip-range:",
    "fake-ip-filter:",
    "default-nameserver:",
    "nameserver:",
    "nameserver-policy:",
    "fallback:",
    "proxy-server-nameserver:",
    "respect-rules:",
    "direct-nameserver:",
    "direct-nameserver-follow-policy:",
    "listeners:",
    "sub-rules:",
    "payload:",
    "behavior:",
    "path:",
    "interval:",
    "url:",
    "type:",
    "size:",
    "interval_seconds:",
    "lazy:",
    "filter:",
    "exclude-filter:",
    "include-all:",
    "include-all-proxies:",
    "include-all-providers:",
    "health-check:",
    "use:",
    "strategy:",
    "disable-udp:",
    "udp:",
    "icon:",
    "hidden:",
    "no-resolve:",
    "src:",
];
const RULE_HEADS: &[&str] = &[
    "DOMAIN",
    "DOMAIN-SUFFIX",
    "DOMAIN-KEYWORD",
    "DOMAIN-REGEX",
    "GEOSITE",
    "IP-CIDR",
    "IP-CIDR6",
    "IP-SUFFIX",
    "IP-ASN",
    "GEOIP",
    "SRC-IP-CIDR",
    "SRC-PORT",
    "DST-PORT",
    "SRC-IP-ASN",
    "PROCESS-NAME",
    "PROCESS-PATH",
    "PROCESS-NAME-REGEX",
    "PROCESS-PATH-REGEX",
    "NETWORK",
    "UID",
    "IN-TYPE",
    "IN-USER",
    "IN-NAME",
    "RULE-SET",
    "SCRIPT",
    "SUB-RULES",
    "MATCH",
    "AND",
    "OR",
    "NOT",
    "LOGIC-AND",
    "LOGIC-OR",
    "LOGIC-NOT",
];
const TYPE_VALUES: &[&str] = &[
    "ss",
    "ssr",
    "vmess",
    "vless",
    "trojan",
    "snell",
    "http",
    "socks5",
    "hysteria",
    "hysteria2",
    "tui\u{63}",
    "wireguard",
    "ssh",
    "mieru",
    "anytls",
    "direct",
    "reject",
    "select",
    "url-test",
    "fallback",
    "load-balance",
    "relay",
    "shadowsocks",
    "shadowtls",
];

/// Detect a Clash config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for k in TOP_KEYS {
        if t.contains(k) {
            hits += 1;
        }
    }
    let rule_lines = t
        .lines()
        .filter(|l| {
            let tr = l.trim_start_matches('-').trim_start();
            RULE_HEADS
                .iter()
                .any(|h| tr.starts_with(h) && tr.contains(','))
        })
        .count();
    (t.contains("proxies:") && hits >= 3) || hits >= 5 || rule_lines >= 2
}

/// Structural counts for a Clash config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clash {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `proxies:`/`proxy-groups:`/`rules:` etc top-level sections.
    pub sections: usize,
    /// `- RULE-TYPE,target,policy` rule lines.
    pub rules: usize,
    /// `type:` proxy/group type values.
    pub type_values: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Clash {
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
            rules: 0,
            type_values: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.ends_with(':')
                && !tr.starts_with('-')
                && !l.starts_with(' ')
                && !l.starts_with('\t')
            {
                c.sections += 1;
            } else if tr.starts_with('-')
                && RULE_HEADS
                    .iter()
                    .any(|h| tr.trim_start_matches('-').trim_start().starts_with(h))
                && tr.contains(',')
            {
                c.rules += 1;
            }
        }
        for k in TOP_KEYS {
            c.keys += t.matches(k).count();
        }
        for v in TYPE_VALUES {
            c.type_values += t.matches(&format!("type: {v}")).count();
            c.type_values += t.matches(&format!("type: \"{v}\"")).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"port: 7890\nsocks-port: 7891\nallow-lan: false\nmode: rule\nlog-level: info\nproxies:\n  - name: \"n1\"\n    type: vmess\n    server: example.com\nproxy-groups:\n  - name: PROXY\n    type: select\n    proxies: [\"n1\"]\nrules:\n  - DOMAIN-SUFFIX,google.com,PROXY\n  - IP-CIDR,10.0.0.0/8,DIRECT\n  - MATCH,PROXY\n";
        assert!(detect(b));
        let c = Clash::parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.rules, 3);
        assert_eq!(c.type_values, 2);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_yaml() {
        let b = b"foo: 1\nbar: 2\nbaz: 3\n";
        assert!(!detect(b));
        assert!(Clash::parse(b).is_none());
    }
}

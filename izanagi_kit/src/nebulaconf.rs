//! Nebula overlay-network `config.yml`.
//!
//! ```
//! let b = b"pki:\n  ca: /etc/nebula/ca.crt\n  cert: /etc/nebula/host.crt\n  key: /etc/nebula/host.key\nstatic_host_map:\n  \"192.168.100.1\": [\"1.2.3.4:4242\"]\nlighthouse:\n  am_lighthouse: false\n  serve_dns: false\n  interval: 60\n  hosts:\n    - \"192.168.100.1\"\nlisten:\n  host: 0.0.0.0\n  port: 4242\npunchy:\n  punch: true\n  respond: true\ntun:\n  dev: nebula1\n  mtu: 1300\nlogging:\n  level: info\n  format: text\nfirewall:\n  outbound:\n    - port: any\n      proto: any\n      host: any\n  inbound:\n    - port: any\n      proto: icmp\n      host: any\n";
//! assert!(izanagi_kit::nebulaconf::detect(b));
//! let c = izanagi_kit::nebulaconf::Nebula::parse(b).unwrap();
//! assert!(c.sections >= 7);
//! ```
const SECTION_KEYS: &[&str] = &[
    "pki:",
    "static_host_map:",
    "lighthouse:",
    "listen:",
    "punchy:",
    "relay:",
    "tun:",
    "logging:",
    "stats:",
    "handshakes:",
    "firewall:",
    "sshd:",
    "preferred_ranges:",
    "unsafe_routes:",
    "local_range:",
    "conntrack:",
];
const KEYS: &[&str] = &[
    "ca",
    "cert",
    "key",
    "blocklist",
    "disconnect_invalid",
    "am_lighthouse",
    "serve_dns",
    "dns",
    "interval",
    "hosts",
    "remote_allow_list",
    "local_allow_list",
    "advertise_addrs",
    "use_dns",
    "host",
    "port",
    "batch",
    "read_buffer",
    "write_buffer",
    "tcp_recv_timeout",
    "stream_mask",
    "punch",
    "respond",
    "delays",
    "am_relay",
    "use_relays",
    "relays",
    "dev",
    "drop_local_broadcast",
    "drop_multicast",
    "tx_queue",
    "mtu",
    "routes",
    "unsafe_routes",
    "level",
    "format",
    "disable_timestamp",
    "timestamp_format",
    "type",
    "listen",
    "namespace",
    "subsystem",
    "interval_seconds",
    "message_metrics",
    "lighthouse_metrics",
    "connect_timeout_ms",
    "wait_delay_ms",
    "group_wait_delay_ms",
    "try_interval_ms",
    "retrigger_wait_ms",
    "underlay",
    "outbound",
    "inbound",
    "proto",
    "code",
    "ca_name",
    "ca_fingerprint",
    "cidr",
    "groups",
    "group",
    "any",
    "fragment",
    "local_cidr",
    "logical_cidr",
    "private_key",
    "users",
    "auth",
    "allow",
    "deny",
    "defaults",
    "tcp_timeout",
    "udp_timeout",
    "default_timeout",
    "max_connections",
    "max_sockets",
    "rpm_zone_size",
    "rate_limit",
    "limit",
    "via",
    "route",
    "install",
    "metri\u{63}",
    "advertise_route",
    "use_own_ips",
];

/// Detect a Nebula config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut sec = 0usize;
    for l in t.lines() {
        let tr = l.trim_end();
        if !l.starts_with(' ') && !l.starts_with('\t') && SECTION_KEYS.contains(&tr) {
            sec += 1;
        }
    }
    sec >= 2
}

/// Structural counts for a Nebula config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nebula {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Top-level `name:` sections.
    pub sections: usize,
    /// Nested `name: value` and `- item` lines.
    pub entries: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Nebula {
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
            entries: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if SECTION_KEYS.contains(&l.trim_end())
                && !l.starts_with(' ')
                && !l.starts_with('\t')
            {
                c.sections += 1;
            } else if (tr.ends_with(':') && !tr.starts_with('-'))
                || tr.contains(':')
                || tr.starts_with('-')
            {
                c.entries += 1;
            }
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
        let b = b"pki:\n  ca: /etc/nebula/ca.crt\n  cert: /etc/nebula/host.crt\n  key: /etc/nebula/host.key\nlighthouse:\n  am_lighthouse: false\n  hosts:\n    - \"192.168.100.1\"\nlisten:\n  host: 0.0.0.0\n  port: 4242\ntun:\n  dev: nebula1\n  mtu: 1300\nfirewall:\n  outbound:\n    - port: any\n";
        assert!(detect(b));
        let c = Nebula::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert!(c.entries >= 12);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_yaml() {
        let b = b"foo: 1\nbar: 2\n";
        assert!(!detect(b));
        assert!(Nebula::parse(b).is_none());
    }
}

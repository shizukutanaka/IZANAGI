//! Sipwise `rtpengine.conf` (and `ngcp-rtpengine-daemon` config) census.
//!
//! INI: `[rtpengine]` (required section) plus optional `[rtpengine-testing]`,
//! `[control]`, `[interfaces]`, `[mqtt]`, `[homer]`, `[monitur]` /
//! `[exporter]` sections; `key = value` rtpengine directives
//! (`interface`, `listen-ng`, `port-min`, `port-max`, `redis`,
//! `recording-dir`, `kamailio` …).
//!
//! ```rust
//! let r = b"[rtpengine]\ninterface = 10.0.0.1\nlisten-ng = 127.0.0.1:22222\nport-min = 30000\nport-max = 40000\nredis = 127.0.0.1:6379/5\n";
//! assert!(izanagi_kit::rtpengine::detect(r));
//! let c = izanagi_kit::rtpengine::Rtpengine::parse(r).unwrap();
//! assert_eq!(c.settings, 5);
//! ```

/// rtpengine.conf census.
#[derive(Debug, Clone)]
pub struct Rtpengine {
    /// `key = value` lines matching a known rtpengine directive.
    pub settings: usize,
    /// `[section]` headers matching known section names.
    pub sections: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Section names used by rtpengine configs.
const SECTIONS: &[&str] = &[
    "rtpengine",
    "rtpengine-testing",
    "control",
    "exporter",
    "homer",
    "interfaces",
    "monitur",
    "mqtt",
];

/// rtpengine directives (key before `=`).
const KEYS: &[&str] = &[
    "active-switchover",
    "allowed-signalling",
    "b2b-url",
    "cli",
    "coderecs",
    "control-pmtu",
    "control-tos",
    "cpu-affinity",
    "delete-delay",
    "dtls",
    "dtls-cert-key",
    "dtls-cert-passwd",
    "dtls-ciphers",
    "dtls-mtu",
    "dtls-passive",
    "endpoint-learning",
    "export-dir",
    "fallback-codecs",
    "foreground",
    "graphite",
    "graphite-interval",
    "graphite-prefix",
    "homer",
    "homer-protocol",
    "homer-id",
    "http",
    "ice",
    "interface",
    "iptables-chain",
    "iptables-table",
    "kamailio",
    "listen-cli",
    "listen-http",
    "listen-ng",
    "listen-tcp",
    "listen-udp",
    "listen-tcp-ng",
    "log-facility",
    "log-facility-cdr",
    "log-facility-mqtt",
    "log-facility-rtcp",
    "log-level",
    "log-mark-prefix",
    "log-mark-suffix",
    "log-sessions",
    "log-subsystems",
    "max-sessions",
    "media-dir",
    "media-encryption",
    "min-port",
    "mqtt-auth-file",
    "mqtt-cafile",
    "mqtt-certfile",
    "mqtt-connect-timeout",
    "mqtt-id",
    "mqtt-keyfile",
    "mqtt-keepalive",
    "mqtt-port",
    "mqtt-publish-interval",
    "mqtt-publish-qos",
    "mqtt-publish-scope",
    "mqtt-publish-topic",
    "mqtt-user",
    "mqtt-password",
    "num-threads",
    "offer-timeout",
    "pidfile",
    "port-max",
    "port-min",
    "recording-dir",
    "recording-format",
    "recording-method",
    "redis",
    "redis-auth",
    "redis-cmd-timeout",
    "redis-connect-timeout",
    "redis-db",
    "redis-delete",
    "redis-disabled",
    "redis-expires",
    "redis-threads",
    "redis-write",
    "redis-subscribed-keyspaces",
    "save-flag-register",
    "silent-timeout",
    "spool-dir",
    "stun",
    "stun-user",
    "table",
    "timeout",
    "tos",
    "xmlrpc-format",
];

fn section(t: &str) -> Option<&str> {
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim())
}

fn assign_key(l: &str) -> Option<&str> {
    let t = l.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with(';') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

/// Detect an `rtpengine.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut keys = 0usize;
    let mut secs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if let Some(s) = section(tr) {
            if SECTIONS.contains(&s) {
                secs += 1;
            }
            continue;
        }
        if let Some(k) = assign_key(l) {
            if KEYS.contains(&k) {
                keys += 1;
            }
        }
    }
    keys >= 3 || (keys >= 1 && secs >= 1)
}

impl Rtpengine {
    /// Count directives and sections. Returns `None` when the input does
    /// not look like an `rtpengine.conf`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            sections: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if let Some(s) = section(tr) {
                if SECTIONS.contains(&s) {
                    c.sections += 1;
                }
                continue;
            }
            if let Some(k) = assign_key(l) {
                if KEYS.contains(&k) {
                    c.settings += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"# rtpengine\n[rtpengine]\ninterface = 10.0.0.1\nlisten-ng = 127.0.0.1:22222\nlisten-cli = 9900\nport-min = 30000\nport-max = 40000\nredis = 127.0.0.1:6379/5\nredis-db = 5\ntable = 0\nlog-level = 6\nrecording-dir = /var/spool/rtpengine\nrecording-method = proc\n";
        assert!(detect(b));
        let c = Rtpengine::parse(b).unwrap();
        assert_eq!(c.settings, 11);
        assert_eq!(c.sections, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[database]\nhost = x\n"));
        assert!(!detect(b"foo=1\nbar=2\n"));
        assert!(Rtpengine::parse(b"").is_none());
    }
}

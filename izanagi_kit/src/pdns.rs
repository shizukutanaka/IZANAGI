//! PowerDNS Authoritative `pdns.conf` census.
//!
//! pdns.conf is `key=value` lines (`#` comments): `local-address`,
//! `local-ipv6`, `launch`, `setuid`, `setgid`, `allow-axfr-ips`,
//! `allow-notify-from`, `api`, `api-key`, `webserver`,
//! `axfr-lower-serial`, `cache-ttl`, `daemon`, `default-ttl`,
//! `distributor-threads`, `edns-subnet-processing`, `entropy-source`,
//! `guardian`, `master`/`slave`, `negquery-cache-ttl`,
//! `query-cache-ttl`, `receiver-threads`, `reuseport`, `server-id`,
//! `slave-cycle-interval`, `soa-expire-default`, `soa-minimum-ttl`,
//! `soa-refresh-default`, `soa-retry-default`, `socket-dir`,
//! `version-string`, `xfr-max-received-mbytes`, `tcp-idle-timeout`,
//! `udp-truncation-threshold`, `signing-threads`, `lua-records-exec-limit`.
//!
//! ```rust
//! let k = b"launch=gmysql\nlocal-address=0.0.0.0\nlocal-ipv6=::\nsetuid=pdns\nsetgid=pdns\napi=yes\nwebserver=yes\nsocket-dir=/var/run\nversion-string=powerdns\n";
//! assert!(izanagi_kit::pdns::detect(k));
//! ```

/// PowerDNS config census.
#[derive(Debug, Clone)]
pub struct Pdns {
    /// `key=value` assignments.
    pub settings: usize,
    /// recognised keys present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "axfr-lower-serial",
    "slave-cycle-interval",
    "distributor-threads",
    "negquery-cache-ttl",
    "soa-expire-default",
    "soa-minimum-ttl",
    "soa-refresh-default",
    "soa-retry-default",
    "xfr-max-received-mbytes",
    "signing-threads",
    "lua-records-exec-limit",
    "entropy-source",
    "guardian",
    "receiver-threads",
    "reuseport",
    "server-id",
    "socket-dir",
    "version-string",
    "tcp-idle-timeout",
    "udp-truncation-threshold",
    "allow-axfr-ips",
    "allow-notify-from",
    "allow-dnsupdate-from",
    "allow-unsigned-autoprimary",
    "allow-unsigned-notify",
    "allow-unsigned-supermaster",
    "also-notify",
    "edns-subnet-processing",
    "forward-dnsupdate",
    "forward-notify",
    "lua-dnsupdate-policy-script",
    "dnsupdate",
    "only-notify",
    "outgoing-axfr-expand-alias",
    "overload-queue-length",
    "packetcache-ttl",
    "prevent-self-notification",
    "primary",
    "secondary",
    "resolver",
    "retrieval-threads",
    "secondary-do-renotify",
    "sender-threads",
    "superslave",
    "trusted-notification-proxy",
    "udp-answer-broadcast",
    "upgrade-unknown-types",
    "webserver-allow-from",
    "webserver-loglevel",
    "webserver-max-bodysize",
    "webserver-password",
    "write-pid",
    "xfr-cycle-interval",
];

const WEAK: &[&str] = &[
    "local-address",
    "local-ipv6",
    "launch",
    "setuid",
    "setgid",
    "api",
    "api-key",
    "api-logfile",
    "api-readonly",
    "webserver",
    "webserver-address",
    "webserver-port",
    "daemon",
    "default-ttl",
    "default-api-readonly",
    "cache-ttl",
    "query-cache-ttl",
    "master",
    "slave",
    "local-port",
    "module-dir",
    "include-dir",
    "carbon-ourname",
    "carbon-server",
    "carbon-interval",
    "carbon-namespace",
    "chroot",
    "max-queue-length",
    "out-of-zone-additional-processing",
    "do-ipv6-additional-processing",
    "8bit-dns",
    "axfr-fetch-timeout",
    "expand-alias",
    "load-modules",
    "max-tcp-connection-duration",
    "max-tcp-connections",
    "max-tcp-connections-per-client",
    "max-tcp-transactions-per-conn",
    "no-shuffle",
    "queue-limit",
    "rng",
    "security-poll-suffix",
    "tcp-fast-open",
    "tcp-control-max-queue-length",
    "tracing",
    "proxy-protocol-from",
];

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with('[') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"');
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a PowerDNS `pdns.conf`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `soa-*-default`/`axfr-lower-serial`/`slave-cycle-interval`/
    // `negquery-cache-ttl` are PowerDNS-exclusive keys.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Pdns {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = assign_key(line) {
                c.settings += 1;
                if STRONG.contains(&k) || WEAK.contains(&k) {
                    c.keys += 1;
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
        let b = b"launch=gmysql\nlocal-address=0.0.0.0\nlocal-ipv6=::\nsetuid=pdns\nsetgid=pdns\napi=yes\nwebserver=yes\nsocket-dir=/var/run\nversion-string=powerdns\n";
        assert!(detect(b));
        let c = Pdns::parse(b).unwrap();
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"local-address=0.0.0.0\nsetuid=x\n"));
        assert!(!detect(
            b"# launch=gmysql\n# axfr-lower-serial=yes\nlocal-address=x\n"
        ));
    }
}

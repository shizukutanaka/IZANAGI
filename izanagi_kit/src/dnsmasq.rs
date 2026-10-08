//! dnsmasq `dnsmasq.conf` configuration format.
//!
//! dnsmasq.conf uses bare directives like `domain-needed`, `bogus-priv`,
//! `no-resolv`, `no-poll`, plus `key=value` settings like
//! `server=/localnet/192.168.0.1`, `address=/double-click.net/127.0.0.1`,
//! `dhcp-range=`, `dhcp-host=`, `interface=`, `listen-address=`,
//! `conf-file=`, `conf-dir=`, `pxe-service=`, `cname=`.
//!
//! ```
//! let b = concat!(
//!     "domain-needed\n",
//!     "bogus-priv\n",
//!     "no-resolv\n",
//!     "server=/localnet/192 0 2 1\n",
//!     "address=/double-click.net/127 0 0 1\n",
//!     "dhcp-range=192 168 0 50,192 168 0 150,12h\n",
//!     "dhcp-host=11 22 33 44 55 66,fred\n"
//! ).as_bytes();
//! assert!(izanagi_kit::dnsmasq::detect(b));
//! let c = izanagi_kit::dnsmasq::Dnsmasq::parse(b).unwrap();
//! assert_eq!(c.flags, 3);
//! assert_eq!(c.key_values, 4);
//! ```

/// Parsed dnsmasq.conf summary.
#[derive(Debug, Clone)]
pub struct Dnsmasq {
    /// Bare flag directives (no `=`).
    pub flags: usize,
    /// `key=value` settings.
    pub key_values: usize,
    /// `address=`, `server=`, `dhcp-range=`, `dhcp-host=`, `dhcp-option=`, `dhcp-option-force=`, `dhcp-boot=`, `dhcp-name-match=`, `dhcp-ignore-names=`, `dhcp-generate-names=`, `dhcp-userclass=`, `dhcp-vendorclass=`, `dhcp-relay=`, `dhcp-luascript=`, `dhcp-script=`, `dhcp-rapid-commit=`, `dhcp-alternate-port=`, `dhcp-fqdn=`, `dhcp-proxy=`, `dhcp-match=`, `dhcp-broadcast=`, `dhcp-ignore=`, `dhcp-torrent=`, `bootp-dynamic=`, `enable-ra=`, `ra-param=`, `slaac=`, `dhcp-hostsfile=`, `dhcp-optsfile=`, `dhcp-hostsdir=`, `dhcp-optsdir=`, `read-ethers=`, `dhcp-scriptuser=`, `tag-if=`, `proxy-dnssec=`, `dnssec=`, `trust-anchor=`, `dnssec-check-unsigned=`, `dnssec-no-timecheck=`, `dnssec-timestamp=`, `dns-forward-max=`, `edns-packet-max=`, `dns-loop-detect=`, `stop-dns-rebind=`, `rebind-private-ok=`, `rebind-localhost-ok=`, `all-servers=`, `dnssec-debug=`, `auth-zone=`, `auth-server=`, `auth-soa=`, `auth-ttl=`, `ipset=`, `connmark=`, `host-record=`, `mx-host=`, `srv-host=`, `ptr-record=`, `txt-record=`, `naptr-record=`, `caa-record=`, `dns-rr=`, `cname=`, `interface=`, `interface-name=`, `bridge-interface=`, `shared-interface=`, `except-interface=`, `auth-interface=`, `no-dhcp-interface=`, `listen-address=`, `bind-interfaces=`, `bind-dynamic=`, `local-service=`, `query-port=`, `min-port=`, `max-port=`, `local=`, `domain=`, `expand-hosts=`, `bogus-priv=`, `domain-needed=`, `localise-queries=`, `selfmx=`, `localmx=`, `mx-target=`, `dumpfile=`, `dumpmask=`, `cache-size=`, `no-negcache=`, `neg-ttl=`, `max-ttl=`, `max-cache-ttl=`, `min-cache-ttl=`, `bogus-nxdomain=`, `alias=`, `filter-aaaa=`, `filter-a=`, `filter-rr=`, `filterwin2k=`, `resolv-file=`, `strict-order=`, `all-servers`, `servers-file=`, `no-hosts=`, `addn-hosts=`, `hostsdir=`, `localnet=`, `expand-hosts`, `domain=`, `cname`, `conf-file=`, `conf-dir=`, `conf-script=`, `user=`, `group=`, `facility=`, `log-facility=`, `log-debug=`, `log-async=`, `log-queries=`, `log-dhcp=`, `log-extra=`, `quiet-dhcp=`, `quiet-dhcp6=`, `quiet-ra=`, `pid-file=`, `enable-dbus=`, `enable-ubus=`, `bootp-dynamic`, `enable-tftp=`, `tftp-root=`, `tftp-no-fail=`, `tftp-secure=`, `tftp-lowercase=`, `tftp-max=`, `tftp-mtu=`, `tftp-no-blocksize=`, `tftp-range=`, `tftp-port-range=`, `tftp-single-port=`, `tftp-bs=`, `tftp-unique-root=`, `pxe-service=`, `pxe-prompt=`, `enable-proxy=`, `dns-proxy=`, `proxy-dnssec`, `dnssec`, `dumpfile`, `dns-loop-detect`, `stop-dns-rebind`, `rebind-private-ok`, `rebind-localhost-ok`, `dnssec-check-unsigned`, `dnssec-no-timecheck`, `dnssec-timestamp`, `dnssec-debug`, `proxy-dnssec`, `dns-forward-max`, `edns-packet-max`, `servers-file`, `no-resolv`, `strict-order`, `all-servers`, `srv-host`, `ptr-record`, `txt-record`, `naptr-record`, `caa-record`, `dns-rr`, `host-record`, `ipset`, `connmark`, `mx-host`, `cname`, `local`, `srv-host`, `interface`, `interface-name`, `bridge-interface`, `shared-interface`, `except-interface`, `auth-interface`, `no-dhcp-interface`, `listen-address`, `bind-interfaces`, `bind-dynamic`, `local-service`, `query-port`, `min-port`, `max-port` keys.
    pub dhcp: usize,
    /// `server=`/`rev-server=`/`local=` upstream server declarations.
    pub servers: usize,
    /// `address=`/`host-record=`/`txt-record=`/`srv-host=`/`mx-host=`/`cname=`/`ptr-record=`/`naptr-record=`/`caa-record=`/`dns-rr=` local RR declarations.
    pub records: usize,
    /// `dhcp-*` declarations.
    pub dhcp_options: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const FLAGS: &[&str] = &[
    "domain-needed",
    "bogus-priv",
    "no-resolv",
    "no-poll",
    "no-hosts",
    "expand-hosts",
    "localise-queries",
    "selfmx",
    "localmx",
    "stop-dns-rebind",
    "rebind-private-ok",
    "rebind-localhost-ok",
    "dns-loop-detect",
    "bind-interfaces",
    "bind-dynamic",
    "local-service",
    "auth-server",
    "proxy-dnssec",
    "dnssec",
    "dnssec-check-unsigned",
    "dnssec-no-timecheck",
    "dnssec-timestamp",
    "dnssec-debug",
    "bogus-nxdomain",
    "filter-aaaa",
    "filter-a",
    "filter-rr",
    "filterwin2k",
    "strict-order",
    "all-servers",
    "clear-on-reload",
    "enable-ra",
    "slaac",
    "dhcp-rapid-commit",
    "dhcp-alternate-port",
    "dhcp-fqdn",
    "dhcp-proxy",
    "dhcp-broadcast",
    "dhcp-torrent",
    "bootp-dynamic",
    "read-ethers",
    "quiet-dhcp",
    "quiet-dhcp6",
    "quiet-ra",
    "log-queries",
    "log-dhcp",
    "log-extra",
    "log-async",
    "log-debug",
    "dumpfile",
    "no-negcache",
    "enable-dbus",
    "enable-ubus",
    "tftp-no-fail",
    "tftp-secure",
    "tftp-lowercase",
    "tftp-no-blocksize",
    "tftp-single-port",
    "tftp-unique-root",
    "enable-proxy",
    "dns-proxy",
    "conntrack",
    "queryport",
    "cache-rr",
    "use-stale-cache",
    "no-roundrobin",
    "localise-queries",
    "no-ident",
    "skip-inotify",
];
const KV_KEYS: &[&str] = &[
    "server",
    "rev-server",
    "local",
    "address",
    "ipset",
    "connmark",
    "host-record",
    "mx-host",
    "srv-host",
    "ptr-record",
    "txt-record",
    "naptr-record",
    "caa-record",
    "dns-rr",
    "cname",
    "mx-target",
    "domain",
    "localnet",
    "interface",
    "interface-name",
    "bridge-interface",
    "shared-interface",
    "except-interface",
    "auth-interface",
    "no-dhcp-interface",
    "listen-address",
    "query-port",
    "min-port",
    "max-port",
    "cache-size",
    "neg-ttl",
    "max-ttl",
    "max-cache-ttl",
    "min-cache-ttl",
    "resolv-file",
    "servers-file",
    "addn-hosts",
    "hostsdir",
    "conf-file",
    "conf-dir",
    "conf-script",
    "user",
    "group",
    "facility",
    "log-facility",
    "pid-file",
    "dumpfile",
    "dumpmask",
    "alias",
    "bogus-nxdomain",
    "auth-zone",
    "auth-soa",
    "auth-ttl",
    "dns-forward-max",
    "edns-packet-max",
    "dhcp-range",
    "dhcp-host",
    "dhcp-option",
    "dhcp-option-force",
    "dhcp-boot",
    "dhcp-name-match",
    "dhcp-ignore-names",
    "dhcp-generate-names",
    "dhcp-userclass",
    "dhcp-vendorclass",
    "dhcp-relay",
    "dhcp-luascript",
    "dhcp-script",
    "dhcp-match",
    "dhcp-ignore",
    "dhcp-hostsfile",
    "dhcp-optsfile",
    "dhcp-hostsdir",
    "dhcp-optsdir",
    "dhcp-scriptuser",
    "tag-if",
    "enable-tftp",
    "tftp-root",
    "tftp-max",
    "tftp-mtu",
    "tftp-range",
    "tftp-port-range",
    "tftp-bs",
    "pxe-service",
    "pxe-prompt",
    "ra-param",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like dnsmasq.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut score = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        let head = tr.split('=').next().unwrap_or("");
        let head = head.trim_start_matches("--");
        if FLAGS.contains(&head) || KV_KEYS.contains(&head) {
            score += 1;
        }
    }
    score >= 3
}

impl Dnsmasq {
    /// Parses a dnsmasq.conf summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            flags: 0,
            key_values: 0,
            dhcp: 0,
            servers: 0,
            records: 0,
            dhcp_options: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            let body = tr.trim_start_matches("--");
            let (key, has_eq) = match body.split_once('=') {
                Some((k, _)) => (k.trim(), true),
                None => (body.trim(), false),
            };
            if !has_eq {
                if FLAGS.contains(&key) {
                    c.flags += 1;
                }
                continue;
            }
            if !KV_KEYS.contains(&key) {
                continue;
            }
            c.key_values += 1;
            match key {
                "server" | "rev-server" | "local" => c.servers += 1,
                "address" | "host-record" | "txt-record" | "srv-host" | "mx-host" | "cname"
                | "ptr-record" | "naptr-record" | "caa-record" | "dns-rr" => c.records += 1,
                _ if key.starts_with("dhcp-") => c.dhcp += 1,
                _ => {}
            }
            if key.starts_with("dhcp-option") {
                c.dhcp_options += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dnsmasq() {
        let b = concat!(
            "domain-needed\n",
            "bogus-priv\n",
            "no-resolv\n",
            "server=/localnet/192 0 2 1\n",
            "address=/double-click.net/127 0 0 1\n",
            "dhcp-range=192 168 0 50,192 168 0 150,12h\n",
            "dhcp-host=11 22 33 44 55 66,fred\n",
            "interface=eth0\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Dnsmasq::parse(b).unwrap();
        assert_eq!(c.flags, 3);
        assert_eq!(c.key_values, 5);
        assert_eq!(c.servers, 1);
        assert_eq!(c.records, 1);
        assert_eq!(c.dhcp, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(Dnsmasq::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

//! dhcpcd `dhcpcd.conf` の検出・カウント。
//!
//! `directive value` / `key=value` フラット設定。`interface`/`profile` ブロック、
//! `static <opt>=<val>`、`option`/`nooption`、`denyinterfaces`/`allowinterfaces`、
//! `nohook`/`require`/`env`、`ipv4/ipv6` 系、`slaac`/`duid`/`clientid` 系を分類する。
//!
//! ```
//! let cfg = b"# dhcpcd.conf\n\
//!             hostname\n\
//!             clientid\n\
//!             noipv6rs\n\
//!             interface eth0\n\
//!               static ip_address=192.168.1.10/24\n\
//!               static routers=192.168.1.1\n\
//!               static domain_name_servers=192.168.1.1\n";
//! assert!(izanagi_kit::dhcpcdconf::detect(cfg));
//! let c = izanagi_kit::dhcpcdconf::parse(cfg).unwrap();
//! assert_eq!(c.static_opts, 3);
//! ```

/// インターフェース/プロファイル選択ディレクティブ。
const SCOPE_KEYS: &[&str] = &[
    "interface",
    "profile",
    "fallback",
    "arping",
    "ssid",
    "denyinterfaces",
    "allowinterfaces",
    "denyraw",
    "whitelist",
    "blacklist",
    "controlgroup",
];

/// `static <opt>=<val>` 内で既知のオプション名。
const STATIC_OPTS: &[&str] = &[
    "ip_address",
    "ip6_address",
    "routers",
    "domain_name_servers",
    "domain_name",
    "domain_search",
    "ntp_servers",
    "sip_servers",
    "nis_servers",
    "ntp_servers",
    "netbios_name_servers",
    "netbios_node_type",
    "interface_mtu",
];

/// DHCP オプション系ディレクティブ。
const DHCP_KEYS: &[&str] = &[
    "option",
    "nooption",
    "require",
    "nohook",
    "env",
    "vendclass",
    "vendorclassid",
    "vendor",
    "userclass",
    "clientid",
    "duid",
    "iaid",
    "rapid_commit",
    "hostname",
    "fqdn",
    "v6only",
    "ipv4only",
    "dhcp",
    "dhcp6",
    "reboot",
    "timeout",
    "waitip",
    "release",
    "expire",
    "informed",
    "inform",
    "inform6",
    "request",
    "lladdr",
];

/// IPv4/IPv6 系ディレクティブ。
const IPV_KEYS: &[&str] = &[
    "ipv4",
    "ipv6",
    "ipv6rs",
    "noipv4ll",
    "noipv6rs",
    "noipv6",
    "ipv6ra_fork",
    "ipv6ra_own",
    "ipv6ra_own_default",
    "ipv6ra_autoconf",
    "ipv6ra_noautoconf",
    "ipv6ra_mtu",
    "ipv6ra_low",
    "ipv6ra_high",
    "ia_na",
    "ia_ta",
    "ia_pd",
    "slaac",
    "private",
    "temporary",
    "noprefix",
    "autoconf",
    "ipv6addr",
    "alias_id",
    "dadtransmits",
    "nodelay",
    "noconfig",
    "nogateway",
    "nodev",
    "nosyslog",
    "logfile",
    "pidfile",
    "waitip4",
    "waitip6",
    "script",
    "runscript",
    "onscript",
    "staticdomainname",
];

/// メトリクス/ルート系ディレクティブ。
const ROUTE_KEYS: &[&str] = &[
    "metric",
    "defaultroute",
    "nodefaultroute",
    "nogateway",
    "staticroute",
    "msstaticroutes",
    "classless_static_routes",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 総ディレクティブ行数。
    pub entries: usize,
    /// `static <opt>=<val>` 数。
    pub static_opts: usize,
    /// `interface`/`profile`/`ssid`/`allowinterfaces`/`denyinterfaces` 等選択数。
    pub scope: usize,
    /// `option`/`nooption`/`require`/`env`/`vend*`/`clientid`/`duid`/`hostname` 等 DHCP 系数。
    pub dhcp: usize,
    /// `ipv*`/`slaac`/`ia_*`/`nogateway`/`script`/`waitip`/`timeout`/`reboot` 等 IPv4/IPv6・制御数。
    pub ipv: usize,
    /// `metric`/`defaultroute`/`classless_static_routes` 等ルート数。
    pub route: usize,
    /// `nohook`/`persistent`/`nodelay` 等フラグ数。
    pub flags: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が `dhcpcd.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を `dhcpcd.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        static_opts: 0,
        scope: 0,
        dhcp: 0,
        ipv: 0,
        route: 0,
        flags: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if let Some(rest) = line.strip_prefix("static ") {
            let opt = rest.split('=').next().unwrap_or("").trim();
            if STATIC_OPTS.contains(&opt) || rest.contains('=') {
                c.static_opts += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let head = line.split([' ', '\t', '=']).next().unwrap_or("");
        if SCOPE_KEYS.contains(&head) {
            c.scope += 1;
            known += 1;
        } else if DHCP_KEYS.contains(&head) {
            c.dhcp += 1;
            known += 1;
        } else if IPV_KEYS.contains(&head) {
            c.ipv += 1;
            known += 1;
        } else if ROUTE_KEYS.contains(&head) {
            c.route += 1;
            known += 1;
        } else if matches!(
            head,
            "persistent"
                | "nodelay"
                | "lastlease"
                | "lastlease6"
                | "local"
                | "background"
                | "dumplease"
                | "link_status"
                | "printpidfile"
                | "timeout_zero"
        ) {
            c.flags += 1;
            known += 1;
        } else {
            c.misc += 1;
        }
    }
    if known + c.static_opts >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# A sample configuration for dhcpcd.\n\
        hostname\n\
        clientid\n\
        persistent\n\
        option rapid_commit\n\
        option domain_name_servers, domain_name, domain_search, host_name\n\
        option classless_static_routes\n\
        option interface_mtu\n\
        require dhcp_server_identifier\n\
        slaac private\n\
        noipv6rs\n\
        \n\
        denyinterfaces eth1\n\
        \n\
        interface eth0\n\
          static ip_address=192.168.1.10/24\n\
          static routers=192.168.1.1\n\
          static domain_name_servers=8.8.8.8\n\
          noipv6rs\n\
          fallback nodhcp\n\
        \n\
        profile nodhcp\n\
          static ip_address=10.0.0.5/24\n\
          metric 200\n";

    #[test]
    fn detects_dhcpcd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 20);
        assert_eq!(c.static_opts, 4);
        assert_eq!(c.scope, 4);
        assert_eq!(c.dhcp, 7);
        assert_eq!(c.ipv, 3);
        assert_eq!(c.route, 1);
        assert_eq!(c.flags, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"option routers 1.2.3.4\n"));
    }
}

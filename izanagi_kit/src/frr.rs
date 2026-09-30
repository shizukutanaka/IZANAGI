//! FRRouting configuration census.
//!
//! FRR config files (`frr.conf` or per-daemon files) are Cisco-IOS-like:
//! `!` comment separators, `frr version`/`frr defaults`/`hostname` global
//! lines, `interface <name>` blocks, `router <proto>` blocks (ospf/bgp/
//! rip/isis/…), indented `ip`/`ipv6` commands, `neighbor`/`network`
//! statements, `access-list`/`prefix-list`/`route-map`/`community-list`
//! filters, `line vty`/`bfd`/`mpls`/`vrf` sections and a closing `end`.
//! `parse` counts each class.
//!
//! ```rust
//! let f = concat!(
//!     "!\n",
//!     "frr version 9\n",
//!     "hostname r1\n",
//!     "!\n",
//!     "interface eth0\n",
//!     " ip address 10.0.0.1/24\n",
//!     "!\n",
//!     "router ospf\n",
//!     " network 10.0.0.0/24 area 0\n",
//!     "!\n",
//!     "router bgp 65001\n",
//!     " neighbor 10.0.0.2 remote-as 65002\n",
//!     "!\n",
//!     "end\n",
//! );
//! let c = izanagi_kit::frr::Frr::parse(f.as_bytes()).unwrap();
//! assert_eq!(c.interfaces, 1);
//! assert_eq!(c.routers, 2);
//! ```

const PROTO: &[&str] = &[
    "ospf",
    "ospf6",
    "bgp",
    "rip",
    "ripng",
    "isis",
    "eigrp",
    "pim",
    "pim6",
    "ldp",
    "nhrp",
    "babel",
    "openfabric",
    "pathd",
    "vrrp",
    "msdp",
    "pbr",
    "sr-te",
    "segment-routing",
    "mlag",
];

/// FRR configuration census.
#[derive(Debug, Clone)]
pub struct Frr {
    /// `!` separator/comment lines.
    pub bangs: usize,
    /// `frr version`/`frr defaults`/`hostname`/`log `/`password`/`enable password`/`service ` lines.
    pub globals: usize,
    /// `interface <name>` block headers.
    pub interfaces: usize,
    /// `router <proto>`/`ip vrf`/`vrf` block headers.
    pub routers: usize,
    /// `ip `/`ipv6 ` command lines (address/route/forwarding/nht/…).
    pub ip_cmds: usize,
    /// `neighbor `/`peer-group` statements.
    pub neighbors: usize,
    /// `network `/`area `/`redistribute` statements.
    pub networks: usize,
    /// `access-list`/`prefix-list`/`route-map`/`community-list`/`bgp ` lines.
    pub filters: usize,
    /// `address-family`/`line `/`bfd`/`mpls`/`end`/`exit` lines.
    pub extras: usize,
}

/// Whether the buffer looks like an FRR configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("frr version")
        || (t.contains("!")
            && t.contains("interface ")
            && (t.contains("router ospf")
                || t.contains("router bgp")
                || t.contains("router rip")
                || t.contains("hostname ")))
}

impl Frr {
    /// Parse an FRR configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            bangs: 0,
            globals: 0,
            interfaces: 0,
            routers: 0,
            ip_cmds: 0,
            neighbors: 0,
            networks: 0,
            filters: 0,
            extras: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s == "!" || s.starts_with("! ") {
                c.bangs += 1;
                continue;
            }
            if s.starts_with("frr ")
                || s.starts_with("hostname ")
                || s.starts_with("log ")
                || s.starts_with("password ")
                || s.starts_with("enable password")
                || s.starts_with("service ")
                || s.starts_with("username ")
                || s.starts_with("agentx")
                || s.starts_with("debug ")
            {
                c.globals += 1;
                continue;
            }
            if s.starts_with("interface ") {
                c.interfaces += 1;
                continue;
            }
            if let Some(rest) = s.strip_prefix("router ") {
                let head = rest.split([' ', '\t']).next().unwrap_or("");
                if PROTO.contains(&head)
                    || rest
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_alphabetic())
                {
                    c.routers += 1;
                }
                continue;
            }
            if s.starts_with("vrf ") || s.starts_with("ip vrf") || s.starts_with("ipv6 vrf") {
                c.routers += 1;
                continue;
            }
            if (s.starts_with("ip ") && !s.starts_with("ip prefix-list"))
                || (s.starts_with("ipv6 ") && !s.starts_with("ipv6 prefix-list"))
            {
                c.ip_cmds += 1;
                continue;
            }
            if s.starts_with("neighbor ") || s.contains(" peer-group") {
                c.neighbors += 1;
                continue;
            }
            if s.starts_with("network ")
                || s.starts_with("area ")
                || s.starts_with("redistribute")
                || s.starts_with("default-information")
            {
                c.networks += 1;
                continue;
            }
            if s.starts_with("access-list")
                || s.starts_with("ip prefix-list")
                || s.starts_with("ipv6 prefix-list")
                || s.starts_with("route-map")
                || s.starts_with("bgp ")
                || s.starts_with("community-list")
                || s.starts_with("extcommunity-list")
                || s.starts_with("as-path")
                || s.starts_with("match ")
                || s.starts_with("set ")
                || s.starts_with("call ")
            {
                c.filters += 1;
                continue;
            }
            if s.starts_with("address-family")
                || s.starts_with("line ")
                || s.starts_with("bfd")
                || s.starts_with("mpls")
                || s == "end"
                || s.starts_with("exit")
                || s.starts_with("no ")
                || s.starts_with("allow-ecmp")
                || s.starts_with("table ")
                || s.starts_with("segment-routing")
            {
                c.extras += 1;
                continue;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let b = concat!(
            "!\n",
            "frr version 9\n",
            "hostname r1\n",
            "log file /var/log/frr\n",
            "!\n",
            "interface eth0\n",
            " ip address 10.0.0.1/24\n",
            " ipv6 address fd00::1/64\n",
            "!\n",
            "router ospf\n",
            " network 10.0.0.0/24 area 0\n",
            " redistribute connected\n",
            "!\n",
            "router bgp 65001\n",
            " neighbor 10.0.0.2 remote-as 65002\n",
            " address-family ipv4 unicast\n",
            "  neighbor 10.0.0.2 activate\n",
            "!\n",
            "access-list 1 permit 10.0.0.0/8\n",
            "route-map RM permit 10\n",
            " match ip address 1\n",
            "!\n",
            "line vty\n",
            "!\n",
            "end\n",
        );
        let c = Frr::parse(b.as_bytes()).unwrap();
        assert_eq!(c.bangs, 7);
        assert_eq!(c.globals, 3);
        assert_eq!(c.interfaces, 1);
        assert_eq!(c.routers, 2);
        assert_eq!(c.ip_cmds, 2);
        assert_eq!(c.neighbors, 2);
        assert_eq!(c.networks, 2);
        assert_eq!(c.filters, 3);
        assert_eq!(c.extras, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Frr::parse(b"foo = 1").is_none());
    }
}

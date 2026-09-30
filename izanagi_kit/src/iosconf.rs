//! Cisco IOS running-config census.
//!
//! IOS configurations use `!` separators, indented sub-commands inside
//! `interface`/`router`/`line`/`vlan`/`ip vrf` blocks, `no ` negations and
//! `ip route`/`access-list`/`hostname` statements. `parse` counts blocks,
//! commands and each feature class.
//!
//! ```rust
//! let i = concat!(
//!     "hostname r1\n",
//!     "!\n",
//!     "interface GigabitEthernet0/0\n",
//!     " ip address 10 255\n",
//!     " no shutdown\n",
//!     "!\n",
//!     "router ospf 1\n",
//!     " network 10 area 0\n",
//!     "!\n",
//! );
//! let c = izanagi_kit::iosconf::Iosconf::parse(i.as_bytes()).unwrap();
//! assert_eq!(c.interfaces, 1);
//! assert_eq!(c.bangs, 3);
//! ```

/// IOS configuration census.
#[derive(Debug, Clone)]
pub struct Iosconf {
    /// `!` separator lines.
    pub bangs: usize,
    /// `interface ` block opens.
    pub interfaces: usize,
    /// `router `/`line `/`vlan `/`ip vrf`/`crypto `/`class-map`/`policy-map`/`route-map`/`router bgp` block opens.
    pub other_blocks: usize,
    /// `ip address` commands.
    pub ip_addresses: usize,
    /// `ip route` commands.
    pub routes: usize,
    /// `access-list`/`ip access-list` commands.
    pub acls: usize,
    /// `no ` negations.
    pub negations: usize,
    /// Other configuration lines.
    pub commands: usize,
    /// `hostname `, `enable`, `username`, `snmp-server`, `ntp`, `logging` lines.
    pub services: usize,
}

/// Whether the buffer looks like an IOS configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let bang = t.lines().any(|l| l.trim() == "!");
    (bang && (t.contains("interface ") || t.contains("hostname ")))
        || (t.contains("interface ") && t.contains(" ip address "))
        || (t.contains("access-list ") && t.contains("router "))
}

impl Iosconf {
    /// Parse an IOS configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            bangs: 0,
            interfaces: 0,
            other_blocks: 0,
            ip_addresses: 0,
            routes: 0,
            acls: 0,
            negations: 0,
            commands: 0,
            services: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s == "!" {
                c.bangs += 1;
                continue;
            }
            if s.starts_with("interface ") {
                c.interfaces += 1;
                continue;
            }
            if [
                "router ",
                "line ",
                "vlan ",
                "ip vrf",
                "crypto ",
                "class-map",
                "policy-map",
                "route-map",
                "key chain",
                "username ",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.other_blocks += 1;
                continue;
            }
            if s.starts_with("ip address ") {
                c.ip_addresses += 1;
                continue;
            }
            if s.starts_with("ip route ") || s.starts_with("ipv6 route ") {
                c.routes += 1;
                continue;
            }
            if s.starts_with("access-list ") || s.starts_with("ip access-list") {
                c.acls += 1;
                continue;
            }
            if s.starts_with("no ") {
                c.negations += 1;
                continue;
            }
            if [
                "hostname ",
                "enable ",
                "snmp-server",
                "ntp ",
                "logging ",
                "banner ",
                "domain-name",
                "aaa ",
                "ssh ",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.services += 1;
                continue;
            }
            c.commands += 1;
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
            "hostname r1\n",
            "enable secret x\n",
            "!\n",
            "interface GigabitEthernet0/0\n",
            " ip address 10 255\n",
            " no shutdown\n",
            "!\n",
            "interface GigabitEthernet0/1\n",
            " switchport mode access\n",
            "!\n",
            "router ospf 1\n",
            " network 10 area 0\n",
            "!\n",
            "ip route 0 0 10 1\n",
            "access-list 1 permit any\n",
            "line vty 0 4\n",
            " transport input ssh\n",
        );
        let c = Iosconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.bangs, 4);
        assert_eq!(c.interfaces, 2);
        assert_eq!(c.other_blocks, 2);
        assert_eq!(c.ip_addresses, 1);
        assert_eq!(c.routes, 1);
        assert_eq!(c.acls, 1);
        assert_eq!(c.negations, 1);
        assert_eq!(c.services, 2);
        assert_eq!(c.commands, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Iosconf::parse(b"hello").is_none());
    }
}

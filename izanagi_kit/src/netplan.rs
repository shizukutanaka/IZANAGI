//! Netplan YAML census.
//!
//! A netplan file has a `network:` root containing `version:`, `renderer:`,
//! and device groups (`ethernets:`/`wifis:`/`bridges:`/`bonds:`/`vlans:`/
//! `tunnels:`/`dummy-devices:`/`modems:`/`vrfs:`/`openvswitch:`) whose
//! indented child keys are device IDs. `parse` counts groups, devices, DHCP
//! flags, address lists and list items.
//!
//! ```rust
//! let n = concat!(
//!     "network:\n",
//!     "  version: 2\n",
//!     "  renderer: networkd\n",
//!     "  ethernets:\n",
//!     "    eth0:\n",
//!     "      dhcp4: true\n",
//!     "      addresses: [10/24]\n",
//!     "  bridges:\n",
//!     "    br0:\n",
//!     "      interfaces: [eth0]\n",
//! );
//! let c = izanagi_kit::netplan::Netplan::parse(n.as_bytes()).unwrap();
//! assert_eq!(c.device_groups, 2);
//! assert_eq!(c.devices, 2);
//! ```

const GROUPS: &[&str] = &[
    "ethernets",
    "wifis",
    "bridges",
    "bonds",
    "vlans",
    "tunnels",
    "dummy-devices",
    "modems",
    "vrfs",
    "openvswitch",
    "nm-devices",
];

/// Netplan YAML census.
#[derive(Debug, Clone)]
pub struct Netplan {
    /// `version:` value present (0 or 1).
    pub version: usize,
    /// `renderer:` value present (0 or 1).
    pub renderer: usize,
    /// Device-group keys present.
    pub device_groups: usize,
    /// Device IDs declared inside device groups.
    pub devices: usize,
    /// `dhcp4:`/`dhcp6:` keys.
    pub dhcps: usize,
    /// `addresses:`/`nameservers:`/`routes:`/`gateway4:`/`gateway6:`/`interfaces:`/`match:`/`link-local:`/`access-points:` keys.
    pub services: usize,
    /// `- ` list items inside the file.
    pub items: usize,
    /// `parameters:`/`optional:`/`mtu:`/`table:`/`accept-ra:`/`auth:`/`password:` keys.
    pub extras: usize,
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// Whether the buffer looks like a Netplan YAML file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let has_net = t.lines().any(|l| l.trim_end() == "network:");
    has_net && GROUPS.iter().any(|g| t.contains(&format!("{g}:")))
}

impl Netplan {
    /// Parse a Netplan YAML file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            version: 0,
            renderer: 0,
            device_groups: 0,
            devices: 0,
            dhcps: 0,
            services: 0,
            items: 0,
            extras: 0,
        };
        let mut in_group = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let ind = indent(l);
            if ind == 0 {
                in_group = false;
            }
            if ind <= 2 {
                let key = s.trim_end_matches(':').trim_matches('"').trim_matches('\'');
                let is_group = ind == 2 && GROUPS.contains(&key);
                if ind == 2 && !is_group {
                    in_group = false;
                }
                if is_group {
                    c.device_groups += 1;
                    in_group = true;
                }
                if ind == 2 && s.starts_with("version:") {
                    c.version += 1;
                }
                if ind == 2 && s.starts_with("renderer:") {
                    c.renderer += 1;
                }
                continue;
            }
            if in_group && ind == 4 {
                if s.starts_with("- ") {
                    c.items += 1;
                } else if s.ends_with(':') {
                    c.devices += 1;
                }
                continue;
            }
            if s.starts_with("dhcp4:") || s.starts_with("dhcp6:") {
                c.dhcps += 1;
            } else if [
                "addresses:",
                "nameservers:",
                "routes:",
                "gateway4:",
                "gateway6:",
                "interfaces:",
                "match:",
                "link-local:",
                "access-points:",
                "via:",
                "to:",
                "search:",
                "domains:",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.services += 1;
            } else if [
                "parameters:",
                "optional:",
                "mtu:",
                "table:",
                "accept-ra:",
                "auth:",
                "password:",
                "key-management:",
                "routes-metric:",
                "macaddress:",
                "wakeonlan:",
                "critical:",
                "netplan.io:",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.extras += 1;
            }
            if s.starts_with("- ") {
                c.items += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_netplan() {
        let b = concat!(
            "network:\n",
            "  version: 2\n",
            "  renderer: networkd\n",
            "  ethernets:\n",
            "    eth0:\n",
            "      dhcp4: true\n",
            "      addresses: [10/24]\n",
            "      gateway4: gw\n",
            "      nameservers:\n",
            "        addresses: [8]\n",
            "    eth1:\n",
            "      mtu: 1500\n",
            "  bridges:\n",
            "    br0:\n",
            "      interfaces: [eth0]\n",
            "      routes:\n",
            "        - to: net\n",
            "          via: gw\n",
        );
        let c = Netplan::parse(b.as_bytes()).unwrap();
        assert_eq!(c.version, 1);
        assert_eq!(c.renderer, 1);
        assert_eq!(c.device_groups, 2);
        assert_eq!(c.devices, 3);
        assert_eq!(c.dhcps, 1);
        assert_eq!(c.services, 7);
        assert_eq!(c.items, 1);
        assert_eq!(c.extras, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Netplan::parse(b"foo: bar").is_none());
    }
}

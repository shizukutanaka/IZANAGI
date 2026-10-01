//! `/etc/network/interfaces` (ifupdown) census.
//!
//! Debian ifupdown files use `auto <iface>` and `allow-hotplug <iface>`
//! stanzas, `iface <name> <family> <method>` blocks and indented option
//! lines (`address`, `netmask`, `gateway`, `dns-*`, `pre-up`/`up`/`down`/
//! `post-down`, `hwaddress`, `bridge-*`, …). `parse` counts stanzas,
//! methods and option classes.
//!
//! ```rust
//! let i = concat!(
//!     "auto lo\n",
//!     "iface lo inet loopback\n",
//!     "auto eth0\n",
//!     "iface eth0 inet static\n",
//!     "    address 10\n",
//!     "    netmask 255\n",
//!     "    gateway gw\n",
//!     "    dns-nameservers 8\n",
//! );
//! let c = izanagi_kit::interfaces::Interfaces::parse(i.as_bytes()).unwrap();
//! assert_eq!(c.ifaces, 2);
//! assert_eq!(c.autos, 2);
//! ```

/// ifupdown interfaces census.
#[derive(Debug, Clone)]
pub struct Interfaces {
    /// `auto ` lines.
    pub autos: usize,
    /// `allow-hotplug`/`allow-auto`/`no-auto-down` lines.
    pub allows: usize,
    /// `iface ` stanza headers.
    pub ifaces: usize,
    /// `static`/`dhcp`/`loopback`/`manual`/`ppp` methods.
    pub methods: usize,
    /// `address`/`netmask`/`gateway`/`broadcast`/`network` options.
    pub addressing: usize,
    /// `dns-*` options.
    pub dns: usize,
    /// `pre-up`/`up`/`post-up`/`pre-down`/`down`/`post-down` hooks.
    pub hooks: usize,
    /// `hwaddress`/`bridge-*`/`bond-*`/`vlan-*`/`wireless-*`/`wpa-*`/`vrf`/`table` options.
    pub extras: usize,
    /// `source`/`source-directory` lines.
    pub sources: usize,
    /// `mapping`/`map` lines.
    pub mappings: usize,
}

/// Whether the buffer looks like an ifupdown interfaces file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| l.trim().starts_with("iface "))
        || (t.contains("auto ") && t.contains("address"))
}

impl Interfaces {
    /// Parse an ifupdown file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            autos: 0,
            allows: 0,
            ifaces: 0,
            methods: 0,
            addressing: 0,
            dns: 0,
            hooks: 0,
            extras: 0,
            sources: 0,
            mappings: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.starts_with("auto ") {
                c.autos += 1;
            } else if s.starts_with("allow-") || s.starts_with("no-auto-down") {
                c.allows += 1;
            } else if let Some(rest) = s.strip_prefix("iface ") {
                c.ifaces += 1;
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.last().is_some_and(|m| {
                    ["static", "dhcp", "loopback", "manual", "ppp", "bootp"].contains(m)
                }) {
                    c.methods += 1;
                }
            } else if s.starts_with("source") {
                c.sources += 1;
            } else if s.starts_with("mapping") || s.starts_with("map ") {
                c.mappings += 1;
            } else if [
                "address",
                "netmask",
                "gateway",
                "broadcast",
                "network",
                "pointopoint",
                "scope",
                "metric",
                "mtu",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.addressing += 1;
            } else if s.starts_with("dns-") {
                c.dns += 1;
            } else if [
                "pre-up ",
                "up ",
                "post-up ",
                "pre-down ",
                "down ",
                "post-down ",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.hooks += 1;
            } else {
                c.extras += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file() {
        let b = concat!(
            "auto lo\n",
            "iface lo inet loopback\n",
            "allow-hotplug eth0\n",
            "iface eth0 inet static\n",
            "    address 10\n",
            "    netmask 255\n",
            "    gateway gw\n",
            "    dns-nameservers 8\n",
            "    up cmd\n",
            "iface wlan0 inet dhcp\n",
            "    wpa-ssid x\n",
            "    wpa-psk y\n",
            "source /etc/network/interfaces.d/*.cfg\n",
        );
        let c = Interfaces::parse(b.as_bytes()).unwrap();
        assert_eq!(c.autos, 1);
        assert_eq!(c.allows, 1);
        assert_eq!(c.ifaces, 3);
        assert_eq!(c.methods, 3);
        assert_eq!(c.addressing, 3);
        assert_eq!(c.dns, 1);
        assert_eq!(c.hooks, 1);
        assert_eq!(c.extras, 2);
        assert_eq!(c.sources, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Interfaces::parse(b"hello").is_none());
    }
}

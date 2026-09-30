//! firewalld zone XML census.
//!
//! firewalld zone files use `<zone>` containing `<short>`, `<description>`,
//! `<interface name=""/>`, `<source address=""/>`, `<service name=""/>`,
//! `<port port="" protocol=""/>`, `<masquerade/>`, `<forward-port .../>`,
//! `<icmp-block name=""/>` and `<rule family="...">` rich rules. `parse`
//! counts each element class.
//!
//! ```rust
//! let z = br#"<zone>
//!   <short>Public</short>
//!   <service name="ssh"/>
//!   <service name="dhcpv6-client"/>
//!   <port port="8080" protocol="tcp"/>
//!   <interface name="eth0"/>
//!   <masquerade/>
//!   <rule family="ipv4"><source address="10/24"/><accept/></rule>
//! </zone>"#;
//! let c = izanagi_kit::firewalld::Firewalld::parse(z).unwrap();
//! assert_eq!(c.services, 2);
//! assert_eq!(c.rules, 1);
//! ```

/// firewalld zone census.
#[derive(Debug, Clone)]
pub struct Firewalld {
    /// `<zone` elements.
    pub zones: usize,
    /// `<service ` elements.
    pub services: usize,
    /// `<port ` elements.
    pub ports: usize,
    /// `<interface ` elements.
    pub interfaces: usize,
    /// `<source ` elements.
    pub sources: usize,
    /// `<masquerade` elements.
    pub masquerade: usize,
    /// `<forward-port ` elements.
    pub forward_ports: usize,
    /// `<rule` elements (rich rules).
    pub rules: usize,
    /// `<icmp-block`/`<icmp-type` elements.
    pub icmps: usize,
    /// `<protocol `/`<destination` elements.
    pub others: usize,
}

fn tag(t: &str, n: &str) -> usize {
    t.matches(&format!("<{n} ")).count()
        + t.matches(&format!("<{n}/")).count()
        + t.matches(&format!("<{n}>")).count()
}

/// Whether the buffer looks like a firewalld zone file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("<zone>") || t.contains("<zone "))
        && (t.contains("<service")
            || t.contains("<port ")
            || t.contains("<interface")
            || t.contains("<masquerade")
            || t.contains("<rule"))
}

impl Firewalld {
    /// Parse a firewalld zone file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        Some(Self {
            zones: tag(t, "zone"),
            services: tag(t, "service"),
            ports: tag(t, "port"),
            interfaces: tag(t, "interface"),
            sources: tag(t, "source"),
            masquerade: tag(t, "masquerade"),
            forward_ports: tag(t, "forward-port"),
            rules: tag(t, "rule"),
            icmps: tag(t, "icmp-block") + tag(t, "icmp-type"),
            others: tag(t, "protocol") + tag(t, "destination"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_zone() {
        let b = br#"<?xml version="1.0"?>
<zone>
  <short>Internal</short>
  <service name="ssh"/>
  <service name="mdns"/>
  <port port="123" protocol="udp"/>
  <port port="80" protocol="tcp"/>
  <interface name="eth0"/>
  <source address="10/24"/>
  <masquerade/>
  <forward-port port="443" protocol="tcp" to-port="8443"/>
  <icmp-block name="redirect"/>
  <rule family="ipv4"><source address="10/24"/><accept/></rule>
</zone>"#;
        let c = Firewalld::parse(b).unwrap();
        assert_eq!(c.zones, 1);
        assert_eq!(c.services, 2);
        assert_eq!(c.ports, 2);
        assert_eq!(c.interfaces, 1);
        assert_eq!(c.sources, 2);
        assert_eq!(c.masquerade, 1);
        assert_eq!(c.forward_ports, 1);
        assert_eq!(c.rules, 1);
        assert_eq!(c.icmps, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Firewalld::parse(b"<xml />").is_none());
    }
}

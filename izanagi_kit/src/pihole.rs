//! Pi-hole `setupVars.conf` census.
//!
//! Pi-hole install state is `KEY=value` lines (`#` comments):
//! `PIHOLE_INTERFACE`, `IPV4_ADDRESS`, `IPV6_ADDRESS`,
//! `PIHOLE_DNS_1`, `WEBPASSWORD`, `QUERY_LOGGING`,
//! `INSTALL_WEB_SERVER`, `INSTALL_WEB_INTERFACE`,
//! `LIGHTTPD_ENABLED`, `BLOCKING_ENABLED`, `DNSSEC`,
//! `DNS_FQDN_REQUIRED`, `DNS_BOGUS_PRIV`, `DNSMASQ_LISTENING`,
//! `DHCP_ACTIVE`/`DHCP_START`/`DHCP_END`/`DHCP_ROUTER`,
//! `PIHOLE_DOMAIN`, `REV_SERVER`, `WEBTHEME`.
//!
//! ```rust
//! let k = b"PIHOLE_INTERFACE=eth0\nIPV4_ADDRESS=192.168.1.2/24\nPIHOLE_DNS_1=8.8.8.8\nQUERY_LOGGING=true\nINSTALL_WEB_SERVER=true\nINSTALL_WEB_INTERFACE=true\nBLOCKING_ENABLED=true\n";
//! assert!(izanagi_kit::pihole::detect(k));
//! ```

/// Pi-hole `setupVars.conf` census.
#[derive(Debug, Clone)]
pub struct Pihole {
    /// `KEY=value` lines.
    pub settings: usize,
    /// recognised keys present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "PIHOLE_INTERFACE",
    "PIHOLE_DOMAIN",
    "WEBPASSWORD",
    "QUERY_LOGGING",
    "INSTALL_WEB_SERVER",
    "INSTALL_WEB_INTERFACE",
    "LIGHTTPD_ENABLED",
    "BLOCKING_ENABLED",
    "DNSMASQ_LISTENING",
    "WEBUIBOXEDLAYOUT",
    "WEBTHEME",
    "TEMPERATUREUNIT",
    "API_QUERY_LOG_SHOW",
    "API_PRIVACY_MODE",
    "API_EXCLUDE_DOMAINS",
    "API_EXCLUDE_CLIENTS",
    "PIHOLE_PTR",
    "PRIVACY_LEVEL",
    "CACHE_SIZE",
    "REV_SERVER",
];

const WEAK: &[&str] = &[
    "IPV4_ADDRESS",
    "IPV6_ADDRESS",
    "DNSSEC",
    "DNS_FQDN_REQUIRED",
    "DNS_BOGUS_PRIV",
    "CONDITIONAL_FORWARDING",
    "CONDITIONAL_FORWARDING_IP",
    "CONDITIONAL_FORWARDING_DOMAIN",
    "CONDITIONAL_FORWARDING_REVERSE",
    "DHCP_ACTIVE",
    "DHCP_START",
    "DHCP_END",
    "DHCP_ROUTER",
    "DHCP_LEASETIME",
    "DHCP_IPv6",
    "DHCP_rapid_commit",
    "FTLCONF_LOCAL_IPV4",
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

fn is_pihole_key(k: &str) -> bool {
    STRONG.contains(&k) || k.starts_with("PIHOLE_DNS_")
}

/// Detect a Pi-hole `setupVars.conf`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `PIHOLE_*`/`QUERY_LOGGING`/`BLOCKING_ENABLED` keys are Pi-hole
    // exclusive; shared network keys only count with a strong anchor.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if is_pihole_key(k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Pihole {
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
                if is_pihole_key(k) || WEAK.contains(&k) {
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
        let b = b"PIHOLE_INTERFACE=eth0\nIPV4_ADDRESS=192.168.1.2/24\nPIHOLE_DNS_1=8.8.8.8\nQUERY_LOGGING=true\nINSTALL_WEB_SERVER=true\nINSTALL_WEB_INTERFACE=true\nBLOCKING_ENABLED=true\n";
        assert!(detect(b));
        let c = Pihole::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"IPV4_ADDRESS=1.2.3.4\nDNSSEC=true\n"));
        assert!(!detect(
            b"# PIHOLE_INTERFACE=eth0\n# QUERY_LOGGING=true\nIPV4_ADDRESS=x\n"
        ));
    }
}

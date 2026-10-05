//! tinc VPN `tinc.conf` / host files (PascalCase `Key = value`).
//!
//! ```
//! let b = b"Name = node1\nAddressFamily = ipv4\nInterface = tun0\nMode = router\nCipher = aes-256-cbc\nDigest = sha256\nCompression = 9\nConnectTo = node2\nPrivateKeyFile = /etc/tinc/rsa_key.priv\nEd25519PrivateKeyFile = /etc/tinc/ed25519_key.priv\nPort = 655\n";
//! assert!(izanagi_kit::tincconf::detect(b));
//! let c = izanagi_kit::tincconf::Tinc::parse(b).unwrap();
//! assert_eq!(c.assignments, 11);
//! ```
const KEYS: &[&str] = &[
    "Address",
    "AddressFamily",
    "AutoConnect",
    "BindToAddress",
    "BindToInterface",
    "Broadcast",
    "Cipher",
    "ClampMSS",
    "Compression",
    "ConnectTo",
    "DecrementTTL",
    "Device",
    "DeviceStandby",
    "DeviceType",
    "Digest",
    "DirectOnly",
    "ECDSAPublicKey",
    "ECDSAPublicKeyFile",
    "Ed25519PublicKey",
    "Ed25519PublicKeyFile",
    "Ed25519PrivateKeyFile",
    "ExperimentalProtocol",
    "Forwarding",
    "GraphDumpFile",
    "Hostnames",
    "IdleTimeout",
    "IffOneQueue",
    "Interface",
    "InterfaceMetri\u{63}",
    "InvitationExpire",
    "KeyExpire",
    "ListenAddress",
    "LocalDiscovery",
    "LocalDiscoveryAddress",
    "LogLevel",
    "MACAddress",
    "MACExpire",
    "MaxConnectionBurst",
    "MaxOutputBufferSize",
    "MaxTimeout",
    "Mode",
    "Name",
    "Netname",
    "PingInterval",
    "PingTimeout",
    "Port",
    "PriorityInheritance",
    "PrivateKey",
    "PrivateKeyFile",
    "ProcessPriority",
    "Proxy",
    "ProxyPassword",
    "ProxyType",
    "ProxyUser",
    "PublicKey",
    "PublicKeyFile",
    "ReplayWindow",
    "RequireStaticRoutes",
    "Scripts",
    "ScriptsInterpreter",
    "StrictSubnets",
    "Subnet",
    "SubnetDownScripts",
    "TCPOnly",
    "TicketKeyFile",
    "UDPDiscovery",
    "UDPDiscoveryKeepaliveInterval",
    "UDPDiscoveryInterval",
    "UDPDiscoveryTimeout",
    "UDPRcvBuf",
    "UDPSndBuf",
    "Weight",
    "ifconfig",
    "route",
    "nameserver",
    "tor",
    "proxy",
];

/// Detect a tinc.conf or host file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for k in KEYS {
        if t.contains(k) {
            hits += 1;
        }
    }
    hits >= 4
}

/// Structural counts for a tinc config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tinc {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `Key = value` lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

impl Tinc {
    /// Parse structural counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            } else if tr.contains('=') {
                c.assignments += 1;
            }
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"Name = node1\nAddressFamily = ipv4\nInterface = tun0\nMode = router\nCipher = aes-256-cbc\nDigest = sha256\nCompression = 9\nConnectTo = node2\nPort = 655\n";
        assert!(detect(b));
        let c = Tinc::parse(b).unwrap();
        assert_eq!(c.assignments, 9);
        assert!(c.keys >= 9);
    }

    #[test]
    fn rejects_ini() {
        let b = b"[server]\nfoo=bar\nbaz=qux\n";
        assert!(!detect(b));
        assert!(Tinc::parse(b).is_none());
    }
}

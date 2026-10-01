//! iwd `main.conf` / `<net>.psk` / `<net>.8021x` / `<net>.open` 形式の検出・カウント。
//!
//! INI: `[General]`/`[Network]`/`[Scan]`/`[IPv4]`/`[IPv6]`/`[Settings]`/
//! `[Rank]`/`[Offline]`/`[Blacklisting]`/`[Security]`/`[AutoToggle]`/
//! `[NetworkAffinity]` 等。キーは CamelCase。
//!
//! ```
//! let cfg = b"[General]\n\
//!             EnableNetworkConfiguration=true\n\
//!             [Network]\n\
//!             NameResolvingService=systemd\n\
//!             [Scan]\n\
//!             DisablePeriodicScan=false\n";
//! assert!(izanagi_kit::iwdconf::detect(cfg));
//! let c = izanagi_kit::iwdconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 3);
//! ```

/// `[General]` セクション既知キー。
const GENERAL_KEYS: &[&str] = &[
    "EnableNetworkConfiguration",
    "UseDefaultInterface",
    "AddressRandomization",
    "AddressRandomizationRange",
    "RoamThreshold",
    "RoamThreshold5G",
    "RoamRetryTimeout",
    "ManagementFrameProtection",
    "ControlPortOverNL80211",
    "DisableANQP",
    "DisableHotspot20",
    "AutoConnect",
    "Country",
    "RegdomFollowCountry",
    "LastConnectedTimeout",
    "AlwaysRandomizeAddress",
    "DisableInterfaces",
    "EnableInterfaces",
    "EnableHyBridScan",
    "DriverQuirks",
    "Idle",
];

/// `[Network]` セクション既知キー。
const NETWORK_KEYS: &[&str] = &[
    "EnableIPv6",
    "NameResolvingService",
    "RoutePriorityOffset",
    "MulticastDNS",
    "AllowDirectedProbeRequests",
    "DisableEAPoL",
    "CriticalPriority",
];

/// `[Scan]` セクション既知キー。
const SCAN_KEYS: &[&str] = &[
    "DisablePeriodicScan",
    "InitialPeriodicScanInterval",
    "MaximumPeriodicScanInterval",
    "DisableRoamingScan",
    "RoamScanInterval",
    "InitialScanInterval",
    "MaxScanInterval",
    "AllSignalThresholds",
];

/// `[IPv4]`/`[IPv6]` セクション既知キー。
const IP_KEYS: &[&str] = &[
    "Address",
    "Netmask",
    "Gateway",
    "Broadcast",
    "DNS",
    "DomainNames",
    "ClientId",
    "SendHostname",
    "RoutePriorityOffset",
    "LeaseTime",
    "APMTU",
];

/// `[Settings]`/`[Security]`/ネットワークファイル既知キー。
const NET_KEYS: &[&str] = &[
    "AutoConnect",
    "Hidden",
    "AlwaysRandomizeAddress",
    "TransitionDisable",
    "PreSharedKey",
    "Passphrase",
    "Password",
    "PasswordHash",
    "EAP-Method",
    "EAP-Identity",
    "EAP-Password",
    "EAP-CACert",
    "EAP-CACertEmbed",
    "EAP-ClientCert",
    "EAP-ClientKey",
    "EAP-ClientKeyPassphrase",
    "EAP-Phase2-Method",
    "EAP-Phase2-Identity",
    "EAP-Phase2-Password",
    "EAP-TLS-Version",
    "EAP-TLS-CipherSuite",
    "EAP-TLS-EnableFragementation",
    "EAP-TLS-DisableDomainMatch",
    "EAP-TLS-MaxCertChainLen",
    "SSID",
    "Security",
    "Hidden2",
    "SaePk",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクション総数。
    pub sections: usize,
    /// `key=value` エントリ総数。
    pub entries: usize,
    /// `[General]` キー数。
    pub general: usize,
    /// `[Network]`/`[NetworkAffinity]` キー数。
    pub network: usize,
    /// `[Scan]` キー数。
    pub scan: usize,
    /// `[IPv4]`/`[IPv6]`/`[IPv6]`/`[Rank]`/`[Offline]`/`[Blacklisting]`/`[AutoToggle]` キー数。
    pub ip: usize,
    /// `[Security]`/`[Settings]`/ネットワークファイル系キー数。
    pub security: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が iwd 設定形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を iwd 設定ファイルとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        general: 0,
        network: 0,
        scan: 0,
        ip: 0,
        security: 0,
        misc: 0,
    };
    let mut section = 9u8;
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                let name = &rest[..end];
                section = match name {
                    "General" => 0,
                    "Network" | "NetworkAffinity" => 1,
                    "Scan" => 2,
                    "IPv4" | "IPv6" | "Rank" | "Offline" | "Blacklisting" | "AutoToggle"
                    | "Priority" | "Protocol" | "Wep" | "Proxy" | "Hotspot" => 3,
                    "Security" | "Settings" => 4,
                    _ => 5,
                };
                c.sections += 1;
                continue;
            }
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        c.entries += 1;
        match section {
            0 => {
                if GENERAL_KEYS.contains(&key) {
                    c.general += 1;
                    known += 1;
                } else {
                    c.misc += 1;
                }
            }
            1 => {
                if NETWORK_KEYS.contains(&key) {
                    c.network += 1;
                    known += 1;
                } else {
                    c.misc += 1;
                }
            }
            2 => {
                if SCAN_KEYS.contains(&key) {
                    c.scan += 1;
                    known += 1;
                } else {
                    c.misc += 1;
                }
            }
            3 => {
                if IP_KEYS.contains(&key) {
                    c.ip += 1;
                    known += 1;
                } else {
                    c.misc += 1;
                }
            }
            4 if NET_KEYS.contains(&key) => {
                c.security += 1;
                known += 1;
            }
            4 => {
                c.misc += 1;
            }
            _ => {
                c.misc += 1;
            }
        }
    }
    if known >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[General]\n\
        EnableNetworkConfiguration=true\n\
        AddressRandomization=once\n\
        RoamThreshold=-70\n\
        ControlPortOverNL80211=true\n\
        \n\
        [Network]\n\
        EnableIPv6=true\n\
        NameResolvingService=systemd\n\
        RoutePriorityOffset=200\n\
        \n\
        [Scan]\n\
        DisablePeriodicScan=false\n\
        InitialPeriodicScanInterval=10\n\
        \n\
        [IPv4]\n\
        Address=192.168.1.10\n\
        Netmask=255.255.255.0\n\
        Gateway=192.168.1.1\n\
        DNS=8.8.8.8\n";

    #[test]
    fn detects_iwd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.entries, 13);
        assert_eq!(c.general, 4);
        assert_eq!(c.network, 3);
        assert_eq!(c.scan, 2);
        assert_eq!(c.ip, 4);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(!detect(b"[foo]\nx=1\n"));
        assert!(!detect(b"[General]\nbar=1\n"));
    }
}

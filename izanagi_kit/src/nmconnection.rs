//! NetworkManager `.nmconnection` / `keyfile` 形式の検出・カウント。
//!
//! INI: `[connection]`/`[ipv4]`/`[ipv6]`/`[wifi]`/`[wifi-security]`/`[802-1x]`/
//! `[ethernet]`/`[proxy]`/`[wireguard]`/`[gsm]`/`[cdma]`/`[bridge]`/`[bond]`/
//! `[vlan]`/`[dummy]`/`[macvlan]`/`[ppp]`/`[pppoe]`/`[vpn]`/`[serial]`/`[user]`/
//! `[wireguard-peer.<pk>]`。
//!
//! ```
//! let cfg = b"[connection]\n\
//!             id=MyWiFi\n\
//!             uuid=11111111-2222-3333-4444-555555555555\n\
//!             type=wifi\n\
//!             autoconnect=true\n\
//!             [wifi]\n\
//!             ssid=MyNet\n\
//!             [wifi-security]\n\
//!             key-mgmt=wpa-psk\n\
//!             psk=secret\n";
//! assert!(izanagi_kit::nmconnection::detect(cfg));
//! let c = izanagi_kit::nmconnection::parse(cfg).unwrap();
//! assert_eq!(c.sections, 3);
//! ```

/// `[ipv4]`/`[ipv6]` セクション既知キー。
const IP_KEYS: &[&str] = &[
    "method",
    "dns",
    "dns-search",
    "dns-options",
    "dns-priority",
    "addresses",
    "address1",
    "address2",
    "address3",
    "address4",
    "address5",
    "address6",
    "address7",
    "address8",
    "address9",
    "gateway",
    "routes",
    "route1",
    "route2",
    "route3",
    "route4",
    "route5",
    "route6",
    "route7",
    "route8",
    "route9",
    "route-metric",
    "route-table",
    "ignore-auto-routes",
    "ignore-auto-dns",
    "dhcp-send-hostname",
    "dhcp-hostname",
    "dhcp-hostname-flags",
    "never-default",
    "may-fail",
    "dad-timeout",
    "dhcp-timeout",
    "dhcp-iaid",
    "dhcp-duid",
    "dhcp-client-id",
    "dhcp-fqdn",
    "dhcp-vendor-class-identifier",
    "dhcp-user-class-identifier",
    "link-local",
    "ra-timeout",
    "addr-gen-mode",
    "privacy",
    "ip6-privacy",
    "temp-pref",
    "token",
    "mtu",
    "required-timeout",
    "replace-local-rule",
    "routing-rules",
];

/// `[wifi]`/`[wifi-security]`/`[802-1x]`/`[ethernet]`/`[proxy]` その他既知キー。
const OTHER_KEYS: &[&str] = &[
    "ssid",
    "mode",
    "band",
    "channel",
    "bssid",
    "rate",
    "tx-power",
    "mac-address",
    "cloned-mac-address",
    "generate-mac-address-mask",
    "mac-address-blacklist",
    "mtu",
    "seen-bssids",
    "hidden",
    "powersave",
    "wake-on-wlan",
    "ap-isolation",
    "key-mgmt",
    "wep-tx-keyidx",
    "auth-alg",
    "proto",
    "pairwise",
    "group",
    "pmf",
    "wep-key0",
    "wep-key1",
    "wep-key2",
    "wep-key3",
    "wep-key-type",
    "psk",
    "psk-flags",
    "leap-username",
    "leap-password",
    "fil",
    "sae-pwe",
    "sae-pk",
    "owe-transition-interface",
    "eap",
    "identity",
    "anonymous-identity",
    "password",
    "password-flags",
    "password-raw",
    "private-key",
    "private-key-password",
    "private-key-password-flags",
    "phase1-auth-flags",
    "phase1-fast-provisioning",
    "phase1-peapver",
    "phase1-peaplabel",
    "phase1-outer-capability",
    "phase2-auth",
    "phase2-autheap",
    "phase2-ca-cert",
    "phase2-client-cert",
    "phase2-private-key",
    "cert-matcher",
    "ca-cert",
    "ca-path",
    "subject-match",
    "altsubject-matches",
    "domain-suffix-match",
    "domain-match",
    "system-ca-certs",
    "client-cert",
    "pac-file",
    "pin",
    "port",
    "duplex",
    "speed",
    "auto-negotiate",
    "s390-subchannels",
    "s390-nettype",
    "s390-options",
    "wake-on-lan",
    "wake-on-lan-password",
    "accept-all-mac-addresses",
    "wired",
    "method2",
    "browser-only",
    "pac-url",
    "pac-script",
];

/// `[proxy]` キー。
const PROXY_KEYS: &[&str] = &["method", "browser-only", "pac-url", "pac-script"];

/// セクション名を分類。
fn classify_section(name: &str) -> u8 {
    match name {
        "connection" => 0,
        "ipv4" | "ipv6" | "ipv4.dad-timeout" => 1,
        "wifi" | "wifi-security" | "802-1x" | "ethernet" | "802-3-ethernet" => 2,
        "proxy" => 3,
        "wireguard" | "gsm" | "cdma" | "bridge" | "bond" | "vlan" | "dummy" | "macvlan" | "ppp"
        | "pppoe" | "vpn" | "serial" | "team" | "tun" | "ip-tunnel" | "macsec" | "veth"
        | "vxlan" | "wpan" | "6lowpan" | "wireguard-peer" | "ovs-bridge" | "ovs-interface"
        | "ovs-port" | "ovs-patch" | "user" | "ethtool" | "match" | "wifi-p2p" | "dun"
        | "bluetooth" | "gsm-cdma" | "infiniband" | "loopback" | "tc" | "vrf" | "hsr" | "ipsec"
        | "crypto" | "ptp" | "mlo" | "adsl" | "isdn" => 4,
        _ if name.starts_with("wireguard-peer.") => 5,
        _ => 6,
    }
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクション総数。
    pub sections: usize,
    /// `key=value` エントリ総数。
    pub entries: usize,
    /// `[connection]` キー数。
    pub connection: usize,
    /// `[ipv4]`/`[ipv6]` キー数。
    pub ip: usize,
    /// `[wifi]`/`[wifi-security]`/`[802-1x]`/`[ethernet]`/`[proxy]` キー数。
    pub wifi: usize,
    /// その他セクション(wireguard/gsm/bridge 等)キー数。
    pub other: usize,
    /// `[wireguard-peer.*]` 等ピア・サブセクション数。
    pub peers: usize,
}

/// `b` が NetworkManager keyfile 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.connection >= 2 && c.sections >= 1)
}

/// `b` を NetworkManager keyfile として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        connection: 0,
        ip: 0,
        wifi: 0,
        other: 0,
        peers: 0,
    };
    let mut section = 7u8;
    let mut has_conn = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                let name = &rest[..end];
                section = classify_section(name);
                c.sections += 1;
                if section == 0 {
                    has_conn = true;
                }
                if section == 5 {
                    c.peers += 1;
                }
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
                c.connection += 1;
            }
            1 => {
                if IP_KEYS.contains(&key) || key.starts_with("address") || key.starts_with("route")
                {
                    c.ip += 1;
                } else {
                    c.other += 1;
                }
            }
            2 | 3 if OTHER_KEYS.contains(&key) || (section == 3 && PROXY_KEYS.contains(&key)) => {
                c.wifi += 1;
            }
            2 | 3 => {
                c.other += 1;
            }
            _ => {
                c.other += 1;
            }
        }
    }
    if has_conn && c.connection >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[connection]\n\
        id=MyWiFi\n\
        uuid=11111111-2222-3333-4444-555555555555\n\
        type=wifi\n\
        interface-name=wlan0\n\
        autoconnect=true\n\
        permissions=user:admin\n\
        \n\
        [wifi]\n\
        ssid=MyNet\n\
        mode=infrastructure\n\
        hidden=false\n\
        \n\
        [wifi-security]\n\
        key-mgmt=wpa-psk\n\
        psk=verysecret\n\
        proto=rsn\n\
        \n\
        [ipv4]\n\
        method=auto\n\
        dns=8.8.8.8;1.1.1.1;\n\
        address1=192.168.1.5/24\n\
        gateway=192.168.1.1\n\
        may-fail=false\n\
        \n\
        [ipv6]\n\
        method=auto\n\
        privacy=2\n";

    #[test]
    fn detects_nmconnection() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.entries, 19);
        assert_eq!(c.connection, 6);
        assert_eq!(c.ip, 7);
        assert_eq!(c.wifi, 6);
        assert_eq!(c.other, 0);
        assert_eq!(c.peers, 0);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(!detect(b"[foo]\nx=1\n"));
        assert!(!detect(b"[connection]\nid=x\n"));
    }
}

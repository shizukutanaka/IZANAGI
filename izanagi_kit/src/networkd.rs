//! systemd-networkd `.network`/`.netdev`/`.link` 設定の認識と計数。
//!
//! networkd 設定は INI 形式で、`.network` は `[Match]`(`Name=`/`MACAddress=`/
//! `Type=`/`Host=`/`Virtualization=`/`KernelCommandLine=`)+`[Link]`
//! (`RequiredForOnline=`)+`[Network]`(`Address=`/`Gateway=`/`DNS=`/`DHCP=`/
//! `IPv6LinkLocalAddressGenerationMode=`/`LLDP=`/`EmitLLDP=`)+`[Address]`
//! `[Route]`(`Gateway=`/`Destination=`/`Metric=`)+`[DHCPv4]`/`[DHCPv6]`/
//! `[IPv6AcceptRA]`/`[DHCPServer]`/`[IPv6PrefixDelegation]`/`[Bridge]`/
//! `[BridgeVLAN]`/`[CAN]` セクションを持つ。
//! `.netdev` は `[NetDev]`(`Name=`/`Kind=`/`MTUBytes=`/`MACAddress=`)+
//! `Kind` 別セクション(`[VLAN]`/`[Bond]`/`[Bridge]`/`[VXLAN]`/`[Tunnel]`/
//! `[WireGuard]`/`[Peer]`)。`.link` は `[Match]`+`[Link]`(`Description=`/
//! `MACAddressPolicy=`/`NamePolicy=`/`MTUBytes=`/`BitsPerSecond=`/`WakeOnLan=`)。
//!
//! ```
//! let b = b"[Match]\nName=eth*\nMACAddress=52:54:00:12:34:56\n\n[Network]\nDHCP=yes\nAddress=192.168.10.10/24\nGateway=192.168.10.1\nDNS=192.168.10.1\nIPv6LinkLocalAddressGenerationMode=eui64\nLLDP=yes\n\n[Route]\nGateway=192.168.10.1\nDestination=0.0.0.0/0\nMetric=100\n\n[DHCPv4]\nUseDNS=yes\nUseNTP=yes\nRouteMetric=100\n";
//! assert!(izanagi_kit::networkd::detect(b));
//! let c = izanagi_kit::networkd::parse(b).unwrap();
//! assert_eq!(c.assigns, 14);
//! assert_eq!(c.sections, 4); // Match/Network/Route/DHCPv4
//! assert_eq!(c.match_keys, 2);
//! assert_eq!(c.route_keys, 6); // Route 3 + DHCPv4 3(RouteMetric含む)
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` 代入行の個数。
    pub assigns: usize,
    /// `[Match]`/`[Link]`/`[Network]`/`[Address]`/`[Route]`/`[DHCPv4]`/
    /// `[DHCPv6]`/`[IPv6AcceptRA]`/`[DHCPServer]`/`[IPv6PrefixDelegation]`/
    /// `[NetDev]`/`[VLAN]`/`[Bond]`/`[Bridge]`/`[VXLAN]`/`[Tunnel]`/
    /// `[WireGuard]`/`[Peer]`/`[CAN]` 等セクションの個数。
    pub sections: usize,
    /// `[Match]` セクション内の `Name=`/`MACAddress=`/`Type=`/`Host=`/
    /// `Virtualization=`/`KernelCommandLine=`/`Architecture=`/`SSID=`/`BSSID=`/
    /// `Driver=`/`PermanentMACAddress=`/`Path=`/`Property=` キーの個数。
    pub match_keys: usize,
    /// `[Address]`/`[Route]`/`[RoutePrefix]`/`[RoutingPolicyRule]`/
    /// `[DHCPv4]`/`[DHCPv6]`/`[DHCPServer]`/`[IPv6AcceptRA]`/`[DHCPPrefixDelegation]`
    /// 配下のキーの個数。
    pub route_keys: usize,
    /// `yes`/`no`/`true`/`false` 値の個数。
    pub bools: usize,
    /// `/` プレフィックス付き値(CIDR `x.x.x.x/24`・`::/0`)の個数。
    pub cidrs: usize,
    /// `#`/`;` コメント行の個数。
    pub comments: usize,
}

const SECTION_KEYS: &[&str] = &[
    "Match",
    "Link",
    "Network",
    "Address",
    "Route",
    "NextHop",
    "RoutePrefix",
    "RoutingPolicyRule",
    "DHCPv4",
    "DHCPv6",
    "DHCPServer",
    "IPv6AcceptRA",
    "IPv6PrefixDelegation",
    "IPv6Prefix",
    "DHCPPrefixDelegation",
    "NetDev",
    "VLAN",
    "MACVLAN",
    "MACVTAP",
    "IPVLAN",
    "IPVTAP",
    "VXLAN",
    "IPIPSITAP",
    "Geneve",
    "L2TP",
    "L2TPSession",
    "MACsec",
    "MACsecReceiveChannel",
    "MACsecReceiveAssociation",
    "MACsecTransmitAssociation",
    "Tunnel",
    "FooOverUDP",
    "TUN",
    "Tap",
    "Bridge",
    "Bond",
    "Peer",
    "VLAN_MAC",
    "BridgeVLAN",
    "BridgeMDB",
    "LLDP",
    "CAN",
    "QDisc",
    "NetworkEmulator",
    "TokenBucketFilter",
    "PIE",
    "FlowQueuePIE",
    "FlowQueuePIEOK",
    "StochasticFairBlue",
    "StochasticFairnessQueueing",
    "BFIFO",
    "PFIFO",
    "PriorityFIFO",
    "ControlledDelay",
    "DeficitRoundRobinScheduler",
    "DeficitRoundRobinSchedulerQDisc",
    "EnhancedTransmissionSelection",
    "GenericRandomEarlyDetection",
    "FairQueueingControlledDelay",
    "FairQueueing",
    "ControlledDelayFlowQueue",
    "FlowQueuePIEControlledDelay",
    "QuickFairQueueingPIE",
    "QuickFairQueueing",
    "HRSystem",
    "DRR",
    "QFQ",
    "CAKE",
    "TAP",
    "SIT",
    "WireGuard",
    "WireGuardPeer",
    "BareUDP",
    "BatmanAdvanced",
    "IPoIB",
    "NLMON",
    "VRF",
    "IFB",
    "MACsec",
];

const MATCH_KEYS: &[&str] = &[
    "Name",
    "MACAddress",
    "Type",
    "Host",
    "Virtualization",
    "KernelCommandLine",
    "Architecture",
    "SSID",
    "BSSID",
    "Driver",
    "PermanentMACAddress",
    "Path",
    "Property",
    "WLANInterfaceType",
    "Kind",
    "PhysicalDevice",
    "IFType",
    "ContainerInterface",
    "BindCarrier",
    "Unmanaged",
    "ActiveSlave",
    "PrimarySlave",
];

const ROUTE_SECTIONS: &[&str] = &[
    "Address",
    "Route",
    "NextHop",
    "RoutePrefix",
    "RoutingPolicyRule",
    "DHCPv4",
    "DHCPv6",
    "DHCPServer",
    "IPv6AcceptRA",
    "DHCPPrefixDelegation",
    "IPv6Prefix",
    "IPv6RoutePrefix",
    "SRv6",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `.network`/`.netdev`/`.link` らしさを返す。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with('[') && s.ends_with(']') && SECTION_KEYS.contains(&&s[1..s.len() - 1]) {
            hits += 1;
        }
    }
    hits >= 1
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        assigns: 0,
        sections: 0,
        match_keys: 0,
        route_keys: 0,
        bools: 0,
        cidrs: 0,
        comments: 0,
    };
    let mut cur: i64 = -1; // -1=なし, 0=その他, 1=Match, 2=Route系
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') && s.ends_with(']') {
            let name = &s[1..s.len() - 1];
            if SECTION_KEYS.contains(&name) {
                c.sections += 1;
            }
            cur = match name {
                "Match" => 1,
                x if ROUTE_SECTIONS.contains(&x) => 2,
                _ => 0,
            };
            continue;
        }
        let Some(eq) = s.find('=') else {
            continue;
        };
        let k = s[..eq].trim_end();
        if k.is_empty() || !k.chars().all(|x| x.is_ascii_alphanumeric() || x == '_') {
            continue;
        }
        c.assigns += 1;
        let v = s[eq + 1..].trim();
        if matches!(v, "yes" | "no" | "true" | "false") {
            c.bools += 1;
        }
        if v.contains('/')
            && v.split('/').next().is_some_and(|a| {
                a.chars()
                    .all(|x| x.is_ascii_digit() || x == '.' || x == ':')
            })
        {
            c.cidrs += 1;
        }
        if cur == 1 && MATCH_KEYS.contains(&k) {
            c.match_keys += 1;
        }
        if cur == 2 {
            c.route_keys += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(b"[Match]\nName=eth*\n[Network]\nDHCP=yes\n"));
        assert!(detect(b"[NetDev]\nName=vlan10\nKind=vlan\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"[Server]\nPort=80\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"[Match]\nName=eth0\n\n[Network]\nDHCP=yes\nDNS=8.8.8.8\n\n[Route]\nGateway=10.0.0.1\n";
        let c = parse(b).unwrap();
        assert_eq!(c.assigns, 4);
        assert_eq!(c.sections, 3);
        assert_eq!(c.match_keys, 1);
        assert_eq!(c.route_keys, 1); // Route 配下のみ
        assert_eq!(c.bools, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"[Match]\nName=x\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

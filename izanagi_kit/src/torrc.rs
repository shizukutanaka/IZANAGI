//! `torrc` (Tor デーモン設定) 検出モジュール。
//!
//! torrc は `Keyword value` 行形式で、`SocksPort`/`ORPort`/
//! `ExitPolicy`/`ExitNodes`/`HiddenServiceDir`/`HiddenServicePort`/
//! `Nickname`/`ContactInfo`/`BandwidthRate`/`RelayBandwidthRate`/
//! `Bridge`/`UseBridges`/`ExcludeExitNodes`/`DataDirectory`/
//! `Log`/`ControlPort`/`CookieAuthentication` 等のキーで構成される。
//!
//! ```
//! let b = br#"SocksPort 9050
//! ORPort 9001
//! Nickname myrelay
//! ContactInfo admin@example.com
//! ExitPolicy reject *:*
//! "#;
//! let c = izanagi_kit::torrc::parse(b);
//! assert!(izanagi_kit::torrc::detect(b));
//! assert_eq!(c.directives, 5);
//! ```

const KEYWORDS: &[&str] = &[
    "AccountingMax",
    "Address",
    "AllowDotExit",
    "AssumeReachable",
    "AuthDirGuardsUpdateInterval",
    "AutomapHostsOnResolve",
    "BandwidthBurst",
    "BandwidthRate",
    "Bridge",
    "BridgeRelay",
    "ClientOnly",
    "ClientPreferIPv6ORPort",
    "ClientUseIPv6",
    "ConnLimit",
    "ContactInfo",
    "ControlPort",
    "ControlSocket",
    "CookieAuthentication",
    "DataDirectory",
    "DirPort",
    "DNSPort",
    "EntryNodes",
    "ExcludeExitNodes",
    "ExcludeNodes",
    "ExitNodes",
    "ExitPolicy",
    "ExtORPort",
    "FascistFirewall",
    "GeoIPFile",
    "HiddenServiceDir",
    "HiddenServicePort",
    "HiddenServiceVersion",
    "HTTPTunnelPort",
    "KeyDir",
    "LearnCircuitBuildTimeout",
    "Log",
    "MaxCircuitDirtiness",
    "MyFamily",
    "NATDPort",
    "Nickname",
    "OfflineMasterKey",
    "ORPort",
    "PublishServerDescriptor",
    "ReachableAddresses",
    "RelayBandwidthBurst",
    "RelayBandwidthRate",
    "RunAsDaemon",
    "SafeLogging",
    "ServerDNSResolvConfFile",
    "SocksPort",
    "TestingTorNetwork",
    "TransPort",
    "UseBridges",
    "UseEntryGuards",
    "VidaliaPidFile",
    "VirtualAddrNetworkIPv4",
];

fn tor_key(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    KEYWORDS.contains(&head)
}

/// `b` が torrc に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if tor_key(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// torrc の統計。
#[derive(Debug, Default, Clone)]
pub struct Torrc {
    /// 既知ディレクティブ行数。
    pub directives: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を torrc として統計する。
pub fn parse(b: &[u8]) -> Torrc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Torrc::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if tor_key(tr) {
            c.directives += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"SocksPort 9050
ORPort 9001
ExitPolicy reject *:*
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 3);
    }

    #[test]
    fn detects_hidden_service() {
        let b = br#"HiddenServiceDir /var/lib/tor/hidden/
HiddenServicePort 80 127.0.0.1:8080
Nickname svc
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"SocksPort 9050\nORPort 9001\n"));
        assert!(!detect(b"socksport 9050\norport 9001\nexitpolicy reject\n"));
        assert!(!detect(b"foo a\nbar b\nbaz c\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}

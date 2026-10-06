//! GNU Gatekeeper `gnugk.ini` census.
//!
//! INI with namespaced sections (`[Gatekeeper::Main]`, `[RoutedMode]`,
//! `[Proxy]`, `[GkStatus::Auth]`, `[RasSrv::*]`, `[Endpoint]`,
//! `[RewriteCLI]`, `[ModeSelection]` …) and `key=value` directives.
//!
//! ```rust
//! let g = b"[Gatekeeper::Main]\nFourtytwo=42\nName=OpenGK\n[RoutedMode]\nGKRouted=1\nH245Routed=0\nAcceptNeighborsCalls=1\n";
//! assert!(izanagi_kit::gnugk::detect(g));
//! let c = izanagi_kit::gnugk::Gnugk::parse(g).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// gnugk.ini census.
#[derive(Debug, Clone)]
pub struct Gnugk {
    /// `[Section]`/`[Section::Sub]` headers matching a known section.
    pub sections: usize,
    /// `key=value` lines matching a known gnugk directive.
    pub settings: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

/// Section names (exact or `prefix::` wildcard namespaces).
const SECTIONS: &[&str] = &[
    "CallTable",
    "Codecs",
    "Endpoint",
    "FileAcct",
    "Gatekeeper::Main",
    "GkStatus::Auth",
    "LogFile",
    "ModeSelection",
    "Proxy",
    "RasSrv::ARQFeatures",
    "RasSrv::AssignedAliases",
    "RasSrv::GWPrefixes",
    "RasSrv::LRQFeatures",
    "RasSrv::Neighbors",
    "RasSrv::PermanentEndpoints",
    "RasSrv::RRQAuth",
    "RasSrv::RRQFeatures",
    "RewriteCLI",
    "RoutedMode",
    "RoutingPolicy",
    "Routing::CatchAll",
    "Routing::DNS",
    "Routing::ENUM",
    "Routing::LDAP",
    "Routing::SQL",
    "SimpleSQLAuth",
    "SqlAcct",
    "SqlAuth",
    "Status::Auth",
    "TCPAcct",
];

/// gnugk directives (key before `=`).
const KEYS: &[&str] = &[
    "AcceptNeighborsCalls",
    "AcceptUnregisteredCalls",
    "AcceptOnlyPrefixedRouteToAliases",
    "AdditantionalFields",
    "AssignAliases",
    "Authenticator",
    "AuthorizeReply",
    "Broadcast",
    "CallSignalPort",
    "CompareAliasType",
    "ConvertToE164",
    "DefaultEndpoint",
    "DefaultTimeout",
    "DelayReject",
    "DisableCallRerouting",
    "DynamicMethod",
    "EmergencyIPs",
    "EnableH235Auth",
    "EnableH46018",
    "EncryptAllPasswords",
    "EndpointFilter",
    "EndpointIdentifierSuffix",
    "ExternalIP",
    "Fourtytwo",
    "Gatekeeper",
    "GKRouted",
    "H245PortRange",
    "H245Routed",
    "H46018Enabled",
    "Home",
    "InterfaceTable",
    "IPAddress",
    "Lis",
    "LocationTable",
    "ManifestsFile",
    "MulticastGroup",
    "Name",
    "NetworkInterfaces",
    "Port",
    "Prefix",
    "Proxy",
    "Q931PortRange",
    "Redirect",
    "RegStatusPort",
    "RemoveH235",
    "RequireH235",
    "RouteToAlias",
    "RTPPortRange",
    "SecSrv",
    "SetupTimeout",
    "SignalCallId",
    "SignalTimeout",
    "SQLDriver",
    "SqlHosts",
    "StatusPort",
    "SupportNATedEndpoints",
    "TimeToLive",
    "TimestampFormat",
    "TotalBandwidth",
    "UseBroadcastListener",
    "UseProvisionalRespToRoute",
    "Username",
];

fn section(t: &str) -> Option<&str> {
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim())
}

fn known_section(s: &str) -> bool {
    SECTIONS.contains(&s)
        || s.starts_with("RasSrv::")
        || s.starts_with("Routing::")
        || s.starts_with("RoutingPolicy::")
        || s.starts_with("Rule::")
        || s.starts_with("RewriteCLI")
        || s.starts_with("Endpoint::")
}

fn assign_key(l: &str) -> Option<&str> {
    let t = l.trim();
    if t.is_empty() || t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

/// Detect a `gnugk.ini`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if let Some(s) = section(tr) {
            if known_section(s) {
                secs += 1;
            }
            continue;
        }
        if let Some(k) = assign_key(l) {
            if KEYS.contains(&k) {
                keys += 1;
            }
        }
    }
    secs >= 1 && keys >= 2 || keys >= 3
}

impl Gnugk {
    /// Count sections and directives. Returns `None` when the input does
    /// not look like a `gnugk.ini`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with(';') || tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(s) = section(tr) {
                if known_section(s) {
                    c.sections += 1;
                }
                continue;
            }
            if let Some(k) = assign_key(l) {
                if KEYS.contains(&k) {
                    c.settings += 1;
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
        let b = b"; gnugk.ini\n[Gatekeeper::Main]\nFourtytwo=42\nName=OpenGK\n[RoutedMode]\nGKRouted=1\nH245Routed=0\nQ931PortRange=30000-39999\nH245PortRange=40000-49999\nAcceptNeighborsCalls=1\nAcceptUnregisteredCalls=1\n[Proxy]\nEnable=1\nProxyForNAT=1\n[Endpoint]\nGatekeeper=192.168.1.1\n";
        assert!(detect(b));
        let c = Gnugk::parse(b).unwrap();
        assert_eq!(c.sections, 4);
        assert!(c.settings >= 9);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[database]\nhost=x\nport=1\n"));
        assert!(!detect(b"foo=1\nbar=2\n"));
        assert!(Gnugk::parse(b"").is_none());
    }
}

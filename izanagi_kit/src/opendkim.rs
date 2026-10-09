//! OpenDKIM `opendkim.conf` census.
//!
//! Whitespace-separated `Key value` settings (`Syslog`,
//! `SyslogFacility`, `SyslogSuccess`, `LogWhy`, `Domain`,
//! `Selector`, `KeyFile`, `KeyTable`, `SigningTable`,
//! `InternalHosts`, `ExternalIgnoreList`, `ExemptDomains`,
//! `Domains`, `SubDomains`, `Socket`, `PidFile`, `UMask`,
//! `UserID`, `Canonicalization`, `Mode`, `SignatureAlgorithm`,
//! `Signatures`, `Minimum`, `BodyLengths`, `BodyLengthDB`,
//! `Include`, `ADSPAction`, `ADSPNoSuchDomain`,
//! `ADSPExtraCandidate`, `AlwaysAddARHeader`,
//! `AlwaysSignHeaders`, `AlwaysAddReputation`,
//! `Anonymous`, `AutoRestart`, `AutoRestartCount`,
//! `AutoRestartRate`, `Background`, `BaseDirectory`,
//! `BodyLengthDB`, `CaptureUnknownErrors`,
//! `ClockDrift`, `CopyHeadersTo`, `Diagnostics`,
//! `DNSTimeout`, `EnableDefaultSender`, `DomainKeysCompat`,
//! `DoNotDeliver`, `ExactConfiguration`, `FixedTimestamp`,
//! `IdentityHeader`, `IdentityHeaderRemove`,
//! `Include`, `InternalHosts`, `KeepAuthResults`,
//! `KeepDisconnected`, `LocalADSP`, `LocalADSP_Always`,
//! `MacroList`, `MilterDebug`, `MinimumKeyBits`,
//! `MTA`, `MTACommand`, `MustBeSigned`,
//! `Nameservers`, `NoHeaderB`, `OmitHeaders`,
//! `On-BadSignature`, `On-Default`, `On-DNSError`,
//! `On-InternalError`, `On-KeyNotFound`, `On-NoSignature`,
//! `On-Security`, `OverSignHeaders`, `PeerList`,
//! `RemoveOldSignatures`, `ReportAddress`,
//! `RequestedReports`, `ReputationRoot`,
//! `RequiredHeaders`, `ResignMailTo`,
//! `ResolverTracing`, `ScreenPolicyScript`,
//! `SelectorHeader`, `SenderHeaders`,
//! `SendReports`, `SignHeaders`, `SignatureTTL`,
//! `StatisticsName`, `StatisticsIgnoreList`,
//! `StrictHeaders`, `TestPublicKeys`,
//! `TrustAnchorFile`, `Unbound`, `UnboundConfigFile`,
//! `VBR-*`, `X-Header`, `zfilterFlags`).
//!
//! ```rust
//! let o = "Syslog yes\nDomain example.org\nSelector mail\nKeyFile /etc/dkim/mail.private\nSocket inet:8891@localhost\n";
//! let c = izanagi_kit::opendkim::Opendkim::parse(o.as_bytes()).unwrap();
//! assert_eq!(c.settings, 5);
//! ```

use crate::textutil::strip_bom;
/// opendkim.conf census.
#[derive(Debug, Clone)]
pub struct Opendkim {
    /// `Key value` lines.
    pub settings: usize,
    /// Recognised opendkim keywords.
    pub named: usize,
    /// `Include` directives.
    pub includes: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "Syslog",
    "SyslogFacility",
    "SyslogSuccess",
    "LogWhy",
    "Domain",
    "Selector",
    "KeyFile",
    "KeyTable",
    "SigningTable",
    "InternalHosts",
    "ExternalIgnoreList",
    "ExemptDomains",
    "Domains",
    "SubDomains",
    "Socket",
    "PidFile",
    "UMask",
    "UserID",
    "Canonicalization",
    "Mode",
    "SignatureAlgorithm",
    "Signatures",
    "Minimum",
    "MinimumKeyBits",
    "BodyLengths",
    "BodyLengthDB",
    "ADSPAction",
    "ADSPNoSuchDomain",
    "ADSPExtraCandidate",
    "AlwaysAddARHeader",
    "AlwaysAddReputation",
    "AlwaysSignHeaders",
    "Anonymous",
    "AutoRestart",
    "AutoRestartCount",
    "AutoRestartRate",
    "Background",
    "BaseDirectory",
    "CaptureUnknownErrors",
    "ClockDrift",
    "CopyHeadersTo",
    "Diagnostics",
    "DNSTimeout",
    "EnableDefaultSender",
    "DomainKeysCompat",
    "DoNotDeliver",
    "ExactConfiguration",
    "FixedTimestamp",
    "IdentityHeader",
    "IdentityHeaderRemove",
    "Include",
    "KeepAuthResults",
    "KeepDisconnected",
    "LocalADSP",
    "MacroList",
    "MilterDebug",
    "MTA",
    "MTACommand",
    "MustBeSigned",
    "Nameservers",
    "NoHeaderB",
    "OmitHeaders",
    "OverSignHeaders",
    "PeerList",
    "RemoveOldSignatures",
    "ReportAddress",
    "RequestedReports",
    "ReputationRoot",
    "RequiredHeaders",
    "ResignMailTo",
    "ResolverTracing",
    "ScreenPolicyScript",
    "SelectorHeader",
    "SenderHeaders",
    "SendReports",
    "SignHeaders",
    "SignatureTTL",
    "StatisticsName",
    "StatisticsIgnoreList",
    "StrictHeaders",
    "TestPublicKeys",
    "TrustAnchorFile",
    "Unbound",
    "UnboundConfigFile",
    "VBR-PurgeFields",
    "VBR-TrustedCertifiers",
    "VBR-Type",
    "X-Header",
    "zfilterFlags",
];

/// Detect opendkim.conf content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let head = s.split_whitespace().next().unwrap_or("");
        if KEYS.contains(&head) {
            hits += 1;
        }
    }
    hits >= 2
}

impl Opendkim {
    /// Census an opendkim.conf buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            settings: 0,
            named: 0,
            includes: 0,
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
            let head = s.split_whitespace().next().unwrap_or("");
            if head.is_empty() {
                continue;
            }
            c.settings += 1;
            if head == "Include" {
                c.includes += 1;
            }
            if KEYS.contains(&head) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_conf() {
        let b = b"Syslog yes\nDomain example.org\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_conf() {
        let b = concat!(
            "# opendkim.conf\n",
            "Syslog yes\n",
            "SyslogSuccess yes\n",
            "Domain example.org\n",
            "Selector mail\n",
            "KeyFile /etc/dkim/mail.private\n",
            "Canonicalization relaxed/simple\n",
            "Mode sv\n",
            "Socket inet:8891@localhost\n",
            "PidFile /var/run/opendkim/opendkim.pid\n",
            "UserID opendkim:opendkim\n",
            "Include /etc/opendkim/extra.conf\n",
            "AutoRestart yes\n",
            "DNSTimeout 5\n",
            "SignatureAlgorithm rsa-sha256\n",
        );
        let c = Opendkim::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 14);
        assert_eq!(c.named, 14);
        assert_eq!(c.includes, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

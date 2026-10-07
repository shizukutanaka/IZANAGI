//! OpenDMARC `opendmarc.conf` census.
//!
//! Whitespace-separated `Key value` settings (`AuthservID`,
//! `TrustedAuthservIDs`, `AuthservIDWithJobID`,
//! `IgnoreMailFrom`, `IgnoreHosts`, `IgnoreAuthenticatedClients`,
//! `RequiredHeaders`, `PublicSuffixList`, `PSLFile`,
//! `SPFIgnoreResults`, `SPFSelfValidate`,
//! `FailureReports`, `FailureReportsBcc`,
//! `FailureReportsOnNone`, `FailureReportsSentBy`,
//! `FailureReportsFrom`, `FailureReportsSender`,
//! `FailureReportsSubject`, `CopyFailuresTo`,
//! `HoldQuarantinedMessages`, `HistoryFile`,
//! `HistoryFilePolicy`, `HistoryFileLockTimeout`,
//! `AutoRestart`, `AutoRestartCount`, `AutoRestartRate`,
//! `Background`, `BaseDirectory`, `ChangeRootDirectory`,
//! `DNSTimeout`, `EnableCoredumps`, `GenerateReports`,
//! `MilterDebug`, `PidFile`, `RecordAllMessages`,
//! `RejectFailures`, `Socket`, `SoftwareHeader`,
//! `SoftwareHeaderAuth`, `Syslog`, `SyslogFacility`,
//! `TempDirectory`, `UMask`, `UserID`, `WhitelistIPAddr`,
//! `IgnoreMailFrom`, `VBR-Certifiers`, `VBR-Type`,
//! `ForensicReportingSentBy`, `ForensicReportingBcc`,
//! `ForensicReportingAddresses`, `ForensicReportingSubject`,
//! `ForensicReportingSender`, `ForensicReportingType`,
//! `ForensicReportsOnNone`, `ForensicReportsSentBy`).
//!
//! ```rust
//! let o = "AuthservID mail.example.org\nFailureReports true\nRejectFailures true\nSocket inet:8893@localhost\n";
//! let c = izanagi_kit::opendmarc::Opendmarc::parse(o.as_bytes()).unwrap();
//! assert_eq!(c.settings, 4);
//! ```

/// opendmarc.conf census.
#[derive(Debug, Clone)]
pub struct Opendmarc {
    /// `Key value` lines.
    pub settings: usize,
    /// Recognised opendmarc keywords.
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "AuthservID",
    "TrustedAuthservIDs",
    "AuthservIDWithJobID",
    "IgnoreMailFrom",
    "IgnoreHosts",
    "IgnoreAuthenticatedClients",
    "IgnoreMimeAuthenticatedClients",
    "RequiredHeaders",
    "PublicSuffixList",
    "PSLFile",
    "SPFIgnoreResults",
    "SPFSelfValidate",
    "FailureReports",
    "FailureReportsBcc",
    "FailureReportsOnNone",
    "FailureReportsSentBy",
    "FailureReportsFrom",
    "FailureReportsSender",
    "FailureReportsSubject",
    "CopyFailuresTo",
    "HoldQuarantinedMessages",
    "HistoryFile",
    "HistoryFilePolicy",
    "HistoryFileLockTimeout",
    "AutoRestart",
    "AutoRestartCount",
    "AutoRestartRate",
    "Background",
    "BaseDirectory",
    "ChangeRootDirectory",
    "DNSTimeout",
    "EnableCoredumps",
    "GenerateReports",
    "MilterDebug",
    "PidFile",
    "RecordAllMessages",
    "RejectFailures",
    "Socket",
    "SoftwareHeader",
    "SoftwareHeaderAuth",
    "Syslog",
    "SyslogFacility",
    "TempDirectory",
    "UMask",
    "UserID",
    "WhitelistIPAddr",
    "VBR-Certifiers",
    "VBR-Type",
    "ForensicReportingSentBy",
    "ForensicReportingBcc",
    "ForensicReportingAddresses",
    "ForensicReportingSubject",
    "ForensicReportingSender",
    "ForensicReportingType",
    "ForensicReportsOnNone",
    "ForensicReportsSentBy",
    "OversignHeaders",
    "ReceiverMode",
    "ShutdownTimeout",
    "SkipTEMPDIRFAIL",
    "SMFIs",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect opendmarc.conf content.
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

impl Opendmarc {
    /// Census an opendmarc.conf buffer.
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
        let b = b"AuthservID mail\nRejectFailures true\n";
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
            "# opendmarc.conf\n",
            "AuthservID mail.example.org\n",
            "AuthservIDWithJobID true\n",
            "TrustedAuthservIDs HOSTNAME\n",
            "FailureReports true\n",
            "FailureReportsBcc postmaster@example.org\n",
            "HistoryFile /var/spool/opendmarc/opendmarc.dat\n",
            "HoldQuarantinedMessages true\n",
            "IgnoreMailFrom example.com\n",
            "PublicSuffixList /etc/opendmarc/effective_tld_names.dat\n",
            "RejectFailures true\n",
            "RequiredHeaders false\n",
            "Socket inet:8893@localhost\n",
            "Syslog true\n",
            "UMask 0002\n",
            "UserID opendmarc:opendmarc\n",
        );
        let c = Opendmarc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 15);
        assert_eq!(c.named, 15);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

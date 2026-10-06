//! `pure-ftpd.conf` (Pure-FTPd) census.
//!
//! `CamelCaseDirective value` lines (`MaxClientsNumber 50`,
//! `NoAnonymous yes`, `TLS 2`, `PassivePortRange 30000 50000`).
//! `#` comments.
//!
//! ```rust
//! let p = b"ChrootEveryone yes\nMaxClientsNumber 50\nNoAnonymous yes\nPassivePortRange 30000 50000\nPureDB /etc/pure-ftpd/pureftpd.pdb\n";
//! assert!(izanagi_kit::pureftpd::detect(p));
//! let c = izanagi_kit::pureftpd::Pureftpd::parse(p).unwrap();
//! assert_eq!(c.settings, 5);
//! ```

/// pure-ftpd.conf census.
#[derive(Debug, Clone)]
pub struct Pureftpd {
    /// `Key value` lines matching a known Pure-FTPd directive.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Pure-FTPd directives (first word).
const KEYS: &[&str] = &[
    "AFS",
    "ALog",
    "AllowDotFiles",
    "AllowUserFXP",
    "AltLog",
    "AnonymousBandwidth",
    "AnonymousCanCreateDirs",
    "AnonymousCantUpload",
    "AnonymousRatio",
    "AntiWarez",
    "AutoRename",
    "Bind",
    "BrokenClientsCompatibility",
    "CallUploadScript",
    "CertFile",
    "ChrootEveryone",
    "ClientCharset",
    "CreateHomeDir",
    "CustomerProof",
    "Daemonize",
    "DisplayDotFiles",
    "DontResolve",
    "ExtCert",
    "ExtPassiveIP",
    "ForcePassiveIP",
    "FortunesFile",
    "FSCharset",
    "IPv4Only",
    "IPv6Only",
    "KeepAllFiles",
    "LimitRecursion",
    "MaxClientsNumber",
    "MaxClientsPerIP",
    "MaxDiskUsage",
    "MaxIdleTime",
    "MaxLoad",
    "MaxUsersPerIP",
    "MinUID",
    "MySQLConfigFile",
    "NATmode",
    "NoAnonymous",
    "NoChmod",
    "NoRename",
    "NoTruncate",
    "PAMAuthentication",
    "PassivePortRange",
    "PerUserLimits",
    "PGSQLConfigFile",
    "PIDFile",
    "ProhibitDotFilesRead",
    "ProhibitDotFilesWrite",
    "PureDB",
    "Quota",
    "SyslogFacility",
    "TLS",
    "TLSCipherSuite",
    "TrustedGID",
    "TrustedIP",
    "Umask",
    "UnixAuthentication",
    "UploadScript",
    "UserBandwidth",
    "UserRatio",
    "VhostIP",
    "VerboseLog",
    "WeLogin",
];

fn first_word(t: &str) -> &str {
    t.split(char::is_whitespace).next().unwrap_or("")
}

/// Detect a `pure-ftpd.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if KEYS.contains(&first_word(tr)) {
            n += 1;
        }
    }
    n >= 3
}

impl Pureftpd {
    /// Count directives. Returns `None` when the input does not look like
    /// a `pure-ftpd.conf`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if KEYS.contains(&first_word(tr)) {
                c.settings += 1;
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
        let b = b"# pure-ftpd\nChrootEveryone yes\nBrokenClientsCompatibility no\nMaxClientsNumber 50\nDaemonize yes\nMaxClientsPerIP 8\nVerboseLog no\nDisplayDotFiles yes\nAnonymousOnly no\nNoAnonymous yes\nSyslogFacility ftp\nPureDB /etc/pure-ftpd/pureftpd.pdb\nPAMAuthentication yes\nUnixAuthentication yes\nPassivePortRange 30000 50000\nTLS 2\n";
        assert!(detect(b));
        let c = Pureftpd::parse(b).unwrap();
        assert_eq!(c.settings, 14);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(
            b"anonymous_enable=NO\nlocal_enable=YES\nwrite_enable=YES\n"
        ));
        assert!(!detect(b"foo bar\n"));
        assert!(Pureftpd::parse(b"").is_none());
    }
}

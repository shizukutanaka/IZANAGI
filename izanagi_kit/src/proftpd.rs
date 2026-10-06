//! `proftpd.conf` (ProFTPD) census.
//!
//! Apache-style whitespace `Directive value` lines plus `<Name …>` /
//! `</Name>` context blocks (`<Global>`, `<VirtualHost>`, `<Anonymous>`,
//! `<Directory>`, `<Limit>`, `<IfModule>`, `<IfDefine>` …).
//!
//! ```rust
//! let p = b"ServerName \"ftp\"\nServerType standalone\nDefaultServer on\nPort 21\n<Global>\nUmask 022\n</Global>\n<Anonymous ~ftp>\nUser ftp\n</Anonymous>\n";
//! assert!(izanagi_kit::proftpd::detect(p));
//! let c = izanagi_kit::proftpd::Proftpd::parse(p).unwrap();
//! assert!(c.contexts >= 2);
//! ```

/// proftpd.conf census.
#[derive(Debug, Clone)]
pub struct Proftpd {
    /// `Directive value` lines matching a known proftpd directive.
    pub settings: usize,
    /// `<Context>` open/close lines.
    pub contexts: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// proftpd directives (first word of a directive line).
const KEYS: &[&str] = &[
    "AccessGrantMsg",
    "AllowAll",
    "AllowForeignAddress",
    "AllowOverwrite",
    "AllowRetrieveRestart",
    "AllowStoreRestart",
    "AnonRequirePassword",
    "AnonymousGroup",
    "AuthAliasOnly",
    "AuthGroupFile",
    "AuthPAM",
    "AuthUserFile",
    "Bind",
    "CommandBufferSize",
    "CreateHome",
    "DefaultAddress",
    "DefaultChdir",
    "DefaultRoot",
    "DefaultServer",
    "DefaultTransferMode",
    "DeferWelcome",
    "DeleteAbortedStores",
    "DenyAll",
    "DirFakeGroup",
    "DirFakeMode",
    "DirFakeUser",
    "DisplayConnect",
    "DisplayFileTransfer",
    "DisplayGoAway",
    "DisplayLogin",
    "DisplayQuit",
    "HiddenStor",
    "HideNoAccess",
    "HideUser",
    "IgnoreHidden",
    "IdentLookups",
    "Include",
    "LDAPAuthBinds",
    "LimitEVAL",
    "MasqueradeAddress",
    "MaxClients",
    "MaxClientsPerHost",
    "MaxConnectionAttempts",
    "MaxHostsPerUser",
    "MaxInstances",
    "MaxLoginAttempts",
    "MultilineRFC2228",
    "PassivePorts",
    "PathAllowFilter",
    "PathDenyFilter",
    "PersistentPasswd",
    "PidFile",
    "Port",
    "Protocol",
    "RATIO",
    "RLimitCPU",
    "RLimitMemory",
    "RLimitOpenFiles",
    "Ratios",
    "RequireValidShell",
    "RootLogin",
    "SQLAuthTypes",
    "SQLBackend",
    "SQLConnectInfo",
    "SabaothClass",
    "ScoreboardFile",
    "ServerAdmin",
    "ServerIdent",
    "ServerLog",
    "ServerName",
    "ServerType",
    "SyslogFacility",
    "SyslogLevel",
    "TCPAccessFiles",
    "TLSCACertificateFile",
    "TLSCertificateFile",
    "TLSCertificateKeyFile",
    "TLSEngine",
    "TLSLog",
    "TLSOptions",
    "TLSProtocol",
    "TLSRequired",
    "TLSRSACertificateFile",
    "TLSRSACertificateKeyFile",
    "TLSVerifyClient",
    "TimeoutIdle",
    "TimeoutLogin",
    "TimeoutNoTransfer",
    "TimeoutStalled",
    "TransferLog",
    "Umask",
    "User",
    "UserAlias",
    "UserDirRoot",
    "WtmpLog",
];

/// Context block names (`<Name>` … `</Name>`).
const CONTEXTS: &[&str] = &[
    "Anonymous",
    "Class",
    "Directory",
    "Global",
    "IfDefine",
    "IfModule",
    "Limit",
    "VirtualHost",
];

fn is_context(t: &str) -> bool {
    if let Some(inner) = t.strip_prefix('<') {
        if inner.starts_with('/') {
            return false;
        }
        let name = inner.split([' ', '\t', '>']).next().unwrap_or("");
        return CONTEXTS.contains(&name);
    }
    false
}

fn is_context_close(t: &str) -> bool {
    if let Some(inner) = t.strip_prefix("</") {
        let name = inner.split('>').next().unwrap_or("");
        return CONTEXTS.contains(&name);
    }
    false
}

fn first_word(t: &str) -> &str {
    t.split(char::is_whitespace).next().unwrap_or("")
}

/// Detect a `proftpd.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut keys = 0usize;
    let mut ctx = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_context(tr) || is_context_close(tr) {
            ctx += 1;
            continue;
        }
        let w = first_word(tr);
        if KEYS.contains(&w) {
            keys += 1;
        }
    }
    keys >= 2 || (keys >= 1 && ctx >= 2)
}

impl Proftpd {
    /// Count directives and context blocks. Returns `None` when the input
    /// does not look like a `proftpd.conf`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            contexts: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if is_context(tr) || is_context_close(tr) {
                c.contexts += 1;
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
        let b = b"# proftpd\nServerName \"ftp site\"\nServerType standalone\nDefaultServer on\nPort 21\nUmask 022\nMaxInstances 30\nUser nobody\nGroup nogroup\nDefaultRoot ~\n<Global>\nAllowOverwrite on\n</Global>\n<Anonymous ~ftp>\nUser ftp\n<Limit LOGIN>\nDenyAll\n</Limit>\n</Anonymous>\n";
        assert!(detect(b));
        let c = Proftpd::parse(b).unwrap();
        assert_eq!(c.settings, 11);
        assert_eq!(c.contexts, 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[http]\nport = 80\n"));
        assert!(!detect(b"foo bar baz\n"));
        assert!(Proftpd::parse(b"").is_none());
    }
}

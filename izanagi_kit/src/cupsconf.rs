//! `cupsd.conf` (CUPS 印刷サーバ) 検出モジュール。
//!
//! CUPS の設定は Apache 風ディレクティブ形式で、`Listen`/`Port`/
//! `ServerName`/`ServerAdmin`/`LogLevel`/`MaxJobs`/
//! `PreserveJobHistory`/`Browsing`/`DefaultShared`/`WebInterface`/
//! `SystemGroup`/`PageLogFormat`/`ErrorLog`/`AccessLog`/`PageLog`/
//! `MaxRequestSize`/`Timeout`/`KeepAlive`/`DefaultAuthType`/
//! `JobRetryInterval`/`Filter`/`Policy`/`DefaultPolicy`/
//! `PrintcapFormat`/`RootCertDuration` と `<Location>`/`<Policy>`/
//! `<Limit>`/`Order`/`Allow`/`Deny`/`Require`/`AuthType`/`Satisfy`
//! ブロックで構成される。
//!
//! ```
//! let b = b"LogLevel warn\n\
//!           MaxJobs 500\n\
//!           Listen localhost:631\n\
//!           Browsing Off\n\
//!           <Location />\n\
//!           Order allow,deny\n\
//!           </Location>\n";
//! let c = izanagi_kit::cupsconf::parse(b);
//! assert!(izanagi_kit::cupsconf::detect(b));
//! assert_eq!(c.directives, 5);
//! ```

const DIRECTIVES: &[&str] = &[
    "AccessLog",
    "Allow",
    "AuthType",
    "AutoPurgeJobs",
    "Browsing",
    "BrowseInterval",
    "BrowseLocalProtocols",
    "BrowseOrder",
    "BrowsePoll",
    "BrowseProtocols",
    "BrowseRemoteProtocols",
    "BrowseShortNames",
    "BrowseTimeout",
    "BrowseUpdate",
    "BrowseWebIF",
    "Classification",
    "ClassifyOverride",
    "CreateSelfSignedCerts",
    "DataDir",
    "DefaultAuthType",
    "DefaultCharset",
    "DefaultEncryption",
    "DefaultLanguage",
    "DefaultPaperSize",
    "DefaultPolicy",
    "DefaultShared",
    "Deny",
    "DirtyCleanInterval",
    "DocumentRoot",
    "Encryption",
    "ErrorLog",
    "FatalErrors",
    "FileDevice",
    "Filter",
    "FontPath",
    "GSSServiceName",
    "HideImplicitMembers",
    "HostNameLookups",
    "ImplicitAnyClasses",
    "ImplicitClasses",
    "Include",
    "JobKillDelay",
    "JobKillTime",
    "JobRetryInterval",
    "JobRetryLimit",
    "KeepAlive",
    "KeepAliveTimeout",
    "Limit",
    "LimitExcept",
    "Listen",
    "ListenBackLog",
    "LogDebugHistory",
    "LogFilePerm",
    "LogLevel",
    "LogMessage",
    "LogTimeFormat",
    "MaxActiveJobs",
    "MaxClients",
    "MaxClientsPerHost",
    "MaxCopies",
    "MaxHoldTime",
    "MaxJobTime",
    "MaxJobs",
    "MaxJobsPerClass",
    "MaxJobsPerPrinter",
    "MaxJobsPerUser",
    "MaxLogSize",
    "MaxRequestSize",
    "MaxSubscriptions",
    "MaxSubscriptionsPerJob",
    "MaxSubscriptionsPerPrinter",
    "MaxSubscriptionsPerUser",
    "MultipleOperationTimeout",
    "Order",
    "PageLog",
    "PageLogFormat",
    "PassEnv",
    "Policy",
    "Port",
    "PreserveJobFiles",
    "PreserveJobHistory",
    "Printcap",
    "PrintcapFormat",
    "ReloadTimeout",
    "RemoteRoot",
    "RequestRoot",
    "Require",
    "RIPCache",
    "RootCertDuration",
    "Satisfy",
    "ServerAdmin",
    "ServerAlias",
    "ServerBin",
    "ServerCertificate",
    "ServerKeychain",
    "ServerKeychainPassphrase",
    "ServerName",
    "ServerRoot",
    "ServerTokens",
    "SetEnv",
    "SMBServerName",
    "SSLListen",
    "SSLPort",
    "SyncOnClose",
    "SystemGroup",
    "TempDir",
    "Timeout",
    "User",
    "UseIPv6",
    "UseIPv4",
    "WebInterface",
];

fn is_directive(t: &str) -> bool {
    let w = t.split_whitespace().next().unwrap_or("");
    DIRECTIVES.contains(&w)
}

fn is_block(t: &str) -> bool {
    t.starts_with("<Location")
        || t.starts_with("</Location")
        || t.starts_with("<Policy")
        || t.starts_with("</Policy")
        || t.starts_with("<Limit")
        || t.starts_with("</Limit")
}

/// `b` が cupsd.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut dirs = 0usize;
    let mut blocks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_block(tr) {
            blocks += 1;
        } else if is_directive(tr) {
            dirs += 1;
        }
    }
    (blocks >= 1 && dirs >= 2) || dirs >= 3
}

/// cupsd.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct CupsConf {
    /// 既知ディレクティブ行数。
    pub directives: usize,
    /// `<Location>`/`<Policy>`/`<Limit>` 系ブロック行数。
    pub blocks: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を cupsd.conf として統計する。
pub fn parse(b: &[u8]) -> CupsConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = CupsConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_block(tr) {
            c.blocks += 1;
        } else if is_directive(tr) {
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
        let b = b"LogLevel warn\nMaxJobs 500\nListen localhost:631\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 3);
    }

    #[test]
    fn detects_block() {
        let b = b"LogLevel warn\nMaxJobs 500\n<Location />\nOrder allow,deny\n</Location>\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.blocks, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"key = value\nfoo = bar\nbaz = quux\n"));
        assert!(!detect(b"LogLevel warn\nMaxJobs 500\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}

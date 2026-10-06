//! Tinyproxy `tinyproxy.conf` census.
//!
//! Tinyproxy config is flat PascalCase `Directive Value` lines
//! (`#` comments): `Port`, `Listen`, `Timeout`, `LogLevel`,
//! `PidFile`, `MaxClients`, `MinSpareServers`, `MaxSpareServers`,
//! `StartServers`, `StatHost`, `ViaProxyName`, `ConnectPort`,
//! `ReverseOnly`, `ReversePath`, `Allow`, `Deny`, `Filter*`,
//! `BasicAuth`, `AddHeader`, `Upstream`, `NoUpstream`.
//!
//! ```rust
//! let k = b"User tinyproxy\nGroup tinyproxy\nPort 8888\nMaxClients 100\nMinSpareServers 5\nStatHost tinyproxy.stats\nAllow 127.0.0.1\n";
//! assert!(izanagi_kit::tinyproxy::detect(k));
//! ```

/// Tinyproxy config census.
#[derive(Debug, Clone)]
pub struct Tinyproxy {
    /// `Directive Value` lines.
    pub settings: usize,
    /// recognised directives present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "MaxClients",
    "MinSpareServers",
    "MaxSpareServers",
    "StartServers",
    "MaxRequestsPerChild",
    "StatHost",
    "StatFile",
    "ViaProxyName",
    "ViaOff",
    "DisableViaHeader",
    "ConnectPort",
    "ReverseOnly",
    "ReversePath",
    "ReverseBaseURL",
    "XTinyproxy",
    "FilterURLs",
    "FilterExtended",
    "FilterCaseSensitive",
    "FilterDefaultDeny",
    "AddHeader",
    "Upstream",
    "NoUpstream",
    "BasicAuth",
];

const WEAK: &[&str] = &[
    "User",
    "Group",
    "Port",
    "Listen",
    "Bind",
    "BindSame",
    "Timeout",
    "ErrorFile",
    "DefaultErrorFile",
    "PidFile",
    "LogFile",
    "LogLevel",
    "LogColor",
    "Syslog",
    "Allow",
    "Deny",
    "Anonymous",
    "Filter",
    "LogHost",
    "DNSServers",
];

fn directive(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    let k = s.split([' ', '\t']).next()?.trim();
    if k.is_empty() {
        None
    } else {
        Some(k)
    }
}

/// Detect a `tinyproxy.conf` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // PascalCase pool directives (`MinSpareServers`/`StatHost`/
    // `ViaProxyName`/…) are tinyproxy-exclusive.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = directive(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Tinyproxy {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            keys: 0,
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
            c.settings += 1;
            if let Some(k) = directive(line) {
                if STRONG.contains(&k) || WEAK.contains(&k) {
                    c.keys += 1;
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
        let b = b"User tinyproxy\nGroup tinyproxy\nPort 8888\nMaxClients 100\nMinSpareServers 5\nStatHost tinyproxy.stats\nAllow 127.0.0.1\n";
        assert!(detect(b));
        let c = Tinyproxy::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"User x\nGroup y\nPort 80\n"));
        assert!(!detect(b"# MaxClients 100\n# MinSpareServers 5\nPort 80\n"));
    }
}

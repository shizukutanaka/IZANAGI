//! net-snmp `snmpd.conf` / `snmptrapd.conf` / `snmp.conf` census.
//!
//! Whitespace `directive value…` lines (`agentaddress`, `rocommunity`,
//! `rouser`, `com2sec`, `view`, `access`, `syslocation`, `trapsink`,
//! `monitor`, `extend`, `createUser` …). `#` comments.
//! Companion to `crate::snmp` (wire packet parser).
//!
//! ```rust
//! let s = b"agentaddress udp:161\nrocommunity public\nsyslocation rack1\nsyscontact ops@ex.com\nrouser monitor ro\n";
//! assert!(izanagi_kit::snmpdconf::detect(s));
//! let c = izanagi_kit::snmpdconf::Snmpdconf::parse(s).unwrap();
//! assert_eq!(c.settings, 5);
//! ```

/// snmpd.conf census.
#[derive(Debug, Clone)]
pub struct Snmpdconf {
    /// `directive value` lines matching a known net-snmp directive.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// net-snmp directives (first word; covers snmpd/snmptrapd/snmp configs).
const KEYS: &[&str] = &[
    "access",
    "agentaddress",
    "agentgroup",
    "agentuser",
    "alarm",
    "aslm",
    "authcommunity",
    "authkey",
    "authtrapenable",
    "com2sec",
    "com2sec6",
    "comsecgroup",
    "cpu",
    "createUser",
    "createuser",
    "defAuthType",
    "defCommunity",
    "defPrivPassphrase",
    "defPrivType",
    "defSecurityLevel",
    "defSecurityName",
    "defVersion",
    "defaultMonitors",
    "disk",
    "dlmod",
    "dontLogTCPWrappersConnects",
    "engineID",
    "exec",
    "extend",
    "file",
    "fix",
    "group",
    "iquerySecName",
    "includeAllDisks",
    "informsink",
    "injectHandler",
    "interface",
    "internalSecName",
    "leave_pidfile",
    "linkUpDownNotifications",
    "load",
    "localCert",
    "logmatch",
    "master",
    "maxGetbulkRepeats",
    "maxGetbulkResponses",
    "mibdirs",
    "mibfiles",
    "monitor",
    "netstat",
    "noAddrChange",
    "noTokenWarnings",
    "notification",
    "notifyFile",
    "override",
    "pass",
    "pass_persist",
    "pcpu",
    "persistentDir",
    "proc",
    "procfix",
    "proxy",
    "proxyCerts",
    "rmon",
    "rocommunity",
    "rocommunity6",
    "rouser",
    "rwcommunity",
    "rwcommunity6",
    "rwuser",
    "setserialno",
    "smuxpeer",
    "smuxsocket",
    "snmpd",
    "snmptrapdaddr",
    "storageUseNFS",
    "swap",
    "syscontact",
    "sysdescr",
    "syslocation",
    "sysname",
    "sysObjectID",
    "sysservices",
    "targetAddr",
    "targetParams",
    "temp",
    "timer",
    "trapsess",
    "trapsink",
    "trap2sink",
    "usmUser",
    "vacm",
    "view",
];

fn first_word(t: &str) -> &str {
    t.split(char::is_whitespace).next().unwrap_or("")
}

/// Detect an `snmpd.conf`-style file.
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

impl Snmpdconf {
    /// Count directives. Returns `None` when the input does not look like
    /// a net-snmp config.
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
        let b = b"# snmpd\nagentaddress udp:161,udp6:[::1]:161\nsyslocation server room\nsyscontact admin <ops@ex.com>\nrocommunity public 127.0.0.1\nrwcommunity private 10.0.0.0/8\nrouser monitor ro\ncreateUser ops SHA pass AES pass\nview systemonly included .1.3.6.1.2.1.1\ndisk / 10000\nload 12 10 5\nproc mountd\nextend test1 /bin/echo hi\ntrapsink localhost public\nmaster agentx\n";
        assert!(detect(b));
        let c = Snmpdconf::parse(b).unwrap();
        assert_eq!(c.settings, 14);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"server {\n  listen 80;\n}\n"));
        assert!(!detect(b"bind 127\nport 6379\n"));
        assert!(Snmpdconf::parse(b"").is_none());
    }
}

//! `/etc/nsswitch.conf` Name Service Switch configuration format.
//!
//! nsswitch.conf lines declare a database then colon-separated sources:
//! `passwd: files sss`, with optional bracket action overrides
//! `[NOTFOUND=return]`, `[UNAVAIL=continue]`, `[TRYAGAIN=forever]`.
//!
//! ```
//! let b = concat!(
//!     "passwd: files sss\n",
//!     "group: files\n",
//!     "hosts: files [NOTFOUND=return] dns mdns4_minimal\n",
//!     "aliases: files\n"
//! ).as_bytes();
//! assert!(izanagi_kit::nsswitch::detect(b));
//! let c = izanagi_kit::nsswitch::Nsswitch::parse(b).unwrap();
//! assert_eq!(c.databases, 4);
//! assert_eq!(c.actions, 1);
//! ```

/// Parsed nsswitch.conf summary.
#[derive(Debug, Clone)]
pub struct Nsswitch {
    /// Database lines (`passwd`, `group`, `hosts`, `services`, `netgroup`, `networks`, `protocols`, `publickey`, `rpc`, `ethers`, `aliases`, `automount`, `bootparams`, `initgroups`, `netmasks`, `sudoers`, `shadow`, `gshadow`).
    pub databases: usize,
    /// Source tokens after the colon (`files`, `dns`, `mdns4_minimal`, `sss`, `nis`, `db`, `compat`, `systemd`, `resolve`, `wins`, `myhostname`, `mymachines`, `nisplus`, `ldap`).
    pub sources: usize,
    /// Bracket action overrides `[ACTION=continue]`/`[ACTION=return]`/`[ACTION=forever]`/`[ACTION=merge]`.
    pub actions: usize,
    /// `?` optional-result bracket entries.
    pub optional: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const DBS: &[&str] = &[
    "passwd",
    "group",
    "hosts",
    "services",
    "netgroup",
    "networks",
    "protocols",
    "publickey",
    "rpc",
    "ethers",
    "aliases",
    "automount",
    "bootparams",
    "initgroups",
    "netmasks",
    "sudoers",
    "shadow",
    "gshadow",
    "shells",
    "tsm_fabrics",
];
const SOURCES: &[&str] = &[
    "files",
    "dns",
    "mdns4_minimal",
    "mdns4",
    "mdns6_minimal",
    "mdns6",
    "mdns_minimal",
    "mdns",
    "sss",
    "nis",
    "db",
    "compat",
    "systemd",
    "resolve",
    "wins",
    "myhostname",
    "mymachines",
    "nisplus",
    "ldap",
    "tacplus",
    "radius",
    "winbind",
    "cache",
    "extrausers",
    "various",
    "dns [NOTFOUND=return]",
];

/// Whether the buffer looks like nsswitch.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut score = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if let Some(colon) = tr.find(':') {
            let db = tr[..colon].trim();
            if DBS.contains(&db) {
                score += 2;
            }
        }
    }
    score >= 2
}

impl Nsswitch {
    /// Parses an nsswitch.conf summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            databases: 0,
            sources: 0,
            actions: 0,
            optional: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            let Some(colon) = tr.find(':') else { continue };
            if !DBS.contains(&tr[..colon].trim()) {
                continue;
            }
            c.databases += 1;
            let rest = &tr[colon + 1..];
            for tok in rest.split_whitespace() {
                if tok.starts_with('[') {
                    if tok.contains('=') {
                        if tok.ends_with('?') {
                            c.optional += 1;
                        } else {
                            c.actions += 1;
                        }
                    }
                } else if SOURCES.contains(&tok) {
                    c.sources += 1;
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
    fn parses_nsswitch() {
        let b = concat!(
            "passwd: files sss\n",
            "group: files\n",
            "hosts: files [NOTFOUND=return] dns mdns4_minimal\n",
            "aliases: files\n",
            "netgroup: nis\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Nsswitch::parse(b).unwrap();
        assert_eq!(c.databases, 5);
        assert_eq!(c.actions, 1);
        assert_eq!(c.sources, 8);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"foo: bar\nbaz: qux\n"));
        assert!(Nsswitch::parse(b"x").is_none());
    }
}

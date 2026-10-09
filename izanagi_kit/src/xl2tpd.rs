//! xl2tpd `xl2tpd.conf`/`l2tpd.conf` census.
//!
//! `[global]`/`[lns <name>]`/`[lac <name>]`/`[ppp]`/`[l2tp]`/
//! `[l2tp-control]`/`[l2tp-ipsec]` sections with `key = value`
//! options (`port`, `auth file`, `access control`,
//! `ipsec saref`, `force userspace`, `listen-addr`,
//! `ip range`, `local ip`, `remote ip`, `assign ip`,
//! `require chap`, `refuse chap`, `refuse pap`,
//! `require authentication`, `name`, `ppp debug`,
//! `pppoptfile`, `call rws`, `tunnel rws`, `flow bit`,
//! `challenge`, `rx bps`, `tx bps`, `length bit`,
//! `hidden bit`, `redial`, `redial timeout`, `idle`,
//! `lns`, `lac`, `hostname`, `pppmtu`, `pass peer`,
//! `routing domain`, `defaultroute`, `exclusive`,
//! `autodial`, `tunnel name`, `bps`, `max redials`,
//! `tunnel setup timeout`, `disconnect timeout`,
//! `connect speed`, `avps`, `ppp version`,
//! `send_gratuitous_arp`, `debug avp`, `debug network`,
//! `debug packet`, `debug state`, `debug tunnel`).
//!
//! ```rust
//! let x = "[global]\nport = 1701\nauth file = /etc/xl2tpd/l2tp-secrets\n[lns default]\nip range = 10.1.0.10-10.1.0.100\nlocal ip = 10.1.0.1\n";
//! let c = izanagi_kit::xl2tpd::Xl2tpd::parse(x.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

use crate::textutil::strip_bom;
/// xl2tpd.conf census.
#[derive(Debug, Clone)]
pub struct Xl2tpd {
    /// `[…]` section headers.
    pub sections: usize,
    /// `key = value` lines.
    pub settings: usize,
    /// Recognised xl2tpd option names.
    pub named: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "port",
    "auth file",
    "access control",
    "ipsec saref",
    "force userspace",
    "listen-addr",
    "ip range",
    "local ip",
    "remote ip",
    "assign ip",
    "require chap",
    "refuse chap",
    "refuse pap",
    "require authentication",
    "name",
    "hostname",
    "ppp debug",
    "pppoptfile",
    "call rws",
    "tunnel rws",
    "flow bit",
    "challenge",
    "rx bps",
    "tx bps",
    "bps",
    "length bit",
    "hidden bit",
    "redial",
    "redial timeout",
    "idle",
    "lns",
    "lac",
    "pppmtu",
    "pass peer",
    "routing domain",
    "defaultroute",
    "exclusive",
    "autodial",
    "tunnel name",
    "max redials",
    "tunnel setup timeout",
    "disconnect timeout",
    "connect speed",
    "avps",
    "ppp version",
    "send_gratuitous_arp",
    "debug avp",
    "debug network",
    "debug packet",
    "debug state",
    "debug tunnel",
    "debug",
    "lns specific",
    "lac specific",
    "nis server",
    "dns server",
    "wins server",
    "pre-shared key",
    "keepalive",
    "lac-pppd-opts",
    "use kernel",
    "load-module",
    "stateful",
];

/// Detect xl2tpd.conf content.
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
        if s.is_empty() || s.starts_with(';') || s.starts_with('#') {
            continue;
        }
        if s.starts_with('[') && s.ends_with(']') {
            let name = s[1..s.len() - 1].split_whitespace().next().unwrap_or("");
            if matches!(
                name,
                "global" | "lns" | "lac" | "ppp" | "l2tp" | "l2tp-control" | "l2tp-ipsec"
            ) {
                hits += 1;
                continue;
            }
        }
        if let Some(eq) = s.find('=') {
            let key = s[..eq].trim();
            if KEYS.contains(&key) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

impl Xl2tpd {
    /// Census an xl2tpd.conf buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sections: 0,
            settings: 0,
            named: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') || s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                continue;
            }
            if let Some(eq) = s.find('=') {
                let key = s[..eq].trim();
                if !key.is_empty() {
                    c.settings += 1;
                    if KEYS.contains(&key) {
                        c.named += 1;
                    }
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
    fn detects_conf() {
        let b = b"[global]\nport = 1701\nauth file = x\n";
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
            "; xl2tpd\n",
            "[global]\n",
            "port = 1701\n",
            "auth file = /etc/xl2tpd/l2tp-secrets\n",
            "access control = no\n",
            "debug avp = yes\n",
            "debug network = yes\n",
            "debug packet = yes\n",
            "debug state = yes\n",
            "debug tunnel = yes\n",
            "[lns default]\n",
            "exclusive = no\n",
            "ip range = 10.1.0.10-10.1.0.100\n",
            "local ip = 10.1.0.1\n",
            "length bit = yes\n",
            "refuse pap = yes\n",
            "require chap = yes\n",
            "require authentication = yes\n",
            "name = LinuxVPNserver\n",
            "ppp debug = yes\n",
            "pppoptfile = /etc/ppp/options.xl2tpd\n",
            "call rws = 4\n",
            "tunnel rws = 4\n",
            "flow bit = yes\n",
        );
        let c = Xl2tpd::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.settings, 21);
        assert!(c.named >= 20);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

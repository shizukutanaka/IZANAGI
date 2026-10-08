//! WireGuard `wg.conf`/`wg-quick.conf` census.
//!
//! `[Interface]` (local node) and `[Peer]` sections; `key = value`
//! options: `PrivateKey`, `Address`, `ListenPort`, `DNS`, `MTU`,
//! `Table`, `PreUp`, `PostUp`, `PreDown`, `PostDown`, `SaveConfig`,
//! `FwMark`, `PublicKey`, `PresharedKey`, `AllowedIPs`, `Endpoint`,
//! `PersistentKeepalive`.
//!
//! ```rust
//! let w = "[Interface]\nPrivateKey = k\nAddress = 10.0.0.1/24\nListenPort = 51820\n[Peer]\nPublicKey = p\nAllowedIPs = 10.0.0.2/32\n";
//! let c = izanagi_kit::wireguard::Wireguard::parse(w.as_bytes()).unwrap();
//! assert_eq!(c.peers, 1);
//! assert_eq!(c.settings, 5);
//! ```

/// wireguard conf census.
#[derive(Debug, Clone)]
pub struct Wireguard {
    /// `[Interface]`/`[Peer]`/other section headers.
    pub sections: usize,
    /// `[Peer]` sections specifically.
    pub peers: usize,
    /// `key = value` lines.
    pub settings: usize,
    /// Recognised wg-quick keys.
    pub named: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "PrivateKey",
    "Address",
    "ListenPort",
    "DNS",
    "MTU",
    "Table",
    "PreUp",
    "PostUp",
    "PreDown",
    "PostDown",
    "SaveConfig",
    "FwMark",
    "PublicKey",
    "PresharedKey",
    "AllowedIPs",
    "Endpoint",
    "PersistentKeepalive",
    "Jc",
    "Jmin",
    "Jmax",
    "S1",
    "S2",
    "S3",
    "S4",
    "H1",
    "H2",
    "H3",
    "H4",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect wireguard conf content.
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
        if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
            continue;
        }
        if s == "[Interface]" || s == "[Peer]" {
            hits += 1;
            continue;
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

impl Wireguard {
    /// Census a wireguard conf buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sections: 0,
            peers: 0,
            settings: 0,
            named: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                if s == "[Peer]" {
                    c.peers += 1;
                }
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
        let b = b"[Interface]\nPrivateKey = x\nAddress = 10.0.0.1/24\n";
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
            "# wireguard\n",
            "[Interface]\n",
            "PrivateKey = aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890abc=\n",
            "Address = 10.0.0.1/24\n",
            "ListenPort = 51820\n",
            "DNS = 1.1.1.1, 8.8.8.8\n",
            "MTU = 1420\n",
            "PreUp = iptables -A FORWARD -i wg0 -j ACCEPT\n",
            "PostDown = iptables -D FORWARD -i wg0 -j ACCEPT\n",
            "[Peer]\n",
            "PublicKey = zYxWvUtSrQpOnMlKjIhGfEdCbA1234567890zyx=\n",
            "PresharedKey = pppppppppppppppppppppppppppppppppppppp=\n",
            "AllowedIPs = 10.0.0.2/32, fd00::2/128\n",
            "Endpoint = vpn.example.org:51820\n",
            "PersistentKeepalive = 25\n",
            "[Peer]\n",
            "PublicKey = qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq=\n",
            "AllowedIPs = 10.0.0.3/32\n",
        );
        let c = Wireguard::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.peers, 2);
        assert_eq!(c.settings, 14);
        assert_eq!(c.named, 14);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

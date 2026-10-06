//! `yggdrasil.conf` (Yggdrasil Network) 検出モジュール。
//!
//! Yggdrasil-overlay の設定は JSON で、`"Peers"`/`"InterfacePeers"`/
//! `"Listen"`/`"AdminListen"`/`"MulticastInterfaces"`/
//! `"AllowedPublicKeys"`/`"IfName"`/`"IfMTU"`/`"NodeInfoPrivacy"`/
//! `"NodeInfo"`/`"PrivateKey"`/`"PublicKey"`/`"AllowedDomains"` 等の
//! キーを持つ。
//!
//! ```
//! let b = br#"{
//!   "Peers": [
//!     "tls://a.b.c.d:443"
//!   ],
//!   "IfName": "auto",
//!   "NodeInfoPrivacy": false,
//!   "PrivateKey": "aaaa"
//! }
//! "#;
//! let c = izanagi_kit::yggdrasil::parse(b);
//! assert!(izanagi_kit::yggdrasil::detect(b));
//! assert_eq!(c.keys, 4);
//! ```

const KEYS: &[&str] = &[
    "AdminListen",
    "AllowedDomains",
    "AllowedPublicKeys",
    "IfMTU",
    "IfName",
    "InterfacePeers",
    "Listen",
    "MulticastInterfaces",
    "NodeInfo",
    "NodeInfoPrivacy",
    "Peers",
    "PrivateKey",
    "PublicKey",
    "TunnelRouting",
];

const HINTS: &[&str] = &[
    "InterfacePeers",
    "MulticastInterfaces",
    "AllowedPublicKeys",
    "NodeInfoPrivacy",
    "IfName",
    "PrivateKey",
];

/// 行内の最初の `"key":` を返す。
fn jkey(line: &str) -> Option<&str> {
    let s = line.trim_start();
    let s = s.strip_prefix('"')?;
    let e = s.find('"')?;
    let key = &s[..e];
    if s[e + 1..].trim_start().starts_with(':') {
        Some(key)
    } else {
        None
    }
}

/// `b` が yggdrasil.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut hints = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                keys += 1;
                if HINTS.contains(&k) {
                    hints += 1;
                }
            }
        }
    }
    (hints >= 1 && keys >= 2) || keys >= 4
}

/// yggdrasil.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct YggdrasilConf {
    /// 既知キー行数。
    pub keys: usize,
    /// Yggdrasil 特有キー行数。
    pub hint_keys: usize,
}

/// `b` を yggdrasil.conf として統計する。
pub fn parse(b: &[u8]) -> YggdrasilConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = YggdrasilConf::default();
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                c.keys += 1;
                if HINTS.contains(&k) {
                    c.hint_keys += 1;
                }
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"{
  "Peers": [],
  "IfName": "auto",
  "PrivateKey": "x"
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_many_keys() {
        let b = br#"{
  "Listen": "tcp://[::]:0",
  "AdminListen": "unix:///var/run/yggdrasil.sock",
  "NodeInfo": {},
  "PublicKey": "y"
}
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\"Peers\": [\"x\"], \"Listen\": \"y\"}"));
        assert!(!detect(b"{\"name\": \"x\", \"version\": 1}"));
        assert!(!detect(b"Peers=x\nIfName=auto\nPrivateKey=y\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

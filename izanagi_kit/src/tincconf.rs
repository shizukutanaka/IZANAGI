//! Tinc VPN `tinc.conf` + `hosts/*` ファイルの検出と構造カウント。
//!
//! PascalCase フラットキー(`Name`/`Address`/`Port`/`Device`/`Interface`/
//! `Subnet`/`ConnectTo`/`AddressFamily`/`BindToAddress`/`Cipher`/`Compression`/
//! `Digest`/`Forwarding`/`Hostnames`/`Mode`/`PingInterval`/`PMTUDiscovery`/
//! `PrivateKeyFile`/`PublicKey*`/`TCPOnly`/`UDPRcvBuf`/`Weight` 等)を識別する。
//!
//! ```
//! let c = izanagi_kit::tincconf::parse(
//!     b"Name = alpha\nDevice = /dev/tun\nConnectTo = beta\nPort = 655\n").unwrap();
//! assert_eq!(c.options, 4);
//! assert!(izanagi_kit::tincconf::detect(b"Name = alpha\nMode = router\n"));
//! ```

/// 既知 PascalCase キー。
const KEYS: &[&str] = &[
    "Address",
    "AddressCacheTime",
    "AddressFamily",
    "AllowSubnet",
    "AutoConnect",
    "BindToAddress",
    "BindToInterface",
    "Broadcast",
    "BroadcastSubnet",
    "Cipher",
    "ClampMSS",
    "Compression",
    "ConnectTo",
    "Device",
    "DeviceStandby",
    "DeviceType",
    "Digest",
    "DirectOnly",
    "Ed25519PrivateKeyFile",
    "Ed25519PublicKey",
    "Ed25519PublicKeyFile",
    "ExperimentalProtocol",
    "Forwarding",
    "Hostnames",
    "Ifconfig",
    "IffOneQueue",
    "Interface",
    "InterfaceMetric",
    "KeyExpire",
    "LocalDiscovery",
    "LocalDiscoveryAddress",
    "MACExpire",
    "MACLength",
    "MaxOutputBufferSize",
    "MaxTimeout",
    "Mode",
    "Name",
    "PingInterval",
    "PingTimeout",
    "PMTUDiscovery",
    "Port",
    "PriorityInheritance",
    "PrivateKey",
    "PrivateKeyFile",
    "ProcessPriority",
    "PublicKey",
    "PublicKeyFile",
    "Proxy",
    "ReplayWindow",
    "StrictSubnets",
    "Subnet",
    "TCPOnly",
    "UDPDiscovery",
    "UDPDiscoveryKeepaliveInterval",
    "UDPDiscoveryTimeout",
    "UDPRcvBuf",
    "UDPSndBuf",
    "VDEGroup",
    "Weight",
];

/// Tinc 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `Key = value` 既知代入。
    pub options: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 未知 `Key =` 代入。
    pub unknown: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行が `Key = value` か(キーを返す; PascalCase のみ)。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find('=')?;
    let k = t[..p].trim();
    if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric()) {
        None
    } else {
        Some(k)
    }
}

/// b が tinc.conf かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| kv_key(l.trim()).is_some_and(|k| KEYS.contains(&k)))
        .count()
        >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        comments: 0,
        unknown: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if let Some(k) = kv_key(t) {
            if KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.unknown += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# tinc\nName = alpha\nDevice = /dev/net/tun\nAddressFamily = ipv4\nInterface = tinc0\nConnectTo = beta\nConnectTo = gamma\nPort = 655\nPMTUDiscovery = yes\nCipher = aes-256-cbc\n";

    #[test]
    fn tincconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 9);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_tinc() {
        assert!(!detect(b"foo = bar\n"));
        assert!(!detect(b"[section]\nkey = value\n"));
    }
}

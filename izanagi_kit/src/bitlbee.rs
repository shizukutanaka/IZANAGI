//! `bitlbee.conf` 検出モジュール。
//!
//! BitlBee の設定は `[defaults]` セクションの INI 形式で、
//! `RunMode`/`DaemonPort`/`User`/`ConfigDir`/`VHostName`/
//! `AuthPassword` 等のキーが使われる。
//!
//! ```
//! let b = br#"[settings]
//! RunMode = ForkDaemon
//! DaemonPort = 6667
//! User = bitlbee
//! ConfigDir = /var/lib/bitlbee
//! VHostName = irc.example.net
//! "#;
//! let c = izanagi_kit::bitlbee::parse(b);
//! assert!(izanagi_kit::bitlbee::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "AuthPassword",
    "Bind",
    "CAfile",
    "ClientInterface",
    "ConfigDir",
    "DaemonHost",
    "DaemonPort",
    "ForkingPort",
    "HostName",
    "IRCBase",
    "IRCPort",
    "MotdFile",
    "PingInterval",
    "PingTimeOut",
    "Proxy",
    "RunMode",
    "User",
    "VHostName",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with(';')
}

fn section_name(t: &str) -> Option<&str> {
    if !(t.starts_with('[') && t.ends_with(']')) {
        return None;
    }
    Some(&t[1..t.len() - 1])
}

fn key_present(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が bitlbee.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut saw_section = false;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if section_name(tr).is_some() {
            saw_section = true;
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            keys += 1;
        }
    }
    keys >= 3 && saw_section
}

/// bitlbee.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct BitlbeeConf {
    /// セクション行数（`[defaults]` 等）。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を bitlbee.conf として統計する。
pub fn parse(b: &[u8]) -> BitlbeeConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = BitlbeeConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if section_name(tr).is_some() {
            c.sections += 1;
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"[settings]
RunMode = ForkDaemon
DaemonPort = 6667
User = bitlbee
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_defaults() {
        let b = br#"[defaults]
RunMode = Daemon
User = bitlbee
ConfigDir = /var/lib/bitlbee
VHostName = irc.example.net
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 4);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"RunMode = x\nUser = y\nDaemonPort = 1\n"));
        assert!(!detect(b"[foo]\nRunMode = x\n"));
        assert!(!detect(b"[settings]\nfoo = 1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

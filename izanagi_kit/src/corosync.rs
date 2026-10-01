//! Corosync `corosync.conf` / `corosync.conf.d/*.conf` の解析。
//!
//! `totem`/`nodelist`/`node`/`logging`/`quorum`/`resources`/`event`/`qb`/`amf`/
//! `knet`/`interface`/`member` 等の中括弧ブロックと `key: value` 行を検出し、
//! セクション・キー・リング/ノード情報を整数で返す。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::corosync;
//!
//! let text = br#"totem {
//!     version: 2
//!     cluster_name: mycluster
//! }
//! nodelist {
//!     node {
//!         ring0_addr: 192.168.1.1
//!         name: node1
//!     }
//! }
//! quorum {
//!     provider: corosync_votequorum
//! }
//! "#;
//!
//! assert!(corosync::detect(text));
//! let c = corosync::parse(text).unwrap();
//! assert_eq!(c.nodes, 1);
//! ```

/// corosync.conf の既知トップセクション。
const KNOWN_SECTIONS: &[&str] = &[
    "totem",
    "nodelist",
    "node",
    "logging",
    "quorum",
    "resources",
    "event",
    "qb",
    "amf",
    "knet",
    "interface",
    "member",
    "cgroup",
    "uidgid",
    "crypto",
    "system",
    "compatibility",
    "aisexec",
    "service",
    "monitor",
];

/// corosync.conf の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 中括弧ブロック開始数 (`totem {` 等)。
    pub blocks: usize,
    /// 既知セクション名でのブロック開始数。
    pub known_sections: usize,
    /// `key: value` 行数。
    pub key_values: usize,
    /// `node {` ブロック数 (nodelist 内ホスト)。
    pub nodes: usize,
    /// `ring0_addr`/`knet_link`/`ring*_addr`/`ipaddr` 系アドレス行数。
    pub addresses: usize,
    /// `name:`/`nodeid:` 行数。
    pub names: usize,
}

/// `b` が corosync.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.known_sections >= 1 && c.key_values >= 1
}

/// corosync.conf を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        blocks: 0,
        known_sections: 0,
        key_values: 0,
        nodes: 0,
        addresses: 0,
        names: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "}" {
            continue;
        }
        if let Some(head) = line.strip_suffix('{') {
            let name = head.split_whitespace().next().unwrap_or("");
            counts.blocks += 1;
            saw_any = true;
            if name == "node" {
                counts.nodes += 1;
            }
            if KNOWN_SECTIONS.contains(&name) {
                counts.known_sections += 1;
            }
            continue;
        }
        if let Some((key, _value)) = line.split_once(':') {
            let key = key.trim();
            if key.is_empty() || key.contains(' ') && !line.contains(": ") {
                continue;
            }
            counts.key_values += 1;
            saw_any = true;
            let klower = key.to_ascii_lowercase();
            if klower.starts_with("ring") && klower.ends_with("_addr")
                || klower.starts_with("knet_link")
                || klower == "ipaddr"
                || klower == "mcastaddr"
                || klower == "bindnetaddr"
            {
                counts.addresses += 1;
            }
            if klower == "name" || klower == "nodeid" {
                counts.names += 1;
            }
        }
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"totem {
    version: 2
    cluster_name: production
    transport: knet
    crypto_cipher: aes256
    crypto_hash: sha256
}
nodelist {
    node {
        ring0_addr: 192.168.1.11
        name: node1
        nodeid: 1
    }
    node {
        ring0_addr: 192.168.1.12
        name: node2
        nodeid: 2
    }
}
logging {
    to_logfile: yes
    logfile: /var/log/corosync/corosync.log
    timestamp: on
}
quorum {
    provider: corosync_votequorum
    expected_votes: 2
    two_node: 1
}
"#;

    #[test]
    fn detects_corosync() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.blocks, 6);
        assert_eq!(c.nodes, 2);
        assert_eq!(c.addresses, 2);
        assert_eq!(c.names, 4);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"section { key = value }"));
    }
}

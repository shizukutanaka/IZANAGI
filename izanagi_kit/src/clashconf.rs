//! Clash/Clash Meta `config.yaml` の検出と構造カウント。
//!
//! `proxies:`/`proxy-groups:`/`rules:`/`proxy-providers:`/`dns:`/`tun:` 等の
//! 既知トップレベルキー + `- name:`/`- `{name: ...}` プロキシ/ルールエントリ
//! + `type:` プロトコル値(`vmess`/`ss`/`trojan`/`hysteria*`/`tuic`/`wireguard` 等)を識別する。
//!
//! ```
//! let c = izanagi_kit::clashconf::parse(
//!     b"mixed-port: 7890\nproxies:\n  - name: \"a\"\n    type: vmess\nrules:\n  - MATCH,PROXY\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert!(izanagi_kit::clashconf::detect(b"proxies:\n  - name: x\nrules:\n  - MATCH,PROXY\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "allow-lan",
    "authentication",
    "bind-address",
    "clash-for-android",
    "dns",
    "ebpf",
    "experimental",
    "external-controller",
    "external-controller-cors",
    "external-ui",
    "geodata-mode",
    "geodata-url",
    "geo-update-interval",
    "global-client-fingerprint",
    "ipv6",
    "keep-alive-interval",
    "listeners",
    "log-level",
    "mixed-port",
    "mode",
    "ntp",
    "port",
    "profile",
    "proxies",
    "proxy-groups",
    "proxy-providers",
    "redir-port",
    "routing-mark",
    "rules",
    "secret",
    "socks-port",
    "sub-rules",
    "tproxy-port",
    "tun",
    "udp",
    "unified-delay",
];
/// プロキシプロトコル値(`type:` の既知値)。
const PROTOCOLS: &[&str] = &[
    "anytls",
    "direct",
    "dns",
    "fallback",
    "hysteria",
    "hysteria2",
    "load-balance",
    "mieru",
    "mtu",
    "relay",
    "reject",
    "selector",
    "shadowsocks",
    "shadowsocksr",
    "snell",
    "socks5",
    "ss",
    "ssh",
    "trojan",
    "tuic",
    "url-test",
    "vless",
    "vmess",
    "wireguard",
];
/// ルール先頭語(行先頭の `DOMAIN` 等)。
const RULE_HEADS: &[&str] = &[
    "AND",
    "DEST-PORT",
    "DOMAIN",
    "DOMAIN-KEYWORD",
    "DOMAIN-REGEX",
    "DOMAIN-SUFFIX",
    "DST-PORT",
    "GEOIP",
    "GEOSITE",
    "IN-PORT",
    "IN-TYPE",
    "IN-USER",
    "IP-ASN",
    "IP-CIDR",
    "IP-CIDR6",
    "IP-SUFFIX",
    "MATCH",
    "NETWORK",
    "NOT",
    "OR",
    "PROCESS-NAME",
    "RULE-SET",
    "SCRIPT",
    "SRC-IP-CIDR",
    "SRC-PORT",
    "SUB-RULES",
    "UID",
];

/// Clash 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルセクション(`proxies:`/`rules:` 等、非インデント)。
    pub sections: usize,
    /// `- name:`/`- `{...}`/`- RULE,…` エントリ。
    pub entries: usize,
    /// `type:`/`key:` 等ネスト代入。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// YAML の `key:` または `key: value` 行のキーを返す。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        None
    } else {
        Some(k)
    }
}

/// エントリ行 `- ...` のルール先頭語が既知か。
fn rule_head(t: &str) -> bool {
    let rest = t.trim_start_matches('-').trim();
    RULE_HEADS
        .iter()
        .any(|h| rest == *h || rest.starts_with(&format!("{h},")))
}

/// b が Clash config.yaml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        let indent = line.len() - line.trim_start().len();
        if (indent == 0 && yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)))
            || (t.starts_with('-') && rule_head(t))
            || (yaml_key(t) == Some("type")
                && t[t.find(':').map_or(0, |p| p + 1)..]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .split('#')
                    .next()
                    .is_some_and(|v| PROTOCOLS.contains(&v.trim())))
        {
            hits += 1;
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('-') {
            c.entries += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if yaml_key(t).is_some() {
            if indent == 0 {
                c.sections += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# clash\nmixed-port: 7890\nallow-lan: true\nproxies:\n  - name: \"edge\"\n    type: vmess\n    server: ex.com\nproxy-groups:\n  - name: PROXY\n    type: select\n    proxies: [edge]\nrules:\n  - DOMAIN-SUFFIX,google.com,PROXY\n  - MATCH,DIRECT\n";

    #[test]
    fn clashconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.entries, 4);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_clash() {
        assert!(!detect(b"foo:\n  bar: baz\n"));
        assert!(!detect(b"hello\n"));
    }
}

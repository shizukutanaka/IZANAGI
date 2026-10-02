//! sing-box `config.json` の検出と構造カウント。
//!
//! トップレベルの `"log"`/`"dns"`/`"ntp"`/`"inbounds"`/`"outbounds"`/`"route"`/
//! `"experimental"`/`"services"`/`"endpoints"`/`"certificate"` キー +
//! `"type": "<known>"` プロトコル値(`"vmess"`/`"vless"`/`"trojan"`/`"shadowsocks"`/
//! `"hysteria"`/`"hysteria2"`/`"tuic"`/`"wireguard"`/`"direct"`/`"block"`/`"dns"` 等)を識別する。
//!
//! ```
//! let c = izanagi_kit::singboxconf::parse(
//!     b"{\n  \"inbounds\": [{\"type\": \"tun\"}],\n  \"outbounds\": [{\"type\": \"direct\"}]\n}\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::singboxconf::detect(
//!     b"{\n  \"dns\": {},\n  \"outbounds\": [{\"type\": \"vless\"}]\n}\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "certificate",
    "dns",
    "endpoints",
    "experimental",
    "inbounds",
    "log",
    "ntp",
    "outbounds",
    "providers",
    "route",
    "services",
];
/// 既知 `"type"` 値(inbound/outbound/endpoint/service/dns/rule)。
const TYPES: &[&str] = &[
    "anytls",
    "block",
    "chained",
    "direct",
    "dns",
    "doc",
    "hysteria",
    "hysteria2",
    "http",
    "https",
    "juicity",
    "local",
    "loopback",
    "mixed",
    "naive",
    "quic",
    "redirect",
    "remote",
    "shadowtls",
    "shadowsocks",
    "shadowsocksr",
    "socks",
    "tailscale",
    "tor",
    "transport",
    "trojan",
    "tproxy",
    "tuic",
    "tun",
    "vless",
    "vmess",
    "wireguard",
];

/// sing-box 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"inbounds"`/`"outbounds"` 等トップレベルキー出現。
    pub sections: usize,
    /// `"type": "<known>"` 出現。
    pub types: usize,
    /// その他の `"key":` 出現。
    pub keys: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行内の全 `"key":` 出現を列挙。
fn keys_in_line<'a>(t: &'a str, out: &mut Vec<&'a str>) {
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j < bytes.len() && j > start {
                let mut k = j + 1;
                while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b':' {
                    out.push(&t[start..j]);
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
}

/// `"type": "..."` の値を抽出。
fn type_value(t: &str) -> Option<&str> {
    let p = t.find("\"type\"")?;
    let rest = &t[p + 6..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// b が sing-box config.json かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut types = 0;
    let mut keys = Vec::new();
    for line in text.lines() {
        keys.clear();
        keys_in_line(line, &mut keys);
        for k in &keys {
            if TOP_KEYS.contains(k) {
                secs += 1;
            }
            if *k == "type" && type_value(line).is_some_and(|v| TYPES.contains(&v)) {
                types += 1;
            }
        }
    }
    secs >= 1 && types >= 1 || secs >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        types: 0,
        keys: 0,
        misc: 0,
    };
    let mut keys = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        keys.clear();
        keys_in_line(line, &mut keys);
        if keys.is_empty() {
            if t.chars().any(|ch| ch.is_alphanumeric()) {
                c.misc += 1;
            }
            continue;
        }
        for k in &keys {
            if TOP_KEYS.contains(k) {
                c.sections += 1;
            } else if *k == "type" {
                if type_value(line).is_some_and(|v| TYPES.contains(&v)) {
                    c.types += 1;
                } else {
                    c.keys += 1;
                }
            } else {
                c.keys += 1;
            }
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"log\": {\"level\": \"info\"},\n  \"dns\": {\"servers\": []},\n  \"inbounds\": [{\"type\": \"tun\", \"tag\": \"tun-in\"}],\n  \"outbounds\": [{\"type\": \"vless\", \"tag\": \"proxy\"}, {\"type\": \"direct\"}],\n  \"route\": {\"rules\": []}\n}\n";

    #[test]
    fn singboxconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.types, 3);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_singbox() {
        assert!(!detect(b"{\"foo\": 1}\n"));
        assert!(!detect(b"hello\n"));
    }
}

//! Xray-core `config.json` の検出と構造カウント。
//!
//! トップレベルの `"inbounds"`/`"outbounds"`/`"routing"`/`"dns"`/`"policy"`/
//! `"log"`/`"stats"`/`"api"`/`"transport"`/`"reverse"`/`"fakedns"`/`"observatory"`/
//! `"burstObservatory"` + プロトコル値(`"vmess"`/`"vless"`/`"trojan"`/`"shadowsocks"`/
//! `"wireguard"`/`"freedom"`/`"blackhole"`/`"dokodemo-door"`/`"socks"`/`"http"`)を識別する。
//!
//! ```
//! let c = izanagi_kit::xrayconf::parse(
//!     b"{\n  \"inbounds\": [{\"protocol\": \"vless\"}],\n  \"outbounds\": [{\"protocol\": \"freedom\"}]\n}\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::xrayconf::detect(
//!     b"{\n  \"inbounds\": [],\n  \"outbounds\": []\n}\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "api",
    "burstObservatory",
    "dns",
    "fakedns",
    "inbounds",
    "log",
    "metrics",
    "observatory",
    "outbounds",
    "policy",
    "reverse",
    "routing",
    "stats",
    "transport",
];
/// 既知プロトコル/設定値。
const PROTOCOLS: &[&str] = &[
    "blackhole",
    "dns",
    "dokodemo-door",
    "ds",
    "freedom",
    "grpc",
    "http",
    "httpupgrade",
    "hy2",
    "juicity",
    "loopback",
    "quic",
    "reality",
    "shadowsocks",
    "socks",
    "splithttp",
    "trojan",
    "tuic",
    "vless",
    "vmess",
    "websocket",
    "wireguard",
    "ws",
];

/// Xray 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"inbounds"`/`"outbounds"` 等トップレベルキー出現。
    pub sections: usize,
    /// `"protocol": "<known>"` 既知プロトコル値。
    pub protocols: usize,
    /// その他の `"key":` 出現。
    pub keys: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行内の全 `"key":` 出現を列挙(1行複数キーの JSON に対応)。
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

/// `"protocol": "..."` や `"network": "..."` の値を抽出。
fn value_of<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    let pat = key;
    let p = t.find(pat)?;
    let rest = &t[p + pat.len()..];
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// b が Xray config.json かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut secs = 0;
    let mut protos = 0;
    let mut keys = Vec::new();
    for line in text.lines() {
        keys.clear();
        keys_in_line(line, &mut keys);
        for k in &keys {
            if TOP_KEYS.contains(k) {
                secs += 1;
            }
            if *k == "protocol"
                && value_of(line, "\"protocol\"").is_some_and(|v| PROTOCOLS.contains(&v))
            {
                protos += 1;
            }
        }
    }
    secs >= 1 && protos >= 1 || secs >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        protocols: 0,
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
            if t.chars()
                .any(|ch| ch.is_alphanumeric() || ch == '"' || ch == '/')
            {
                // JSON 構造行(`{}`, `]`)は misc 対象外とみなす。
                if t.chars().any(|ch| ch.is_alphanumeric()) {
                    c.misc += 1;
                }
            }
            continue;
        }
        for k in &keys {
            if TOP_KEYS.contains(k) {
                c.sections += 1;
            } else if *k == "protocol" || *k == "network" || *k == "security" {
                if value_of(line, &format!("\"{k}\"")).is_some_and(|v| PROTOCOLS.contains(&v)) {
                    c.protocols += 1;
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

    const SAMPLE: &[u8] = b"{\n  \"log\": {\"loglevel\": \"warning\"},\n  \"inbounds\": [{\"port\": 1080, \"protocol\": \"vless\", \"settings\": {}}],\n  \"outbounds\": [{\"protocol\": \"freedom\"}],\n  \"routing\": {\"domainStrategy\": \"IPIfNonMatch\"}\n}\n";

    #[test]
    fn xrayconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.protocols, 2);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_xray() {
        assert!(!detect(b"{\"foo\": 1}\n"));
        assert!(!detect(b"hello\n"));
    }
}

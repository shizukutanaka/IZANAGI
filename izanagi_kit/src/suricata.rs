//! Suricata IDS/IPS `suricata.yaml` パーサ。
//!
//! `%YAML 1.1` + `---` ドキュメントマーカと `vars:`/`af-packet:`/`outputs:`/
//! `app-layer:`/`detect-engine:` 等既知トップキーを計数する。
//!
//! ```
//! use izanagi_kit::suricata;
//! let conf = b"%YAML 1.1\n---\nvars:\n    HOME_NET: \"[192.168.0.0/16]\"\naf-packet:\n  - interface: eth0\noutputs:\n  - eve-log:\n        enabled: yes\n";
//! assert!(suricata::detect(conf));
//! let c = suricata::parse(conf).unwrap();
//! assert_eq!(c.known_tops, 3);
//! assert_eq!(c.top_keys, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `%`/`---`/`...` マーカ行数。
    pub markers: usize,
    /// 列0 `key:` トップキー数。
    pub top_keys: usize,
    /// 既知トップキー数。
    pub known_tops: usize,
    /// インデント `key:`/`key: v` 入れ子エントリ数。
    pub entries: usize,
    /// `-` リスト項目数。
    pub list_items: usize,
}

const KNOWN_TOPS: &[&str] = &[
    "vars",
    "address-groups",
    "port-groups",
    "af-packet",
    "outputs",
    "default-rule-path",
    "rule-files",
    "classification-file",
    "reference-config-file",
    "threshold-file",
    "app-layer",
    "coredump",
    "host-mode",
    "unix-command",
    "stream",
    "defrag",
    "flow",
    "flow-timeouts",
    "detect",
    "detect-engine",
    "pfring",
    "napatech",
    "erf",
    "netmap",
    "ipfw",
    "pcap",
    "pcap-file",
    "eve-log",
    "logging",
    "plugin",
    "vlan",
    "asn1-max-frames",
    "stats",
    "decoder",
    "threading",
    "cpu-affinity",
    "resolution-mode",
    "datasets",
    "security",
    "hostbits",
    "host-os-policy",
    "mpm-algo",
    "spm-algo",
    "action-order",
    "prelude",
    "cuda",
    "commondir",
];

/// 簡易判定 (YAML マーカ + 既知トップキー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.markers >= 1 && c.known_tops >= 2) || (c.known_tops >= 3 && c.entries >= 3)
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        markers: 0,
        top_keys: 0,
        known_tops: 0,
        entries: 0,
        list_items: 0,
    };
    let mut found = false;
    for line in s.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let t = line.trim_start();
        if t.starts_with('%') || t == "---" || t == "..." {
            c.markers += 1;
            found = true;
            continue;
        }
        if t.starts_with('-') {
            c.list_items += 1;
            found = true;
            continue;
        }
        let Some(colon) = t.find(':') else {
            continue;
        };
        let key = &t[..colon];
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            continue;
        }
        if line.starts_with(|b: char| b.is_ascii_alphabetic() || b == '_') {
            c.top_keys += 1;
            if KNOWN_TOPS.contains(&key) {
                c.known_tops += 1;
            }
        } else {
            c.entries += 1;
        }
        found = true;
    }
    found.then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"%YAML 1.1\n---\nvars:\n  address-groups:\n    HOME_NET: \"[192.168.0.0/16]\"\n  port-groups:\n    HTTP_PORTS: \"80\"\naf-packet:\n  - interface: eth0\noutputs:\n  - eve-log:\n      enabled: yes\n      filetype: regular\nlogging:\n  default-log-level: notice\ndetect-engine:\n  - profile: medium\n";

    #[test]
    fn detects_suricata() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.markers, 2);
        assert_eq!(c.top_keys, 5);
        assert_eq!(c.known_tops, 5);
        assert_eq!(c.entries, 7);
        assert_eq!(c.list_items, 3);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(!detect(b"name: app\nversion: 1\nfeatures:\n  - a\n"));
        assert!(!detect(b"server:\n  host: x\n  port: 1\n"));
    }
}

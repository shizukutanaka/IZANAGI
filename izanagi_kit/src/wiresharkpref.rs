//! Wireshark プリファレンスファイル (`preferences`, `enabled_protos` 等) パーサ。
//!
//! `pref.name: value` コロン区切り行と `#` コメント。
//! `gui.`/`nameres.`/`tcp.`/`wlan.`/`uat.`/`extcap.` 等プロトコル接頭辞で識別する。
//!
//! ```
//! use izanagi_kit::wiresharkpref;
//! let p = b"# comment\ngui.qt.enabled: TRUE\nnameres.network_name: TRUE\nwlan.enable_decryption: FALSE\ntcp.desegment_tcp_streams: TRUE\nip.check_checksum: FALSE\n";
//! assert!(wiresharkpref::detect(p));
//! let c = wiresharkpref::parse(p).unwrap();
//! assert_eq!(c.entries, 5);
//! assert_eq!(c.known_prefixes, 5);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `pref.name: value` 行数。
    pub entries: usize,
    /// 既知プロトコル/機能接頭辞に一致する行数。
    pub known_prefixes: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

const KNOWN_PREFIXES: &[&str] = &[
    "gui.",
    "qt.",
    "console.",
    "nameres.",
    "dns.",
    "print.",
    "stream.",
    "wlan.",
    "wlan_extended.",
    "ip.",
    "ipv6.",
    "tcp.",
    "udp.",
    "eth.",
    "epl.",
    "http.",
    "http2.",
    "sip.",
    "rtp.",
    "rtsp.",
    "t38.",
    "h225.",
    "h245.",
    "h248.",
    "h323.",
    "sccp.",
    "sctp.",
    "diameter.",
    "radius.",
    "nas-eps.",
    "nas5gs.",
    "gsm_a.",
    "gtp.",
    "gtpv2.",
    "snort.",
    "extcap.",
    "uat.",
    "stats.",
    "tap.",
    "transum.",
    "tls.",
    "ssl.",
    "dmp.",
    "isup.",
    "m3ua.",
    "m2pa.",
    "m2ua.",
    "snmp.",
    "smtp.",
    "ssh.",
    "tshark.",
    "x11.",
    "ansi_map.",
    "ansi_tcap.",
    "ansi_is637.",
    "ansi_a.",
    "ansi_itss.",
    "ansi_i501.",
    "ansi_i503.",
    "ansi_i602.",
    "ansi_i603.",
    "mac-lte.",
    "mac-nr.",
    "pdml.",
    "lapd.",
    "mgcp.",
    "nbap.",
    "nbss.",
    "ospf.",
    "pdu.",
    "pdu-log.",
    "pkcs12.",
    "profinet.",
    "ptp.",
    "rpl.",
    "s7comm.",
    "sscop.",
    "smb.",
    "smb2.",
    "usb.",
    "wireshark.",
    "geoip.",
    "ipfix.",
    "kafka.",
    "krb5.",
    "ldap.",
    "llc.",
    "loquacious.",
    "manolito.",
    "mars.",
    "mate.",
    "mms.",
    "mndp.",
    "mpls.",
    "mtp2.",
    "mtp3.",
    "mqtt.",
    "multicast.",
    "netflow.",
    "ntp.",
    "openflow.",
    "pgm.",
    "proxy.",
    "sametime.",
    "sc6.",
    "sigcomp.",
    "sofastats.",
];

/// 簡易判定 (`pref: value` 行 + 既知接頭辞)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.entries >= 5 && c.known_prefixes >= 3
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        known_prefixes: 0,
        comments: 0,
    };
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let Some(colon) = t.find(':') else {
            continue;
        };
        let key = t[..colon].trim();
        // `uat:file:section` 形式は最初のコロンが接頭辞区切り
        let key = if key == "uat" && t[colon + 1..].contains(':') {
            "uat."
        } else {
            key
        };
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        {
            continue;
        }
        c.entries += 1;
        if KNOWN_PREFIXES.iter().any(|p| key.starts_with(p)) {
            c.known_prefixes += 1;
        }
    }
    (c.entries > 0 || c.comments > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"# Wireshark Preferences\ngui.qt.toolbar.enabled: TRUE\ngui.qt.font_name: Mono,10,-1,5,50,0,0,0,0,0\ngui.recent_files: /tmp/a.pcap\nnameres.network_name: TRUE\nnameres.use_external_name_resolver: TRUE\nwlan.enable_decryption: FALSE\ntcp.desegment_tcp_streams: TRUE\ntcp.check_checksum: FALSE\nip.check_checksum: FALSE\nuat:filter.buttons: \"New\"\nextcap.user_log.snort: false\nstats.colorfilter_buttons: \"On\"\ntransum.packets_considered: 1000\n";

    #[test]
    fn detects_wiresharkpref() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 13);
        assert_eq!(c.known_prefixes, 13);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"key: value\nfoo: bar\n"));
        assert!(!detect(b"# only comments\n# nothing else\n"));
    }
}

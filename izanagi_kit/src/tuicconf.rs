//! `config.json` (TUIC) 検出モジュール。
//!
//! TUIC プロキシの設定は JSON で、`"server"`/`"tag"`/`"uuid"`/
//! `"password"`/`"congestion_control"`/`"alpn"`/`"dual_stack"`/
//! `"udp_relay_mode"`/`"reduce_rtt"`/`"heartbeat"`/`"timeout"`/
//! `"gc_lifetime"`/`"send_window"`/`"receive_window"`/
//! `"max_external_packet_size"`/`"disable_sni"`/`"zero_rtt_handshake"`
//! 等のキーを持つ。
//!
//! ```
//! let b = br#"{
//!     "server": "example.com:443",
//!     "uuid": "00000000-0000-0000-0000-000000000001",
//!     "password": "passwd",
//!     "congestion_control": "bbr",
//!     "alpn": ["h3"]
//! }
//! "#;
//! let c = izanagi_kit::tuicconf::parse(b);
//! assert!(izanagi_kit::tuicconf::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "alpn",
    "certificates",
    "congestion_control",
    "disable_sni",
    "dual_stack",
    "gc_lifetime",
    "heartbeat",
    "ip",
    "local_addr",
    "local_port",
    "log_level",
    "max_datagram_frame_size",
    "max_external_packet_size",
    "network",
    "password",
    "private_key",
    "receive_window",
    "reduce_rtt",
    "rest_natives",
    "send_window",
    "server",
    "server_module_path",
    "tag",
    "timeout",
    "tls_fragment",
    "tls_fragment_size",
    "udp_relay_mode",
    "users",
    "uuid",
    "zero_rtt_handshake",
];

const HINTS: &[&str] = &[
    "congestion_control",
    "udp_relay_mode",
    "reduce_rtt",
    "uuid",
    "zero_rtt_handshake",
    "send_window",
    "receive_window",
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

/// `b` が TUIC config.json に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut hints = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                keys += 1;
            }
            if HINTS.contains(&k) {
                hints += 1;
            }
        }
    }
    (hints >= 1 && keys >= 2) || keys >= 5
}

/// TUIC config.json の統計。
#[derive(Debug, Default, Clone)]
pub struct TuicConf {
    /// 既知キー行数。
    pub keys: usize,
    /// TUIC 特有キー行数。
    pub hint_keys: usize,
}

/// `b` を TUIC config.json として統計する。
pub fn parse(b: &[u8]) -> TuicConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = TuicConf::default();
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                c.keys += 1;
            }
            if HINTS.contains(&k) {
                c.hint_keys += 1;
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
    "server": "x.com:443",
    "uuid": "00000000-0000-0000-0000-000000000001",
    "password": "p"
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
        assert_eq!(c.hint_keys, 1);
    }

    #[test]
    fn detects_many() {
        let b = br#"{
    "server": "a",
    "tag": "t",
    "timeout": "3s",
    "alpn": [],
    "dual_stack": true
}
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\"server\": \"x\", \"uuid\": \"u\"}"));
        assert!(!detect(
            b"{\"name\": \"x\", \"version\": 1, \"config\": {}}"
        ));
        assert!(!detect(b"server=x\nuuid=u\npassword=p\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

//! `aprx.conf` 検出モジュール。
//!
//! aprx (APRS ゲートウェイ/デジピータ) の設定は `mycall N0CALL`/
//! `myloc lat lon`/`login`/`server`/`tx-ok`/`beacon`/`telemetry`
//! 等のディレクティブと `<aprsis>`/`<interface>`/`<beacon>`/
//! `<logging>`/`<digipeater>`/`<telemetry>` のブロックで構成される。
//!
//! ```
//! let b = br#"mycall N0CALL-2
//! myloc lat 35.68 lon 139.69
//! <aprsis>
//!    login N0CALL-2
//!    server t2.aprs.net
//! </aprsis>
//! <interface>
//!    tx-ok true
//! </interface>
//! "#;
//! let c = izanagi_kit::aprxconf::parse(b);
//! assert!(izanagi_kit::aprxconf::detect(b));
//! assert!(c.blocks >= 2);
//! ```

const BLOCKS: &[&str] = &[
    "<aprsis>",
    "<beacon>",
    "<digipeater>",
    "<dprs>",
    "<interface>",
    "<logging>",
    "<telemetry>",
];

const DIRECTIVES: &[&str] = &[
    "beacon",
    "beaconmode",
    "beacon-symbol",
    "beacon-via",
    "filter",
    "login",
    "mycall",
    "myloc",
    "passcode",
    "relay-type",
    "serial-device",
    "server",
    "telemetry",
    "timeout",
    "tx-ok",
    "viscus-delay",
];

fn block_line(t: &str) -> bool {
    BLOCKS.contains(&t) || (t.starts_with("</") && BLOCKS.iter().any(|b| b[1..] == t[2..]))
}

fn directive(t: &str) -> bool {
    let head = t.split_whitespace().next().unwrap_or("");
    DIRECTIVES.contains(&head)
}

/// `b` が aprx.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut blocks = 0usize;
    let mut dirs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if block_line(tr) {
            blocks += 1;
        } else if directive(tr) {
            dirs += 1;
        }
    }
    (blocks >= 2 && dirs >= 1) || dirs >= 3 || blocks >= 3
}

/// aprx.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct AprxConf {
    /// `<block>`/`</block>` 行数。
    pub blocks: usize,
    /// 既知ディレクティブ行数。
    pub directives: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を aprx.conf として統計する。
pub fn parse(b: &[u8]) -> AprxConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = AprxConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if block_line(tr) {
            c.blocks += 1;
        } else if directive(tr) {
            c.directives += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"mycall N0CALL-2
<aprsis>
login N0CALL-2
</aprsis>
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.blocks, 2);
        assert_eq!(c.directives, 2);
    }

    #[test]
    fn detects_directives() {
        let b = br#"mycall N0CALL
myloc lat 35.68 lon 139.69
passcode 12345
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"mycall N0CALL\n"));
        assert!(!detect(b"<aprsis>\n</aprsis>\n"));
        assert!(!detect(b"<html>\n<body>\ntext\n</body>\n</html>\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.blocks, 0);
    }
}

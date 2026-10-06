//! `avrdude.conf` 検出モジュール。
//!
//! AVRDUDE の設定は `part`/`programmer`/`memory` ブロックと
//! `id`/`desc`/`signature`/`baudrate` 等の `key = value;` 代入で
//! 構成される独自形式。
//!
//! ```
//! let b = br#"programmer
//!     id = "usbasp";
//!     desc = "USBasp";
//!     type = "usbasp";
//!     connection_type = usb;
//!     baudrate = 19200;
//! ;
//! part
//!     id = "m328p";
//!     desc = "ATmega328P";
//! ;
//! "#;
//! let c = izanagi_kit::avrdude::parse(b);
//! assert!(izanagi_kit::avrdude::detect(b));
//! assert_eq!(c.blocks, 2);
//! ```

const BLOCKS: &[&str] = &["part", "programmer", "memory", "pin"];

const KEYS: &[&str] = &[
    "baudrate",
    "blocksize",
    "bufsize",
    "chip_erase_delay",
    "connection_type",
    "default_serial",
    "desc",
    "descoff",
    "flash",
    "eeprom",
    "hvupdi_support",
    "id",
    "is_at90s1200",
    "memory",
    "min_write_delay",
    "num_pages",
    "offset",
    "page_size",
    "parent_id",
    "password",
    "pgm_enable",
    "readsize",
    "signature",
    "size",
    "type",
    "usbdev",
    "usbpid",
    "usbproduct",
    "usbserial",
    "usbvendor",
    "usbsn",
    "usbvid",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn key_present(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が avrdude.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut blocks = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if BLOCKS.contains(&tr) {
            blocks += 1;
        } else if KEYS.iter().any(|k| key_present(tr, k)) {
            keys += 1;
        }
    }
    (blocks >= 1 && keys >= 3) || keys >= 5
}

/// avrdude.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct AvrdudeConf {
    /// `part`/`programmer`/`memory`/`pin` ブロック開始行数。
    pub blocks: usize,
    /// 既知 `key = value;` 行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を avrdude.conf として統計する。
pub fn parse(b: &[u8]) -> AvrdudeConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = AvrdudeConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if BLOCKS.contains(&tr) {
            c.blocks += 1;
        } else if KEYS.iter().any(|k| key_present(tr, k)) {
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
        let b = br#"part
    id = "m328p";
    desc = "ATmega328P";
    signature = 0x1e 0x95 0x0f;
;
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.blocks, 1);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_keys_only() {
        let b = br#"id = "usbasp";
desc = "USBasp";
type = "usbasp";
connection_type = usb;
baudrate = 19200;
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 5);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"id = 1\ndesc = x\n"));
        assert!(!detect(b"foo = 1\nbar = 2\nbaz = 3\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.blocks, 0);
    }
}

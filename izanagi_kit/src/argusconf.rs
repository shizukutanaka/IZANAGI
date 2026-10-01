//! Argus フロー監視設定ファイル (`argus.conf`, `ra.conf`, `rarc`) パーサ。
//!
//! `ARGUS_*` / `RA_*` 接頭辞の `KEY=value` 環境変数風代入を計数する。
//!
//! ```
//! use izanagi_kit::argusconf;
//! let conf = b"ARGUS_DAEMON=yes\nARGUS_INTERFACE=eth0\nARGUS_GO_PROMISCUOUS=yes\n";
//! assert!(argusconf::detect(conf));
//! let c = argusconf::parse(conf).unwrap();
//! assert_eq!(c.entries, 3);
//! assert_eq!(c.argus_keys, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `KEY=value` 行数。
    pub entries: usize,
    /// `ARGUS_*` キー行数。
    pub argus_keys: usize,
    /// `RA_*` キー行数 (ra.conf/rarc)。
    pub ra_keys: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// 簡易判定 (`ARGUS_*`/`RA_*` 接頭辞キー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.entries >= 3 && (c.argus_keys + c.ra_keys) >= 3
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        argus_keys: 0,
        ra_keys: 0,
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
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            continue;
        }
        c.entries += 1;
        if key.starts_with("ARGUS_") {
            c.argus_keys += 1;
        }
        if key.starts_with("RA_") {
            c.ra_keys += 1;
        }
    }
    (c.entries > 0 || c.comments > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"ARGUS_DAEMON=yes\nARGUS_INTERFACE=any\nARGUS_GO_PROMISCUOUS=yes\nARGUS_FILTER=\"ip\"\nARGUS_ACCESS_PORT=561\nARGUS_OUTPUT_FILE=/var/log/argus/argus.ra\nARGUS_MAR_STATUS_INTERVAL=60\nARGUS_DEBUG_LEVEL=0\nARGUS_SET_PID=yes\nARGUS_PID_PATH=/var/run\nRA_INPUT=argus.ra\nRA_TIME_FORMAT=\"%M/%d/%y.%T\"\nRA_SORT=-M time\n";

    #[test]
    fn detects_argusconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 13);
        assert_eq!(c.argus_keys, 10);
        assert_eq!(c.ra_keys, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"HOME=/root\nPATH=/bin\nEDITOR=vim\n"));
        assert!(!detect(b"#!/bin/sh\nexec foo\n"));
    }
}

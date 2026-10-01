//! DRBD `drbd.conf`/`*.res` パーサ。
//!
//! `global`/`common`/`resource` ブロックと `on <host> { ... }` 内の
//! `device`/`disk`/`address`/`meta-disk`/`protocol`/`net`/`disk` オプションを計数する。
//!
//! ```
//! use izanagi_kit::drbdconf;
//! let conf = b"resource r0 {\n  protocol C;\n  on alice {\n    device /dev/drbd0;\n    disk /dev/sda3;\n    address 10.0.0.1:7788;\n    meta-disk internal;\n  }\n}\n";
//! assert!(drbdconf::detect(conf));
//! let c = drbdconf::parse(conf).unwrap();
//! assert_eq!(c.known_keywords, 7);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セミコロン終端ステートメント数。
    pub statements: usize,
    /// `{` ブロック開始数。
    pub blocks: usize,
    /// 既知キーワード先頭ステートメント数。
    pub known_keywords: usize,
    /// `on <host>` ブロック数。
    pub on_blocks: usize,
}

const KNOWN_KEYWORDS: &[&str] = &[
    "resource",
    "global",
    "common",
    "on",
    "device",
    "disk",
    "address",
    "meta-disk",
    "meta-disk-index",
    "node-id",
    "protocol",
    "net",
    "disk",
    "startup",
    "handlers",
    "syncer",
    "options",
    "connection",
    "peer-device",
    "volume",
    "shared-secret",
    "cram-hmac-alg",
    "c-plan-ahead",
    "c-max-rate",
    "sndbuf-size",
    "rcvbuf-size",
    "rate",
    "verify-alg",
    "csums-alg",
    "use-rle",
    "ko-count",
    "max-buffers",
    "max-epoch-size",
    "unplug-watermark",
    "al-extents",
    "allow-two-primaries",
    "disable-ip-verification",
    "fencing",
    "always-asbp",
    "rr-conflict",
    "ping-timeout",
    "ping-int",
    "timeout",
    "connect-int",
    "wfc-timeout",
    "degr-wfc-timeout",
    "become-primary-on",
    "usage-count",
    "skip-disk",
    "export",
    "connection-mesh",
];

/// `drbd.conf` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => (c.known_keywords >= 4 && c.statements >= 5) || c.on_blocks >= 2,
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        statements: 0,
        blocks: 0,
        known_keywords: 0,
        on_blocks: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('{') || t.ends_with('{') {
            c.blocks += 1;
        }
        if t.starts_with('}') || t.ends_with("};") {
            continue;
        }
        let word = t
            .split([' ', '\t', ';', '{'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if word.is_empty() || !word.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            continue;
        }
        if t.ends_with(';') || t.contains(';') {
            c.statements += 1;
        }
        if KNOWN_KEYWORDS.contains(&word.as_str()) {
            c.known_keywords += 1;
        }
        if word == "on" {
            c.on_blocks += 1;
        }
    }
    (c.known_keywords > 0 || c.statements > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"global {\n  usage-count yes;\n}\nresource r0 {\n  protocol C;\n  net {\n    cram-hmac-alg sha1;\n    shared-secret secret;\n  }\n  on alice {\n    device /dev/drbd0;\n    disk /dev/sda3;\n    address 10.0.0.1:7788;\n    meta-disk internal;\n  }\n  on bob {\n    device /dev/drbd0;\n    disk /dev/sdb3;\n    address 10.0.0.2:7788;\n    meta-disk internal;\n  }\n}\n";

    #[test]
    fn detects_drbdconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.on_blocks, 2);
        assert_eq!(c.known_keywords, 17);
    }

    #[test]
    fn rejects_nginx() {
        let ngx = b"http {\n  server {\n    listen 80;\n    location / {\n      proxy_pass http://x;\n    }\n  }\n}\n";
        assert!(!detect(ngx));
    }
}

//! `ssmtp.conf` / `revaliases` (sSMTP) 検出モジュール。
//!
//! sSMTP の設定は平坦な `key=value` 形式で、`root`/`mailhub`/
//! `rewriteDomain`/`hostname`/`FromLineOverride`/`UseTLS`/
//! `UseSTARTTLS`/`AuthUser`/`AuthPass`/`AuthMethod`/`TLSCert`/
//! `TLS_CA_File`/`TLS_CA_Dir`/`Debug`/`maildomain`/`MinUserId` 等の
//! キーで構成される。
//!
//! ```
//! let b = b"root=postmaster\n\
//!           mailhub=smtp.example.com:587\n\
//!           rewriteDomain=example.com\n\
//!           hostname=host.example.com\n\
//!           UseSTARTTLS=YES\n";
//! let c = izanagi_kit::ssmtpconf::parse(b);
//! assert!(izanagi_kit::ssmtpconf::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "AuthMethod",
    "AuthPass",
    "AuthToken",
    "AuthUser",
    "Debug",
    "FromLineOverride",
    "hostname",
    "maildomain",
    "mailhub",
    "MinUserId",
    "revaliases",
    "rewriteDomain",
    "root",
    "TLS_CA_Dir",
    "TLS_CA_File",
    "TLSCert",
    "TLSKey",
    "UseSTARTTLS",
    "UseTLS",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が ssmtp.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.contains('=') && key_present(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// ssmtp.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct SsmtpConf {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
    /// 総行数。
    pub lines: usize,
}

/// `b` を ssmtp.conf として統計する。
pub fn parse(b: &[u8]) -> SsmtpConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SsmtpConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        c.lines += 1;
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if tr.contains('=') && key_present(tr) {
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
        let b = b"root=postmaster\nmailhub=smtp.x:25\nrewriteDomain=x.com\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"root=x\nmailhub=y\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[misc]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# root=x\n# mailhub=y\n# hostname=z\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 3);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

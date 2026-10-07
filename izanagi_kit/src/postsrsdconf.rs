//! postsrsd 設定 (`/etc/default/postsrsd`) 検出モジュール。
//!
//! postsrsd(Postfix SRS デーモン)の設定は systemd env 風の
//! `KEY=value` 形式で、`SRS_DOMAIN`/`SRS_EXCLUDE_DOMAINS`/
//! `SRS_FORWARD_PORT`/`SRS_REVERSE_PORT`/`SRS_SECRET`/
//! `SRS_SECRETS`/`SRS_TIMEOUT`/`SRS_FORWARD_ENVELOPE`/
//! `SRS_ORIGIN_ENVELOPE`/`SRS_LIST`/`RUN_AS`/`CHROOT_DIR`/
//! `SEPARATOR`/`SRS_NO_FORWARD`/`SRS_NO_REVERSE`/`SRS_HASHLENGTH`/
//! `SRS_SEPARATOR`/`SRS_EXTRA_PARAMETERS`/`SRS_ENGAGE_AFTER`
//! 等のキーで構成される。
//!
//! ```
//! let b = b"SRS_DOMAIN=example.com\n\
//!           SRS_EXCLUDE_DOMAINS=.example.com,example.org\n\
//!           SRS_FORWARD_PORT=10001\n\
//!           SRS_REVERSE_PORT=10002\n\
//!           SRS_SECRET=/etc/postsrsd.secret\n";
//! let c = izanagi_kit::postsrsdconf::parse(b);
//! assert!(izanagi_kit::postsrsdconf::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    k.starts_with("SRS_")
        || matches!(
            k,
            "RUN_AS" | "CHROOT_DIR" | "SEPARATOR" | "SRS_ENGAGE_AFTER"
        )
}

/// `b` が postsrsd 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.contains('=') && is_key(tr) {
            keys += 1;
        }
    }
    keys >= 2
}

/// postsrsd 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct PostsrsdConf {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を postsrsd 設定として統計する。
pub fn parse(b: &[u8]) -> PostsrsdConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = PostsrsdConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if tr.contains('=') && is_key(tr) {
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
        let b = b"SRS_DOMAIN=x.com\nSRS_FORWARD_PORT=10001\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"SRS_DOMAIN=x.com\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
        assert!(!detect(b"RUN_AS=nobody\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# SRS_DOMAIN=x\n# SRS_FORWARD_PORT=1\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 2);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

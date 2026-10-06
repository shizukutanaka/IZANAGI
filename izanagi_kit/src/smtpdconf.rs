//! `smtpd.conf` (OpenSMTPD) 検出モジュール。
//!
//! OpenSMTPD の設定はフリーフォームのコマンド行で、`listen on`/
//! `pki`/`table`/`action`/`match`/`queue`/`filter`/`mta`/`smtp`/
//! `bounce`/`masquerade`/`mda`/`mmda`/`include`/`ca`/`tag`/`limit`
//! 等のディレクティブと、`maildir`/`mbox`/`maildir`/`relay`/
//! `forward-only`/`lmtp`/`expand-only`/`for local`/`for any`/
//! `from any`/`auth`/`tls`/`smtps`/`verify`/`mask-src`/`hostname`/
//! `cert`/`key`/`ca`/`chain`/`dhparams`/`alias`/`virtual`/`domain`/
//! `userbase`/`credentials`/`senders`/`recipient`/`src`/`helo` 等の
//! 引数句で構成される。
//!
//! ```
//! let b = b"pki mail.example.com cert \"/etc/ssl/cert.pem\"\n\
//!           pki mail.example.com key \"/etc/ssl/key.pem\"\n\
//!           table aliases db:/etc/mail/aliases.db\n\
//!           listen on lo0\n\
//!           listen on egress tls pki mail.example.com\n\
//!           action \"mbox\" maildir \"~/mail\"\n\
//!           match for local action \"mbox\"\n\
//!           match from any for domain example.com action \"mbox\"\n";
//! let c = izanagi_kit::smtpdconf::parse(b);
//! assert!(izanagi_kit::smtpdconf::detect(b));
//! assert_eq!(c.directives, 8);
//! ```

const VERBS: &[&str] = &[
    "action",
    "bounce",
    "ca",
    "filter",
    "include",
    "limit",
    "listen",
    "masquerade",
    "match",
    "mda",
    "mmda",
    "mta",
    "pki",
    "queue",
    "smtp",
    "table",
    "tag",
    "ttl",
];

fn is_directive(t: &str) -> bool {
    let w = t.split_whitespace().next().unwrap_or("");
    VERBS.contains(&w)
}

/// `b` が smtpd.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut dirs = 0usize;
    let mut anchors = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_directive(tr) {
            dirs += 1;
            if tr.starts_with("listen on")
                || tr.starts_with("match ")
                || tr.starts_with("table ")
                || tr.starts_with("action ")
                || tr.starts_with("pki ")
            {
                anchors += 1;
            }
        }
    }
    anchors >= 2 || dirs >= 5
}

/// smtpd.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct SmtpdConf {
    /// ディレクティブ行数。
    pub directives: usize,
    /// listen/match/table/action/pki 行数。
    pub anchors: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を smtpd.conf として統計する。
pub fn parse(b: &[u8]) -> SmtpdConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SmtpdConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_directive(tr) {
            c.directives += 1;
            if tr.starts_with("listen on")
                || tr.starts_with("match ")
                || tr.starts_with("table ")
                || tr.starts_with("action ")
                || tr.starts_with("pki ")
            {
                c.anchors += 1;
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
        let b = b"listen on lo0\nmatch for local action \"m\"\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.anchors, 2);
    }

    #[test]
    fn detects_verbs() {
        let b = b"queue\nmta\nsmtp\nbounce warn\nmda x\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"listen on lo0\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}

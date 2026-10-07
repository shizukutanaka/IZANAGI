//! `.pinerc` (Alpine/Pine) 検出モジュール。
//!
//! Alpine の設定は平坦な `key=value` 形式で、`personal-name=`/
//! `user-domain=`/`smtp-server=`/`nntp-server=`/`inbox-path=`/
//! `folder-collections=`/`news-collections=`/`incoming-folders=`/
//! `default-fcc=`/`default-composer-hdrs=`/`customized-hdrs=`/
//! `signature-file=`/`signature-at-bottom`/`feature-list=`/
//! `initial-keystroke-list=`/`display-filters=`/`sending-filters=`/
//! `alt-addresses=`/`addressbook=`/`global-address-book=`/
//! `viewer-hdrs=`/`speller=`/`postponed-folder=`/`read-message-folder=`/
//! `sent-mail=`/`printer=`/`bugs-`/`rsh-`/`ssh-`/`ldap-servers=`/
//! `literal-signature=`/`quell-`/`empty-header-message=`/
//! `url-viewers=`/`download-interceptors=`/`last-time-prune-asked=`/
//! `last-version-used=`/`charset=`/`editor=`/`image-viewer=`/
//! `use-only-domain-name` 等のキーで構成される。
//!
//! ```
//! let b = b"personal-name=Taro Yamada\n\
//!           user-domain=example.com\n\
//!           smtp-server=smtp.example.com\n\
//!           inbox-path=INBOX\n\
//!           feature-list=enable-flag-cmd,enable-dot-files\n";
//! let c = izanagi_kit::pinerc::parse(b);
//! assert!(izanagi_kit::pinerc::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "addressbook",
    "addressbook-formats",
    "alt-addresses",
    "bugs-additional-data",
    "bugs-address",
    "bugs-name",
    "check-mail",
    "composer-wrap-column",
    "customized-hdrs",
    "default-composer-hdrs",
    "default-fcc",
    "display-filters",
    "download-interceptors",
    "editor",
    "empty-header-message",
    "fcc-on-bounce",
    "fcc-rule",
    "feature-list",
    "folder-collections",
    "form-letter-folder",
    "global-address-book",
    "image-viewer",
    "incoming-folders",
    "incoming-startup-rule",
    "inbox-path",
    "inbox-rule",
    "initial-keystroke-list",
    "ldap-servers",
    "last-time-prune-asked",
    "last-version-used",
    "literal-signature",
    "mailcap-search-path",
    "mail-check-interval",
    "metamail-charset",
    "mimetype-search-path",
    "newsrc-path",
    "news-active-file-path",
    "news-collections",
    "news-spool-directory",
    "nntp-server",
    "operator",
    "personal-name",
    "personal-print-category",
    "personal-print-command",
    "pine-score",
    "postponed-folder",
    "printer",
    "quell-extra-folders",
    "quell-filter-leader",
    "quell-news-forecast",
    "quell-newsrec-envelope",
    "quell-status-message",
    "read-message-folder",
    "reply-indent-string",
    "reply-leadin",
    "rsh-command",
    "rsh-open-timeout",
    "rsh-path",
    "scroll-text",
    "sendmail-path",
    "sending-filters",
    "sent-mail",
    "shared-prefix",
    "show-plain-text",
    "signature-at-bottom",
    "signature-file",
    "smime-adequate-hostnames",
    "smime-ca-cert-directory",
    "smime-ca-cert-file",
    "smime-private-cert-directory",
    "smime-private-cert-file",
    "smime-public-cert-directory",
    "smime-public-cert-file",
    "smime-security",
    "smtp-server",
    "speller",
    "status-message-delay",
    "tcp-open-timeout",
    "upload-command",
    "upload-command-prefix",
    "url-viewers",
    "use-only-domain-name",
    "user-domain",
    "user-id",
    "viewer-hdrs",
    "viewer-overlap",
    "version",
    "word-quotes",
];

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
        || k.starts_with("bugs-")
        || k.starts_with("rsh-")
        || k.starts_with("ssh-")
        || k.starts_with("smime-")
        || k.starts_with("quell-")
        || k.starts_with("feature-")
}

/// `b` が pinerc に見えるかを返す。
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
    keys >= 3
}

/// pinerc の統計。
#[derive(Debug, Default, Clone)]
pub struct Pinerc {
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を pinerc として統計する。
pub fn parse(b: &[u8]) -> Pinerc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Pinerc::default();
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
        let b = b"personal-name=X\nuser-domain=x.com\ninbox-path=INBOX\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"personal-name=X\nuser-domain=y\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# personal-name=X\n# user-domain=y\n# smtp-server=z\n";
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

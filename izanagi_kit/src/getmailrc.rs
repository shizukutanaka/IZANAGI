//! `getmailrc` 検出モジュール。
//!
//! getmail の設定は INI 風で、`[retriever]`/`[destination]`/
//! `[options]`/`[filter-...]`/`[filter_external]`/`[filter_classifier]`
//! セクションと `type = SimplePOP3Retriever`/
//! `SimpleIMAPSSLRetriever`/`SimplePOP3SSLRetriever`/`IMAPRetriever`/
//! `BrokenUIDLPOP3Retriever`/`MultidropPOP3Retriever`/`Maildir`/`Mboxrd`/
//! `MDA_external`/`Filter_external`/`Filter_classifier` 等の type 値、
//! `server`/`username`/`password`/`port`/`mailboxes`/`path`/
//! `unixfrom`/`delete`/`read_all`/`verbose`/`message_log`/
//! `delete_after`/`max_messages_per_session`/`timeout`/
//! `delete_bigger_than`/`max_bytes_per_session`/`delivered_to`/
//! `received`/`ignore_stderr`/`allow_root_commands`/`user`/`group`/
//! `local`/`command`/`exitcodes_drop`/`exitcodes_keep`/
//! `arguments`/`pass_all_parameters`/`pass_all_headers`/`users`/
//! `multiuser`/`keychain`/`movemail`/`use_netrc`/`ssl_version`/
//! `ca_certs`/`certfile`/`keyfile`/`ssl_ciphers`/`ssl_fingerprints`/
//! `envelope_sender`/`use_xoauth2`/`imap_on_error`/`pop3_delete_on_error`
//! 等のキーで構成される。
//!
//! ```
//! let b = b"[retriever]\n\
//!           type = SimplePOP3SSLRetriever\n\
//!           server = pop.example.com\n\
//!           username = me\n\
//!           password = secret\n\
//!           [destination]\n\
//!           type = Maildir\n\
//!           path = ~/Maildir/\n";
//! let c = izanagi_kit::getmailrc::parse(b);
//! assert!(izanagi_kit::getmailrc::detect(b));
//! assert_eq!(c.sections, 2);
//! ```

const SECTIONS: &[&str] = &[
    "[retriever]",
    "[destination]",
    "[options]",
    "[filter]",
    "[filter-1]",
    "[filter_external]",
    "[filter_classifier]",
    "[sorter]",
];

const KEYS: &[&str] = &[
    "allow_root_commands",
    "arguments",
    "ca_certs",
    "certfile",
    "command",
    "delivered_to",
    "delete",
    "delete_after",
    "delete_bigger_than",
    "envelope_sender",
    "exitcodes_drop",
    "exitcodes_keep",
    "explicit_envelope_to",
    "force_times",
    "group",
    "ignore_stderr",
    "imap_on_error",
    "keep_old_errors",
    "keychain",
    "keyfile",
    "local",
    "mailboxes",
    "max_bytes_per_session",
    "max_message_size",
    "max_messages_per_session",
    "mda",
    "message_log",
    "message_log_syslog",
    "message_log_verbose",
    "movemail",
    "multiuser",
    "pass_all_headers",
    "pass_all_parameters",
    "password",
    "path",
    "pop3_delete_on_error",
    "port",
    "postmaster",
    "preserve_dupes",
    "read_all",
    "received",
    "record_mailbox",
    "server",
    "skip_imap_fetch_size",
    "ssl_ciphers",
    "ssl_fingerprints",
    "ssl_version",
    "timeout",
    "to_oldmaildir",
    "type",
    "unixfrom",
    "use_netrc",
    "use_peek",
    "use_xoauth2",
    "user",
    "username",
    "users",
    "verbose",
    "verses",
];

const RETRIEVERS: &[&str] = &[
    "BrokenUIDLPOP3Retriever",
    "BrokenUIDLPOP3SSLRetriever",
    "IMAPRetriever",
    "IMAPSSLRetriever",
    "Maildir",
    "Mboxrd",
    "MDA_external",
    "MultidropIMAPRetriever",
    "MultidropIMAPSSLRetriever",
    "MultidropPOP3Retriever",
    "MultidropPOP3SSLRetriever",
    "MultidropSDPSRetriever",
    "POP3Retriever",
    "SimpleIMAPSSLRetriever",
    "SimpleIMAPRetriever",
    "SimplePOP3Retriever",
    "SimplePOP3SSLRetriever",
    "Filter_external",
    "Filter_classifier",
    "Filter_TMDA",
    "Filter_bogofilter",
    "Filter_spamassassin",
];

fn is_section(t: &str) -> bool {
    SECTIONS.contains(&t) || t.starts_with("[filter-")
}

fn is_retriever(v: &str) -> bool {
    let v = v.trim();
    RETRIEVERS.contains(&v)
}

fn key_present(t: &str) -> bool {
    if let Some((k, v)) = t.split_once('=') {
        let k = k.trim();
        if is_retriever(v) {
            return true;
        }
        KEYS.contains(&k)
    } else {
        false
    }
}

/// `b` が getmailrc に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if is_section(tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 4
}

/// getmailrc の統計。
#[derive(Debug, Default, Clone)]
pub struct Getmailrc {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を getmailrc として統計する。
pub fn parse(b: &[u8]) -> Getmailrc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Getmailrc::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if is_section(tr) {
            c.sections += 1;
        } else if key_present(tr) {
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
        let b = b"[retriever]\ntype = Maildir\nserver = x\nusername = y\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_type() {
        let b = b"type = SimplePOP3Retriever\nserver = x\nusername = y\npassword = z\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[retriever]\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"server = x\nusername = y\npassword = z\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}

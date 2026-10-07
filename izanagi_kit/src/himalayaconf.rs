//! himalaya `config.toml` 検出モジュール。
//!
//! himalaya(CLI メールクライアント)の設定は TOML 形式で、
//! `[accounts.<name>]` テーブルと `email`/`display-name`/`default`/
//! `backend.type`/`imap-host`/`imap-port`/`imap-login`/`imap-passwd`/
//! `imap-encryption`/`imap-starttls`/`smtp-host`/`smtp-port`/
//! `smtp-login`/`smtp-passwd`/`smtp-encryption`/`smtp-starttls`/
//! `sendmail.sendmail-cmd`/`notmuch-db-path`/`maildir.root-dir`/
//! `pgp`/`folder`/`sync`/`message`/`template`/`envelope`/`sync.folders`/
//! `account.discovery`/`oauth2`/`keyring`/`cmd`/`raw`/`query`/
//! `message.send.save-copy`/`message.delete.style`/`message.write.headers`
//! 等のキーで構成される。
//!
//! ```
//! let b = b"[accounts.main]\n\
//!           email = \"me@example.com\"\n\
//!           display-name = \"Taro\"\n\
//!           default = true\n\
//!           imap-host = \"imap.example.com\"\n\
//!           imap-port = 993\n\
//!           smtp-host = \"smtp.example.com\"\n\
//!           smtp-port = 465\n";
//! let c = izanagi_kit::himalayaconf::parse(b);
//! assert!(izanagi_kit::himalayaconf::detect(b));
//! assert_eq!(c.accounts, 1);
//! ```

const KEYS: &[&str] = &[
    "account.discovery",
    "backend",
    "backend.type",
    "cmd",
    "default",
    "display-name",
    "email",
    "envelope",
    "folder",
    "imap-auth",
    "imap-encryption",
    "imap-host",
    "imap-login",
    "imap-oauth2-access-token",
    "imap-oauth2-client-id",
    "imap-oauth2-client-secret",
    "imap-oauth2-method",
    "imap-oauth2-refresh-token",
    "imap-oauth2-scope",
    "imap-oauth2-token-url",
    "imap-passwd",
    "imap-port",
    "imap-starttls",
    "imap-watch-cmds",
    "imap-watch-timeout",
    "keyring",
    "maildir.root-dir",
    "message",
    "message.delete.style",
    "message.send.save-copy",
    "message.write.headers",
    "notmuch-db-path",
    "oauth2",
    "pgp",
    "query",
    "raw",
    "sendmail.sendmail-cmd",
    "smtp-auth",
    "smtp-encryption",
    "smtp-host",
    "smtp-login",
    "smtp-oauth2-access-token",
    "smtp-oauth2-client-id",
    "smtp-oauth2-client-secret",
    "smtp-oauth2-method",
    "smtp-oauth2-refresh-token",
    "smtp-oauth2-scope",
    "smtp-oauth2-token-url",
    "smtp-passwd",
    "smtp-port",
    "smtp-starttls",
    "sync",
    "sync.folders",
    "template",
];

fn is_account(t: &str) -> bool {
    t.starts_with("[accounts.") && t.ends_with(']')
}

fn is_key(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
        || k.starts_with("imap-")
        || k.starts_with("smtp-")
        || k.starts_with("message.")
        || k.starts_with("sync.")
        || k.starts_with("sendmail.")
        || k.starts_with("maildir.")
        || k.starts_with("account.")
        || k.starts_with("folder.")
}

/// `b` が himalaya 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut accts = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_account(tr) {
            accts += 1;
        } else if tr.contains('=') && is_key(tr) {
            keys += 1;
        }
    }
    (accts >= 1 && keys >= 2) || keys >= 4
}

/// himalaya 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct HimalayaConf {
    /// `[accounts.*]` テーブル数。
    pub accounts: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を himalaya 設定として統計する。
pub fn parse(b: &[u8]) -> HimalayaConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = HimalayaConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_account(tr) {
            c.accounts += 1;
        } else if tr.contains('=') && is_key(tr) {
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
        let b = b"[accounts.x]\nemail = \"a@b\"\nimap-host = \"h\"\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.accounts, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[accounts.x]\n"));
        assert!(!detect(b"[package]\nname = \"x\"\nversion = \"1\"\n"));
        assert!(!detect(b"key = value\nfoo = bar\n"));
    }

    #[test]
    fn keys_without_accounts() {
        let b = b"email = \"a@b\"\nimap-host = \"h\"\nsmtp-host = \"s\"\ndefault = true\n";
        assert!(detect(b));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.accounts, 0);
    }
}

//! rss2email 設定(config.cfg)の検出と構造カウント。
//!
//! `[DEFAULT]` と `[feed.<name>]`/`[feed.<url>]` セクションに分かれた
//! INI 形式。既知オプションを計数する。
//!
//! ```
//! let c = izanagi_kit::rss2email::parse(
//!     b"[DEFAULT]\nfrom = rss@example.com\nto = me@example.com\n[feed.https://ex.com/rss]\nactive = True\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.feeds, 1);
//! assert!(izanagi_kit::rss2email::detect(b"[feed.https://x/rss]\nactive = True\n"));
//! ```

use crate::textutil::strip_bom;
/// 既知 rss2email オプション。
const KEYS: &[&str] = &[
    "active",
    "bcc",
    "bonus-header",
    "date-header",
    "digest",
    "email-protocol",
    "encodings",
    "entry-timeout",
    "fetch-timeout",
    "from",
    "from-format",
    "guid-format",
    "html-mail",
    "imap-deliver",
    "imap-mailbox",
    "imap-port",
    "imap-server",
    "imap-ssl",
    "include-titles",
    "maildir",
    "mailer",
    "maildir-send",
    "name-format",
    "oauth2-auth",
    "other-header",
    "pause",
    "post-process",
    "process",
    "proxy",
    "reply-changes",
    "reply-to",
    "same-date-fetch",
    "sendmail",
    "sendmail-config",
    "smtp-auth",
    "smtp-password",
    "smtp-port",
    "smtp-server",
    "smtp-ssl",
    "smtp-user",
    "subject-format",
    "to",
    "trust-link",
    "trust-guid",
    "unix-mailbox",
    "use-8bit",
    "use-css",
    "use-publisher-email",
    "user-agent",
    "verbose",
];

/// rss2email 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクション数(`[DEFAULT]` + `[feed.*]`)。
    pub sections: usize,
    /// `[feed.*]` セクション数。
    pub feeds: usize,
    /// 既知オプション行。
    pub options: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が rss2email 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut feeds = 0;
    let mut opts = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("[feed.") {
            feeds += 1;
        } else if t.find('=').is_some_and(|p| KEYS.contains(&t[..p].trim())) {
            opts += 1;
        }
    }
    feeds >= 1 || opts >= 2
}

/// rss2email 設定の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        feeds: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            c.sections += 1;
            if t.starts_with("[feed.") {
                c.feeds += 1;
            }
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if KEYS.contains(&t[..pos].trim()) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[DEFAULT]\nfrom = rss@example.com\nto = me@example.com\nhtml-mail = True\nuse-8bit = False\n\n# feeds\n[feed.https://news.example.com/rss]\nactive = True\ndigest = False\n\n[feed.https://blog.example.org/atom]\nactive = True\nname-format = {feed-title}\n";

    #[test]
    fn rss2email() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.feeds, 2);
        assert_eq!(c.options, 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_rss2email() {
        assert!(!detect(b"[section]\nkey = value\n"));
        assert!(!detect(b"hello\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

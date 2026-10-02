//! newsboat/newsbeuter 設定(~/.newsboat/config)の検出と構造カウント。
//!
//! `key value` 形式(~= も可)。`bind-key`/`unbind-key`/`macro`・`color`/`highlight`
//! を別枠で分類する。
//!
//! ```
//! let c = izanagi_kit::newsboat::parse(
//!     b"browser firefox\nbind-key j next\nmax-items 50\n").unwrap();
//! assert_eq!(c.bindings, 1);
//! assert_eq!(c.options, 2);
//! assert!(izanagi_kit::newsboat::detect(b"browser lynx\nreload-time 30\nauto-reload yes\n"));
//! ```

/// 既知 newsboat 設定ディレクティブ。
const KEYS: &[&str] = &[
    "always-display-description",
    "article-sort-order",
    "auto-reload",
    "cache-file",
    "cleanup-on-quit",
    "confirm-delete",
    "confirm-mark-all-feeds-read",
    "datetime-format",
    "delete-read-articles-on-quit",
    "dialogs-title-format",
    "display-article-progress",
    "download-path",
    "error-log",
    "external-url-viewer",
    "feed-sort-order",
    "feedlist-format",
    "feedlist-title-format",
    "first-run",
    "goto-first-unread",
    "goto-next-feed",
    "history-limit",
    "ignore-http-version",
    "ignore-mode",
    "itemlist-format",
    "itemlist-title-format",
    "keep-articles-days",
    "max-downloads",
    "max-items",
    "notify-always",
    "notify-beep",
    "notify-format",
    "notify-program",
    "notify-screen",
    "notify-xterm",
    "oldbrowser",
    "openbrowser-and-mark-jumps-to-next-unread",
    "podcache-auto-enqueue",
    "prepopulate-query-feeds",
    "proxy",
    "proxy-auth",
    "proxy-auth-method",
    "proxy-type",
    "refresh-on-startup",
    "reload-only-visible-feeds",
    "reload-time",
    "reload-threads",
    "run-on-startup",
    "save-path",
    "scrolloff",
    "search-highlight-colors",
    "show-keymap-hint",
    "show-read-feeds",
    "show-read-articles",
    "ssl-verifyhost",
    "ssl-verifypeer",
    "suppress-first-reload",
    "swap-title-and-hints",
    "text-width",
    "toggleitemread-jumps-to-next-unread",
    "ttrss-api",
    "ttrss-flag-publish",
    "ttrss-flag-star",
    "ttrss-login",
    "ttrss-mode",
    "ttrss-password",
    "ttrss-url",
    "urls-source",
    "use-proxy",
    "user-agent",
    "wrap-scroll",
    "browser",
    "cookie-cache",
    "download-timeout",
    "feedhq-flag-share",
    "feedhq-flag-star",
    "feedhq-login",
    "feedhq-password",
    "feedhq-url",
    "inoreader-app-id",
    "inoreader-app-key",
    "inoreader-flag-share",
    "inoreader-flag-star",
    "inoreader-login",
    "inoreader-password",
    "miniflux-login",
    "miniflux-password",
    "miniflux-token",
    "miniflux-url",
    "newsblur-login",
    "newsblur-password",
    "newsblur-url",
    "ocnews-flag-star",
    "ocnews-login",
    "ocnews-password",
    "ocnews-server",
    "oldreader-flag-share",
    "oldreader-flag-star",
    "oldreader-login",
    "oldreader-password",
    "theoldreader-flag-share",
    "theoldreader-flag-star",
    "theoldreader-login",
    "theoldreader-password",
];
/// バインド・外観系ディレクティブ。
const BINDINGS: &[&str] = &["bind-key", "unbind-key", "macro"];
/// 色・ハイライト系ディレクティブ。
const STYLES: &[&str] = &["color", "highlight", "highlight-article"];

/// newsboat 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知オプション行。
    pub options: usize,
    /// `bind-key`/`unbind-key`/`macro`。
    pub bindings: usize,
    /// `color`/`highlight*`。
    pub styles: usize,
    /// `define-filter`/`prepopulate-query-feeds` 外の filter 系。
    pub filters: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が newsboat 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            let t = l.trim();
            let h = t.split_whitespace().next().unwrap_or("");
            KEYS.contains(&h) || BINDINGS.contains(&h) || STYLES.contains(&h)
        })
        .count()
        >= 3
}

/// newsboat 設定の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        bindings: 0,
        styles: 0,
        filters: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let h = t.split_whitespace().next().unwrap_or("");
        if BINDINGS.contains(&h) {
            c.bindings += 1;
        } else if STYLES.contains(&h) {
            c.styles += 1;
        } else if h == "define-filter" || h == "reset-unread-on-update" {
            c.filters += 1;
        } else if KEYS.contains(&h) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options + c.bindings + c.styles >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# newsboat config\nbrowser firefox\ndownload-path \"~/dl\"\nmax-items 50\nreload-time 30\nauto-reload yes\nshow-read-feeds no\n\nbind-key j next\nbind-key k prev\nunbind-key g\nmacro i set browser \"mpv\"; open-in-browser\n\ncolor info default default reverse\nhighlight feedlist \"^[0-9]+ +N \" magenta default bold\ndefine-filter \"unread\" \"unread == \\\"yes\\\"\"\n\nunknown-key value\n";

    #[test]
    fn newsboat() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 6);
        assert_eq!(c.bindings, 4);
        assert_eq!(c.styles, 2);
        assert_eq!(c.filters, 1);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_newsboat() {
        assert!(!detect(b"key = value\n[section]\n"));
        assert!(!detect(b"hello world\n"));
    }
}

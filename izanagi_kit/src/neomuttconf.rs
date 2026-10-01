//! NeoMutt `.neomuttrc` の検出・カウント。
//!
//! mutt 構文を継承しつつ `sidebar_*`/`nm_*`/`ts_*` 変数、
//! `virtual-mailboxes`/`named-mailboxes`/`lua` 等 NeoMutt 固有コマンドを持つ。
//!
//! ```
//! let cfg = b"set mail_check = 30\nset sidebar_visible = yes\n\
//!             set sidebar_width = 30\nset nm_query_type = \"threads\"\n\
//!             virtual-mailboxes \"INBOX\" \"notmuch://?query=tag:inbox\"\n\
//!             named-mailboxes \"Work\" =inbox.work\n";
//! assert!(izanagi_kit::neomuttconf::detect(cfg));
//! let c = izanagi_kit::neomuttconf::parse(cfg).unwrap();
//! assert_eq!(c.lines, 6);
//! assert_eq!(c.neomutt_entries, 5);
//! assert_eq!(c.sidebar_entries, 2);
//! ```

/// NeoMutt 固有の変数/コマンド名。
const NEOMUTT: &[&str] = &[
    "virtual-mailboxes",
    "named-mailboxes",
    "unvirtual-mailboxes",
    "lua",
    "lua-source",
    "tags",
    "tags-transformed",
    "tag-formats",
    "tag-transforms",
    "sidebar_whitelist",
    "unsidebar_whitelist",
    "sidebar_pin",
    "sidebar_unpin",
];

/// 変数名が NeoMutt 固有プレフィックスを持つか。
fn is_neomutt_var(v: &str) -> bool {
    v.starts_with("sidebar_") || v.starts_with("nm_") || v.starts_with("ts_")
}

/// NeoMutt 以外 (mutt 互換) の代表的コマンド。
const MUTT_COMMANDS: &[&str] = &[
    "set",
    "unset",
    "reset",
    "toggle",
    "bind",
    "unbind",
    "macro",
    "color",
    "uncolor",
    "mailboxes",
    "my_hdr",
    "alias",
    "alternates",
    "source",
    "exec",
    "push",
    "score",
    "spam",
    "nospam",
    "group",
    "ungroup",
    "lists",
    "subscribe",
    "unsubscribe",
    "ignore",
    "unignore",
    "mono",
    "finish",
];

fn is_hook(cmd: &str) -> bool {
    cmd.ends_with("-hook")
}

/// `set`/`unset` 行から変数名を取り出す (`set opt = x` / `unset opt` / `set "opt=x"`)。
fn var_of<'a>(s: &'a str, cmd: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(cmd)?.trim_start();
    let rest = rest.trim_start_matches('"');
    let end = rest.find([' ', '\t', '=']).unwrap_or(rest.len());
    let v = &rest[..end];
    if v.is_empty() {
        return None;
    }
    Some(v)
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知コマンド行数。
    pub lines: usize,
    /// `set`/`unset`/`reset`/`toggle` 行数。
    pub set_lines: usize,
    /// NeoMutt 固有コマンド/変数を含む行数。
    pub neomutt_entries: usize,
    /// `sidebar_*` 変数行数。
    pub sidebar_entries: usize,
    /// `virtual-mailboxes`/`named-mailboxes`/`mailboxes`/`unmailboxes` 行数。
    pub mailbox_lines: usize,
    /// `*-hook` 行数。
    pub hook_lines: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// `b` が `.neomuttrc` 形式かどうか (NeoMutt 固有要素が必須)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.lines >= 3 && c.neomutt_entries >= 2
}

/// `b` を `.neomuttrc` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        set_lines: 0,
        neomutt_entries: 0,
        sidebar_entries: 0,
        mailbox_lines: 0,
        hook_lines: 0,
        comments: 0,
    };
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let cmd = s.split([' ', '\t']).next().unwrap_or("");
        let known = MUTT_COMMANDS.contains(&cmd) || NEOMUTT.contains(&cmd) || is_hook(cmd);
        if !known {
            continue;
        }
        c.lines += 1;
        if matches!(cmd, "set" | "unset" | "reset" | "toggle") {
            c.set_lines += 1;
            if let Some(v) = var_of(s, cmd) {
                if is_neomutt_var(v) {
                    c.neomutt_entries += 1;
                    if v.starts_with("sidebar_") {
                        c.sidebar_entries += 1;
                    }
                }
            }
        }
        if NEOMUTT.contains(&cmd) {
            c.neomutt_entries += 1;
        }
        if matches!(
            cmd,
            "mailboxes"
                | "virtual-mailboxes"
                | "named-mailboxes"
                | "unmailboxes"
                | "unvirtual-mailboxes"
        ) {
            c.mailbox_lines += 1;
        }
        if is_hook(cmd) {
            c.hook_lines += 1;
        }
    }
    if c.lines == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_neomuttrc() {
        let cfg = b"set folder = \"~/Mail\"\nset sidebar_visible = yes\n\
                    set sidebar_divider_char = \"|\"\nset nm_db_limit = 300\n\
                    virtual-mailboxes \"All Mail\" \"notmuch://?query=tag:all\"\n\
                    folder-hook . 'set sort=reverse-date'\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.lines, 6);
        assert_eq!(c.neomutt_entries, 4);
        assert_eq!(c.sidebar_entries, 2);
        assert_eq!(c.mailbox_lines, 1);
        assert_eq!(c.hook_lines, 1);
    }

    #[test]
    fn rejects_plain_mutt() {
        // mutt 構文だけでは NeoMutt と判定しない
        assert!(!detect(
            b"set folder = \"~/Mail\"\nset editor = vim\nbind index gg first-entry\n"
        ));
    }
}

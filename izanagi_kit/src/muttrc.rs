//! Mutt `.muttrc`/`muttrc` の検出・カウント。
//!
//! `set opt = value`/`unset opt`/`bind`/`macro`/`color`/`*-hook`/`source`/
//! `my_hdr`/`mailboxes`/`alias` 等のコマンド行からなる設定ファイル。
//!
//! ```
//! let cfg = b"set folder = \"~/Mail\"\nset mbox_type = Maildir\n\
//!             set from = \"me@example.com\"\nunset beep\n\
//!             bind index gg first-entry\nmacro index <f2> \"<sync-mailbox>\"\n\
//!             color status green black\nfolder-hook . 'set sort=threads'\n";
//! assert!(izanagi_kit::muttrc::detect(cfg));
//! let c = izanagi_kit::muttrc::parse(cfg).unwrap();
//! assert_eq!(c.lines, 8);
//! assert_eq!(c.set_lines, 4);
//! assert_eq!(c.hook_lines, 1);
//! ```

/// muttrc の先頭語として知られるコマンド。
const COMMANDS: &[&str] = &[
    "set",
    "unset",
    "reset",
    "toggle",
    "bind",
    "unbind",
    "macro",
    "color",
    "uncolor",
    "mono",
    "mailboxes",
    "named-mailboxes",
    "virtual-mailboxes",
    "unmailboxes",
    "my_hdr",
    "unmy_hdr",
    "alias",
    "unalias",
    "alternates",
    "unalternates",
    "alternative_order",
    "unalternative_order",
    "auto_view",
    "unauto_view",
    "mime_lookup",
    "unmime_lookup",
    "mailcap",
    "attachments",
    "unattachments",
    "mailto_allow",
    "unmailto_allow",
    "subscribe",
    "unsubscribe",
    "lists",
    "unlists",
    "group",
    "ungroup",
    "score",
    "unscore",
    "spam",
    "nospam",
    "source",
    "exec",
    "push",
    "finish",
    "cd",
    "fcc_attach",
    "fcc_clear",
    "tag-formats",
    "tag-transforms",
    "subjectrx",
    "unsubjectrx",
    "echo",
    "if",
    "else",
    "endif",
    "lua",
    "lua-source",
    "ignore",
    "unignore",
    "hdr_order",
    "unhdr_order",
    "sidebar_whitelist",
    "unsidebar_whitelist",
    "sidebar_pin",
    "sidebar_unpin",
    "crypt_autoencrypt",
    "crypt_autosign",
];

/// `*-hook` 系コマンドの接尾辞。
fn is_hook(cmd: &str) -> bool {
    cmd.ends_with("-hook")
}

fn head(s: &str) -> &str {
    s.split([' ', '\t']).next().unwrap_or("")
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知コマンド行数。
    pub lines: usize,
    /// `set`/`unset`/`reset`/`toggle` 行数。
    pub set_lines: usize,
    /// `bind`/`unbind` キーバインド行数。
    pub bind_lines: usize,
    /// `macro` 行数。
    pub macro_lines: usize,
    /// `color`/`uncolor`/`mono` 行数。
    pub color_lines: usize,
    /// `*-hook` 行数。
    pub hook_lines: usize,
    /// `source`/`mailcap`/`lua-source` 外部読み込み行数。
    pub source_lines: usize,
    /// `mailboxes`/`named-mailboxes`/`virtual-mailboxes`/`alias` 行数。
    pub mailbox_lines: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// `b` が `.muttrc` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.lines >= 3
}

/// `b` を `.muttrc` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        set_lines: 0,
        bind_lines: 0,
        macro_lines: 0,
        color_lines: 0,
        hook_lines: 0,
        source_lines: 0,
        mailbox_lines: 0,
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
        let cmd = head(s);
        let known = COMMANDS.contains(&cmd) || is_hook(cmd);
        if !known {
            continue;
        }
        c.lines += 1;
        match cmd {
            "set" | "unset" | "reset" | "toggle" => c.set_lines += 1,
            "bind" | "unbind" => c.bind_lines += 1,
            "macro" => c.macro_lines += 1,
            "color" | "uncolor" | "mono" => c.color_lines += 1,
            "source" | "mailcap" | "lua-source" => c.source_lines += 1,
            "mailboxes" | "named-mailboxes" | "virtual-mailboxes" | "unmailboxes" | "alias"
            | "unalias" => c.mailbox_lines += 1,
            _ => {}
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
    fn detects_muttrc() {
        let cfg = b"# mutt config\nset editor = \"vim\"\nset sendmail = \"/usr/sbin/sendmail\"\n\
                    set ssl_starttls = yes\nset ssl_force_tls = yes\n\
                    bind editor <Tab> complete-query\nmacro index G \"<shell-escape>!mbsync -a<enter>\"\n\
                    color hdrdefault cyan default\nsource ~/.mutt/accounts/one.muttrc\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.lines, 8);
        assert_eq!(c.set_lines, 4);
        assert_eq!(c.bind_lines, 1);
        assert_eq!(c.macro_lines, 1);
        assert_eq!(c.color_lines, 1);
        assert_eq!(c.source_lines, 1);
    }

    #[test]
    fn rejects_shell() {
        assert!(!detect(b"export PATH=$PATH:/usr/bin\necho hello\nls -la\n"));
    }
}

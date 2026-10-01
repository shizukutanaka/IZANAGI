//! WeeChat `*.conf`(weechat.conf, irc.conf, alias.conf …)— `[section]` +
//! `key = value`、ドット階層キー (`server.libera.addresses`) を持つ形式。
//!
//! ```
//! let cfg = b"[look]\nsave_config_on_exit = on\nprefix_error = \"*\"\n[network]\nconnection_timeout = 60\n[server]\nlibera.addresses = \"irc.libera.chat/6697\"\nlibera.autoconnect = on\nlibera.nicks = \"shizuku\"\n";
//! assert!(izanagi_kit::weechat::detect(cfg));
//! let c = izanagi_kit::weechat::parse(cfg).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.dotted_keys, 3);
//! ```

/// WeeChat conf の代表的なセクション名。
const KNOWN_SECTIONS: &[&str] = &[
    "look",
    "palette",
    "color",
    "completion",
    "history",
    "proxy",
    "network",
    "server",
    "ctcp",
    "typing",
    "layout",
    "filters",
    "notify",
    "keys",
    "spell",
    "charset",
    "fifo",
    "logger",
    "plugins",
    "relay",
    "script",
    "xfer",
    "alias",
    "buflist",
    "exec",
    "fset",
    "guile",
    "irc",
    "javascript",
    "lua",
    "perl",
    "php",
    "python",
    "ruby",
    "tcl",
    "trigger",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `[name]` セクション数。
    pub sections: usize,
    /// 既知名のセクション数。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// `.` を含むキー数。
    pub dotted_keys: usize,
    /// `on`/`off`/`true`/`false` 値の行数。
    pub bool_values: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// WeeChat conf らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.sections >= 1 && c.entries >= 3 && (c.known_sections >= 1 || c.dotted_keys >= 3)
}

/// 行を走査して集計する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        sections: 0,
        known_sections: 0,
        entries: 0,
        dotted_keys: 0,
        bool_values: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
            let name = &t[1..t.len() - 1];
            if name
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-')
            {
                c.sections += 1;
                if KNOWN_SECTIONS.contains(&name) {
                    c.known_sections += 1;
                }
                continue;
            }
        }
        if let Some((key, val)) = t.split_once('=') {
            let key = key.trim();
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'.' || ch == b'-')
            {
                continue;
            }
            c.entries += 1;
            if key.contains('.') {
                c.dotted_keys += 1;
            }
            let v = val.trim().trim_matches('"');
            if matches!(v, "on" | "off" | "true" | "false" | "yes" | "no") {
                c.bool_values += 1;
            }
        }
    }
    if c.entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# weechat\n[look]\nsave_config_on_exit = on\nprefix_error = \"*\"\nbuffer_notify_default = message\n[network]\nconnection_timeout = 60\n[server]\nlibera.addresses = \"irc.libera.chat/6697\"\nlibera.autoconnect = on\nlibera.realname = \"shizuku\"\n";

    #[test]
    fn detects_weechat() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.entries, 7);
        assert_eq!(c.dotted_keys, 3);
        assert_eq!(c.bool_values, 2);
    }

    #[test]
    fn rejects_plain_ini() {
        assert!(!detect(b"[aaa]\nx = 1\ny = 2\nz = 3\n"));
        assert!(!detect(b"key=value\n"));
    }
}

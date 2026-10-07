//! `greetd`/`gtkgreet`/`tuigreet`/`regreet`/`wlgreet` 設定検出モジュール。
//!
//! greetd `config.toml` は `[terminal]`/`[default_session]`/
//! `[initial_session]` セクションと `vt`/`command`/`user`/`agreety`/
//! `switch`/`service`/`shell`/`note`/`xsessions`/`xfsettings`/
//! `sessions`/`remember`/`remember_session`/`remember_user_session`
//! 等のキーで構成される(greeter側設定も同形式)。
//!
//! ```
//! let b = b"[terminal]\n\
//!           vt = 1\n\
//!           switch = true\n\
//!           [default_session]\n\
//!           command = \"agreety --cmd sway\"\n\
//!           user = \"greeter\"\n";
//! let c = izanagi_kit::greetd::parse(b);
//! assert!(izanagi_kit::greetd::detect(b));
//! assert_eq!(c.sections, 2);
//! ```

const SECTIONS: &[&str] = &[
    "[terminal]",
    "[default_session]",
    "[initial_session]",
    "[general]",
    "[greeter]",
    "[gtk]",
    "[Gtk]",
    "[theme]",
];

const KEYS: &[&str] = &[
    "agreety",
    "application_prefer_dark_mode",
    "background",
    "command",
    "cursor_theme_name",
    "cursor_theme_size",
    "environments",
    "exec-cmd",
    "greeter",
    "greeter-session",
    "greeter-supplementary-groups",
    "gtk-application-prefer-dark-theme",
    "gtk-cursor-theme-name",
    "gtk-cursor-theme-size",
    "gtk-font-name",
    "gtk-icon-theme-name",
    "gtk-theme-name",
    "icon_theme_name",
    "initial_session",
    "initial_sessions",
    "lang",
    "layer_mode",
    "output-mode",
    "remember",
    "remember_session",
    "remember_user_session",
    "service",
    "sessions",
    "shell",
    "switch",
    "terminal",
    "time",
    "time-format",
    "time_format",
    "user",
    "user_session",
    "vt",
    "window-padding",
    "xedge",
    "xoffset",
    "xsessions",
    "xfsettings",
    "yedge",
    "yoffset",
];

fn is_section(t: &str) -> bool {
    SECTIONS.contains(&t)
}

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が greetd 系設定に見えるかを返す。
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
    (secs >= 1 && keys >= 2) || (secs >= 2 && keys >= 1) || keys >= 4
}

/// greetd 系設定の統計。
#[derive(Debug, Default, Clone)]
pub struct Greetd {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を greetd 系設定として統計する。
pub fn parse(b: &[u8]) -> Greetd {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Greetd::default();
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
        let b = b"[terminal]\nvt = 1\nswitch = true\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn detects_session() {
        let b = b"[default_session]\ncommand = \"sway\"\nuser = \"greeter\"\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[terminal]\n"));
        assert!(!detect(b"[terminal]\nkey = x\n"));
        assert!(!detect(b"command = \"x\"\nuser = \"y\"\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}

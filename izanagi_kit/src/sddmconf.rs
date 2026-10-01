//! SDDM 設定ファイル (`sddm.conf`, `sddm.conf.d/*.conf`) パーサ。
//!
//! `[General]`/`[Theme]`/`[Users]`/`[Wayland]`/`[X11]`/`[Autologin]` セクションと
//! `HaltCommand`/`Numlock`/`Current`/`MinimumUid`/`VirtualTerminal` 等キーを計数する。
//!
//! ```
//! use izanagi_kit::sddmconf;
//! let conf = b"[Theme]\nCurrent = breeze\nCursorTheme = breeze\nFont = Noto Sans\n";
//! assert!(sddmconf::detect(conf));
//! let c = sddmconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 1);
//! assert_eq!(c.known_keys, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_SECTIONS: &[&str] = &["General", "Theme", "Users", "Wayland", "X11", "Autologin"];

const KNOWN_KEYS: &[&str] = &[
    "HaltCommand",
    "RebootCommand",
    "Numlock",
    "InputMethod",
    "GreeterEnvironment",
    "EnableAvatars",
    "FacesDir",
    "ThemeDir",
    "CursorTheme",
    "CursorSize",
    "CursorAnimationDuration",
    "Current",
    "Font",
    "FontSize",
    "Background",
    "EnableHiDPI",
    "Version",
    "MinimumUid",
    "MaximumUid",
    "HideUsers",
    "HideShells",
    "RememberLastUser",
    "RememberLastSession",
    "SessionDir",
    "SessionsDir",
    "UserIconFile",
    "SessionLogFile",
    "DisplayCommand",
    "DisplayStopCommand",
    "ServerArguments",
    "XephyrPath",
    "XauthPath",
    "XDisplayServer",
    "VirtualTerminal",
    "CompositorCommand",
    "User",
    "Session",
    "Relogin",
    "ReuseSession",
    "DefaultPath",
    "Namespaces",
];

/// 簡易判定 (既知セクション + 既知キー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.known_sections >= 2 && c.known_keys >= 2) || (c.known_sections >= 1 && c.known_keys >= 3)
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    let mut found = false;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
            c.sections += 1;
            if KNOWN_SECTIONS.contains(&t[1..t.len() - 1].trim()) {
                c.known_sections += 1;
            }
            found = true;
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim();
        if key.is_empty() || key.bytes().any(|b| b == b' ') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            c.known_keys += 1;
        }
        found = true;
    }
    found.then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"[General]\nHaltCommand = /usr/bin/systemctl poweroff\nRebootCommand = /usr/bin/systemctl reboot\nNumlock = on\n\n[Theme]\nCurrent = breeze\nCursorTheme = breeze\n\n[Users]\nMinimumUid = 1000\nMaximumUid = 60513\nHideUsers = git\n\n[Autologin]\nUser = alice\nSession = plasma.desktop\n";

    #[test]
    fn detects_sddmconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.known_sections, 4);
        assert_eq!(c.entries, 10);
        assert_eq!(c.known_keys, 10);
    }

    #[test]
    fn rejects_generic_ini() {
        assert!(!detect(b"[General]\nfoo = 1\nbar = 2\n"));
        assert!(!detect(b"[Theme]\nbackground = x\naccent = y\n"));
    }
}

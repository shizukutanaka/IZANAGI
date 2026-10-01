//! Weston コンポジタ設定ファイル (`weston.ini`) パーサ。
//!
//! `[core]`/`[shell]`/`[output]`/`[input-device]`/`[keyboard]`/`[terminal]`/
//! `[launcher]`/`[screen-share]`/`[xwayland]`/`[libinput]`/`[remote]`/`[remoting]`
//! セクションと `key = value` 行を計数する。
//!
//! ```
//! use izanagi_kit::westonconf;
//! let conf = b"[core]\nxwayland = true\n[shell]\npanel-position = bottom\n";
//! assert!(westonconf::detect(conf));
//! let c = westonconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 2);
//! assert_eq!(c.entries, 2);
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

const KNOWN_SECTIONS: &[&str] = &[
    "core",
    "shell",
    "output",
    "input-device",
    "keyboard",
    "terminal",
    "launcher",
    "screen-share",
    "xwayland",
    "libinput",
    "remote",
    "remoting",
];

const KNOWN_KEYS: &[&str] = &[
    "modules",
    "backend",
    "xwayland",
    "shell",
    "panel-position",
    "locking",
    "animation",
    "startup-animation",
    "cursor-theme",
    "cursor-size",
    "keymap_rules",
    "keymap_model",
    "keymap_layout",
    "keymap_variant",
    "keymap_options",
    "numlock-on",
    "vt-switching",
    "idle-time",
    "icon",
    "mode",
    "transform",
    "scale",
    "seat",
    "name",
    "command",
    "path",
    "drm",
    "host",
    "port",
    "type",
    "enable",
    "disable",
    "color",
    "button",
    "environment",
    "force-on",
    "same-as",
    "position",
    "font",
    "term",
    "accel-profile",
    "constant-deceleration",
    "natural-scrolling",
    "left-handed",
];

/// 簡易判定 (既知セクション + 既知キー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_sections >= 2 && c.known_keys >= 1
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

    const SAMPLE: &[u8] = b"[core]\nxwayland = true\nmodules = xwayland.so,cms-colord.so\nshell = desktop-shell.so\n\n[shell]\npanel-position = top\nlocking = false\nanimation = fade\n\n[output]\nname = LVDS1\nmode = 1600x900\ntransform = 90\n\n[keyboard]\nkeymap_model = pc105\nkeymap_layout = jp\n\n[terminal]\nfont = monospace\n";

    #[test]
    fn detects_westonconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.known_sections, 5);
        assert_eq!(c.entries, 12);
        assert_eq!(c.known_keys, 12);
    }

    #[test]
    fn rejects_generic_ini() {
        assert!(!detect(b"[main]\nfoo = 1\n[server]\nhost = x\n"));
        assert!(!detect(b"[core]\nbar = 1\n"));
    }
}

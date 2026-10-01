//! foot (Wayland ターミナル) `foot.ini` パーサ。
//!
//! `[main]`/`[colors]`/`[cursor]`/`[scrollback]`/`[mouse]`/`[key-bindings]`/
//! `[search-bindings]`/`[url-bindings]`/`[tweak]`/`[csd]`/`[bell]` セクションと
//! `font`/`shell`/`term`/`alpha`/`regular`/`blink`/`scrollback` 等既知キーを計数する。
//!
//! ```
//! use izanagi_kit::footconf;
//! let conf = b"[main]\nfont = monospace:size=12\nterm = foot\n[colors]\nalpha = 0.9\nregular0 = 000000\n";
//! assert!(footconf::detect(conf));
//! let c = footconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 2);
//! assert_eq!(c.known_keys, 4);
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
    "main",
    "scrollback",
    "url",
    "cursor",
    "mouse",
    "colors",
    "colors-dark",
    "colors-light",
    "csd",
    "bell",
    "tweak",
    "key-bindings",
    "search-bindings",
    "url-bindings",
    "text-bindings",
    "mouse-bindings",
];

const KNOWN_KEYS: &[&str] = &[
    "font",
    "font-bold",
    "font-italic",
    "font-bold-italic",
    "font-size-adjustment",
    "line-height",
    "letter-spacing",
    "horizontal-letter-offset",
    "vertical-letter-offset",
    "underline-offset",
    "underline-thickness",
    "strikeout-thickness",
    "box-drawings-uses-font-glyphs",
    "dpi-aware",
    "pad",
    "resize-delay-ms",
    "initial-window-size-pixels",
    "initial-window-size-chars",
    "initial-window-mode",
    "title",
    "locked-title",
    "shell",
    "login-shell",
    "term",
    "utmp-helper",
    "app-id",
    "workers",
    "enabled",
    "multiplier",
    "indicator-position",
    "style",
    "blink",
    "color",
    "beam-thickness",
    "underline-thickness",
    "hide-when-typing",
    "alternate-scroll-mode",
    "selection-override-modifier",
    "alpha",
    "background",
    "foreground",
    "flash",
    "regular0",
    "regular1",
    "regular2",
    "regular3",
    "regular4",
    "regular5",
    "regular6",
    "regular7",
    "bright0",
    "bright1",
    "bright2",
    "bright3",
    "bright4",
    "bright5",
    "bright6",
    "bright7",
    "selection-foreground",
    "selection-background",
    "jump-labels",
    "scrollback-indicator",
    "search-box-no-match",
    "search-box-match",
    "urls",
    "flash-alpha",
    "size",
    "border-width",
    "border-color",
    "color",
    "location",
    "button-width",
    "button-color",
    "button-cancel-color",
    "button-minimize-color",
    "button-maximize-color",
    "button-close-color",
    "alpha-mode",
    "notifications",
    "utf8",
    "sixel",
    "bold-text-in-bright",
    "dim",
    "bell-command",
    "bell-command-focused",
    "urgent",
    "notify",
    "visual",
    "command",
    "osc52-paste",
    "clipboard-write",
    "primary-paste",
    "show-urls-launch",
    "show-urls-copy",
    "show-urls-persistent",
    "prompt-scroll",
    "scrollback-output",
    "fullscreen",
    "pipe-visible",
    "pipe-scrollback",
    "pipe-selected",
    "pipe-command-output",
    "search-start",
    "find-next",
    "find-prev",
    "cursor-left",
    "cursor-right",
    "cursor-up",
    "cursor-down",
    "cursor-home",
    "cursor-end",
    "cursor-word-left",
    "cursor-word-right",
    "clipboard-copy",
    "clipboard-paste",
    "primary-copy",
    "primary-paste",
    "unicode-input",
    "noop",
    "quit",
];

/// `foot.ini` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => (c.known_sections >= 1 && c.known_keys >= 2) || c.known_keys >= 3,
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            c.sections += 1;
            let name = t[1..t.len() - 1].trim().to_ascii_lowercase();
            if KNOWN_SECTIONS.contains(&name.as_str()) {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim().to_ascii_lowercase();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
    }
    (c.entries > 0 || c.sections > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[main]\nfont = monospace:size=12\nfont-bold = monospace:size=12:weight=bold\nterm = foot\nshell = /bin/zsh\npad = 4x4\nworkers = 4\n[colors]\nalpha = 0.9\nregular0 = 000000\nbright7 = ffffff\n[cursor]\nstyle = beam\nblink = yes\n";

    #[test]
    fn detects_footini() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.entries, 11);
        assert_eq!(c.known_keys, 11);
    }

    #[test]
    fn rejects_generic_ini() {
        let ini = b"[section]\nfoo = bar\nbaz = 1\n";
        assert!(!detect(ini));
    }
}

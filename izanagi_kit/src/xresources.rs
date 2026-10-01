//! X リソースファイル (`.Xresources`, `Xdefaults`, `app-defaults/*`) パーサ。
//!
//! `Name.Class*resource: value` 形式の行と `!` コメント、
//! cpp プリプロセッサ (`#define`/`#include`/`#if`) 行を計数する。
//!
//! ```
//! use izanagi_kit::xresources;
//! let res = b"! comment\nXft.dpi: 96\nxterm*faceName: DejaVu Sans Mono\n*background: #1d1f21\n";
//! assert!(xresources::detect(res));
//! let c = xresources::parse(res).unwrap();
//! assert_eq!(c.entries, 3);
//! assert_eq!(c.comments, 1);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `name: value` リソース行数。
    pub entries: usize,
    /// キーに `*`/`?` を含む行数。
    pub starred: usize,
    /// `!` コメント行数。
    pub comments: usize,
    /// cpp ディレクティブ (`#define`/`#include`/`#if*`/`#endif` 等) 行数。
    pub cpp: usize,
    /// 既知リソース接頭辞 (`Xft.`/`xterm*`/`urxvt*`/`rofi.`/`*color*` 等) に一致する行数。
    pub known: usize,
}

const KNOWN_PREFIXES: &[&str] = &[
    "Xft.",
    "Xft*",
    "xterm*",
    "xterm.",
    "XTerm",
    "urxvt*",
    "URxvt",
    "Emacs",
    "rofi.",
    "rofi*",
    "dmenu.",
    "xscreensaver",
    "XScreenSaver",
    "Xcursor.",
    "Sxiv.",
    "Nsxiv.",
    "Rofi.",
    "i3wm.",
    "*background",
    "*foreground",
    "*color",
    "*font",
    "*faceName",
    "*scroll",
    "*cursor",
    "*border",
    "*geometry",
    "*title",
    "*login",
    "*multi",
    "*visual",
    "*icon",
    "*saveLines",
    "*termName",
    "*dynamicColors",
    "*cursorColor",
    "*pointerColor",
    "*highlightColor",
];

const CPP_WORDS: &[&str] = &[
    "#define", "#include", "#if", "#ifdef", "#ifndef", "#else", "#elif", "#endif", "#undef",
];

/// 簡易判定 (`name: value` 行 + `*`/`?`/cpp/既知接頭辞)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.entries >= 3 && (c.starred >= 1 || c.known >= 1 || c.cpp >= 1)
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        starred: 0,
        comments: 0,
        cpp: 0,
        known: 0,
    };
    let mut found = false;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('!') {
            c.comments += 1;
            found = true;
            continue;
        }
        if t.starts_with('#') {
            if CPP_WORDS.iter().any(|w| t.starts_with(w)) {
                c.cpp += 1;
            }
            found = true;
            continue;
        }
        let Some(colon) = t.find(':') else {
            continue;
        };
        let key = t[..colon].trim_end();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'*' | b'?' | b'.' | b'-' | b'_'))
        {
            continue;
        }
        c.entries += 1;
        if key.bytes().any(|b| matches!(b, b'*' | b'?')) {
            c.starred += 1;
        }
        if KNOWN_PREFIXES.iter().any(|p| key.starts_with(p)) {
            c.known += 1;
        }
        found = true;
    }
    found.then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"! .Xresources\n#define DPI 96\nXft.dpi: DPI\nXft.antialias: true\n*background: #1d1f21\n*foreground: #c5c8c6\n*color0: #282a2e\n*color7: #a3685a\nxterm*faceName: DejaVu Sans Mono\nxterm*faceSize: 11\nurxvt*scrollBar: false\n";

    #[test]
    fn detects_xresources() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 9);
        assert_eq!(c.starred, 7);
        assert_eq!(c.comments, 1);
        assert_eq!(c.cpp, 1);
        assert_eq!(c.known, 9);
    }

    #[test]
    fn rejects_yaml() {
        assert!(!detect(b"name: app\nversion: 1\nlist: item\n"));
        assert!(!detect(b"#!/bin/sh\nexec openbox\n"));
    }
}

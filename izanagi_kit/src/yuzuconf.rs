//! yuzu / sudachi `qt-config.ini` の認識と計数。
//!
//! Qt 設定形式の INI:`[General]`/`[UI]`/`[UIGameList]`/`[UILayout]`/`[Paths]`/
//! `[Shortcuts]`/`[Core]`/`[CPU]`/`[Graphics]`/`[Renderer]`/`[Audio]`/`[Controls]`/
//! `[Data%20Storage]`/`[Miscellaneous]`/`[Debugging]`/`[WebService]`/`[Network]`/
//! `[LibraryApplet]`/`[Multiplayer]` セクション、`key=value`、および
//! Qt `beginGroup` 由来の `Parent\Child=value` バックスラッシュキー、
//! `%20` 等のパーセントエスケープを扱う。
//!
//! ```
//! let b = b"[General]\nconfirmClose=true\n[UI]\nfullscreen=false\nuse_docked_mode=true\n[Shortcuts]\nMain%20Window/ToggleFullscreen\\\\KeySeq=F4\nPaths\\\\gamedirs\\\\1\\\\path=/games\n[Graphics]\nuse_vulkan=true\n";
//! assert!(izanagi_kit::yuzuconf::detect(b));
//! let c = izanagi_kit::yuzuconf::parse(b).unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.assigns, 6);
//! assert_eq!(c.backslash_keys, 2);
//! assert_eq!(c.escaped_keys, 1);
//! assert_eq!(c.bool_assigns, 4);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[Section]` ヘッダ数。
    pub sections: usize,
    /// 既知 yuzu セクション名の数。
    pub known_sections: usize,
    /// `key=value` 代入数。
    pub assigns: usize,
    /// `A\B=value` のバックスラッシュ入りキー数。
    pub backslash_keys: usize,
    /// `%20`/`%5B` 等エスケープを含むキー数。
    pub escaped_keys: usize,
    /// 値が `true`/`false` の代入数。
    pub bool_assigns: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "General",
    "UI",
    "UIGameList",
    "UILayout",
    "UISettings",
    "Paths",
    "Shortcuts",
    "Core",
    "CPU",
    "Graphics",
    "Renderer",
    "Audio",
    "Controls",
    "Miscellaneous",
    "Debugging",
    "WebService",
    "Network",
    "LibraryApplet",
    "Multiplayer",
    "Data%20Storage",
    "Games",
    "Player",
    "Services",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `qt-config.ini` らしさを返す。既知セクション + 小文字 bool 代入 ≥2。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut secs = 0usize;
    let mut bools = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.starts_with('[') && s.ends_with(']') {
            if SECTIONS.contains(&s[1..s.len() - 1].trim()) {
                secs += 1;
            }
            continue;
        }
        if s.ends_with("=true") || s.ends_with("=false") {
            bools += 1;
        }
    }
    secs >= 2 && bools >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let Ok(t) = std::str::from_utf8(b) else {
        return None;
    };
    let t = strip_bom(t);
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        assigns: 0,
        backslash_keys: 0,
        escaped_keys: 0,
        bool_assigns: 0,
        comments: 0,
    };
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') && s.ends_with(']') {
            c.sections += 1;
            if SECTIONS.contains(&s[1..s.len() - 1].trim()) {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(i) = s.find('=') else {
            continue;
        };
        let k = s[..i].trim();
        let v = s[i + 1..].trim();
        if k.is_empty()
            || !k.bytes().all(|ch| {
                ch.is_ascii_alphanumeric() || matches!(ch, b'_' | b'-' | b'.' | b'\\' | b'%' | b'/')
            })
        {
            continue;
        }
        c.assigns += 1;
        if k.contains('\\') {
            c.backslash_keys += 1;
        }
        if k.contains('%') {
            c.escaped_keys += 1;
        }
        if v == "true" || v == "false" {
            c.bool_assigns += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[General]\nconfirmClose=true\n[UI]\nfullscreen=false\n[Controls]\nplayer_0_button_a=0\nPaths\\gamedirs\\1\\path=/x\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.assigns, 4);
        assert_eq!(c.backslash_keys, 1);
        assert_eq!(c.bool_assigns, 2);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(parse(b"[server]\nhost=x\nport=1\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

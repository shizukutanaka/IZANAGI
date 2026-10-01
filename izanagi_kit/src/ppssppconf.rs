//! PPSSPP `ppsspp.ini` / `controls.ini` の認識と計数。
//!
//! `[General]`/`[CPU]`/`[Graphics]`/`[Sound]`/`[Control]`/`[SystemParam]`/
//! `[Network]`/`[Log]`/`[Debug]`/`[SpeedHacks]`/`[Upgrade]`/`[Recent]`/
//! `[WindowPosition]`/`[SystemInfo]`/`[Achievements]`/`[Chat]`/`[Dumps]`/
//! `[VR]`/`[RemoteISOPort]`/`[Search]`/`[Plugins]` 等のセクションと
//! `Key = Value`(bool は `True`/`False`)で構成される。
//!
//! ```
//! let b = b"[General]\nFirstRun = False\nAutoRun = True\nBrowse = True\nCheckForNewVersion = True\n[Graphics]\nBackend = 0\nFullScreen = False\nInternalResolution = 2\nRenderDuplicateFrames = False\n[SystemParam]\nPSPModel = 1\nNickName = PPSSPP\n";
//! assert!(izanagi_kit::ppssppconf::detect(b));
//! let c = izanagi_kit::ppssppconf::parse(b).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.known_sections, 3);
//! assert_eq!(c.assigns, 10);
//! assert_eq!(c.bool_assigns, 6);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[Section]` ヘッダ数。
    pub sections: usize,
    /// 既知 PPSSPP セクション名の数。
    pub known_sections: usize,
    /// `Key = Value` 代入数。
    pub assigns: usize,
    /// 値が `True`/`False` の代入数。
    pub bool_assigns: usize,
    /// 値が数値の代入数。
    pub number_assigns: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "General",
    "CPU",
    "Graphics",
    "Sound",
    "Control",
    "SystemParam",
    "Network",
    "Log",
    "Debug",
    "SpeedHacks",
    "Upgrade",
    "Recent",
    "WindowPosition",
    "SystemInfo",
    "Achievements",
    "Chat",
    "Dumps",
    "VR",
    "RemoteISOPort",
    "Search",
    "Plugins",
    "Reporting",
    "JitProfiling",
    "LogManager",
];

fn table_name(s: &str) -> Option<&str> {
    if !(s.starts_with('[') && s.ends_with(']')) {
        return None;
    }
    let n = s[1..s.len() - 1].trim();
    if n.is_empty() {
        return None;
    }
    Some(n)
}

/// `ppsspp.ini` らしさを返す。既知セクション + 大文字 bool 値。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut secs = 0usize;
    let mut caps = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if let Some(n) = table_name(s) {
            if SECTIONS.contains(&n) {
                secs += 1;
            }
            continue;
        }
        if s.ends_with("= True") || s.ends_with("= False") {
            caps += 1;
        }
    }
    secs >= 1 && caps >= 1
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
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        assigns: 0,
        bool_assigns: 0,
        number_assigns: 0,
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
        if let Some(n) = table_name(s) {
            c.sections += 1;
            if SECTIONS.contains(&n) {
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
            || !k
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'.' || ch == b'_')
        {
            continue;
        }
        c.assigns += 1;
        if v == "True" || v == "False" {
            c.bool_assigns += 1;
        } else if !v.is_empty()
            && v.bytes()
                .all(|ch| ch.is_ascii_digit() || ch == b'-' || ch == b'.')
        {
            c.number_assigns += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[Sound]\nEnable = True\nVolume = 4\n[CPU]\nJit = True\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.assigns, 3);
        assert_eq!(c.bool_assigns, 2);
    }

    #[test]
    fn rejects_lower_bool() {
        assert!(parse(b"[General]\nflag = true\n").is_none());
    }
}

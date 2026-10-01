//! X.Org / Xorg 設定ファイル (`xorg.conf`, `xorg.conf.d/*.conf`) パーサ。
//!
//! `Section "name"` / `SubSection` / `EndSection` ブロックと
//! `Identifier`/`Option`/`Driver`/`Screen` 等のステートメントを計数する。
//!
//! ```
//! use izanagi_kit::xorgconf;
//! let conf = b"Section \"InputClass\"\n    Identifier \"kbd\"\n    Driver \"libinput\"\n    Option \"XkbLayout\" \"jp\"\nEndSection\n";
//! assert!(xorgconf::detect(conf));
//! let c = xorgconf::parse(conf).unwrap();
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.known_sections, 1);
//! assert_eq!(c.options, 1);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `Section`/`SubSection` 行数。
    pub sections: usize,
    /// `EndSection`/`EndSubSection` 行数。
    pub end_sections: usize,
    /// 既知セクション名数。
    pub known_sections: usize,
    /// `Identifier` ステートメント数。
    pub identifiers: usize,
    /// `Option` ステートメント数。
    pub options: usize,
    /// その他のステートメント (`Driver`/`Screen`/`MatchIs*` 等、`Identifier` 含む全行)。
    pub entries: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "ServerLayout",
    "ServerFlags",
    "Files",
    "Module",
    "Extensions",
    "InputDevice",
    "InputClass",
    "Device",
    "Monitor",
    "Modes",
    "Display",
    "Screen",
    "VideoAdaptor",
    "Vendor",
    "DRI",
    "XrdpXorg",
    "XrdpServer",
];

/// 簡易判定 (既知セクション名 + `EndSection` 有無)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_sections >= 1 && c.end_sections >= 1
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        end_sections: 0,
        known_sections: 0,
        identifiers: 0,
        options: 0,
        entries: 0,
    };
    let mut found = false;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let Some(first) = t.split_whitespace().next() else {
            continue;
        };
        match first {
            "Section" | "SubSection" => {
                c.sections += 1;
                if let Some(name) = t.split('"').nth(1) {
                    if KNOWN_SECTIONS.contains(&name) {
                        c.known_sections += 1;
                    }
                }
            }
            "EndSection" | "EndSubSection" => c.end_sections += 1,
            "Identifier" => {
                c.identifiers += 1;
                c.entries += 1;
            }
            "Option" => c.options += 1,
            _ => c.entries += 1,
        }
        found = true;
    }
    found.then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"Section \"ServerLayout\"\n    Identifier \"layout\"\n    Screen 0 \"screen0\"\n    InputDevice \"kbd\" \"CoreKeyboard\"\nEndSection\n\nSection \"InputClass\"\n    Identifier \"touchpad\"\n    Driver \"libinput\"\n    MatchIsTouchpad \"on\"\n    Option \"Tapping\" \"on\"\nEndSection\n\nSection \"Device\"\n    Identifier \"gpu\"\n    Driver \"modesetting\"\n    Option \"AccelMethod\" \"glamor\"\nEndSection\n";

    #[test]
    fn detects_xorgconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.end_sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.identifiers, 3);
        assert_eq!(c.options, 2);
        assert_eq!(c.entries, 8);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"#!/bin/sh\nexec i3\n"));
        assert!(!detect(
            b"Section \"Foo\"\n    Identifier \"x\"\nEndSection\n"
        ));
        assert!(!detect(b"Section \"Device\"\n    Driver \"modesetting\"\n"));
    }
}

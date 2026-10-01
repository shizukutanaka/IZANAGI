//! PCSX2 `PCSX2_ui.ini` / `PCSX2_vm.ini` の認識と計数。
//!
//! `[Filenames]`/`[SysSettings]`/`[UI]`/`[EmuCore]`/`[EmuCore/Speedhacks]`/
//! `[EmuCore/Gamefixes]`/`[EmuCore/CPU]`/`[EmuCore/CPU/Recompiler]`/
//! `[EmuCore/CPU/FPU]`/`[EmuCore/GS]`/`[EmuCore/GS/FrameSkip]`/`[GSWindow]`/
//! `[PluginDirectories]`/`[ProgramLog]`/`[Hotkeys]`/`[GameList]`/`[Folders]`/
//! `[EmuCore/Mcd]`/`[EmuCore/DEV9]`/`[EmuCore/USB]`/`[EmuCore/PAD]`/
//! `[EmuCore/TraceLog]` 等、パス風の `/` 入りセクションと `Key = Value`
//! (値 `enabled`/`disabled`/`true`/`false`)で構成される。
//!
//! ```
//! let b = b"[Filenames]\nBIOS = /bios/scph39001.bin\n[EmuCore]\nEnableSpeedHacks = enabled\nEnableGameFixes = disabled\nEnablePatches = enabled\n[EmuCore/CPU]\nEnableEE = enabled\nEnableIOP = enabled\nEnableVU0 = enabled\nEnableVU1 = enabled\n[EmuCore/GS]\nSynchronousMTGS = disabled\nDisableGSOutput = false\n";
//! assert!(izanagi_kit::pcsx2conf::detect(b));
//! let c = izanagi_kit::pcsx2conf::parse(b).unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.slash_sections, 2);
//! assert_eq!(c.assigns, 10);
//! assert_eq!(c.bool_assigns, 9);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[Section]` ヘッダ数。
    pub sections: usize,
    /// `A/B` 形式のパス風セクション数。
    pub slash_sections: usize,
    /// `[EmuCore*]` 配下のセクション数。
    pub emucore_sections: usize,
    /// `Key = Value` 代入数。
    pub assigns: usize,
    /// 値が `enabled`/`disabled`/`true`/`false` の代入数。
    pub bool_assigns: usize,
    /// `Enable*`/`Disable*` 系の機能トグルキー数。
    pub toggle_keys: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

fn table_name(s: &str) -> Option<&str> {
    if !(s.starts_with('[') && s.ends_with(']')) {
        return None;
    }
    let n = s[1..s.len() - 1].trim();
    if n.is_empty()
        || !n
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'/' || c == b'_' || c == b'-')
    {
        return None;
    }
    Some(n)
}

/// `PCSX2_*.ini` らしさを返す。`[EmuCore*]` セクション、または
/// `enabled`/`disabled` 値トグル ≥2。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut emu = 0usize;
    let mut tog = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if let Some(n) = table_name(s) {
            if n.starts_with("EmuCore") || n.starts_with("Filenames") || n.starts_with("GS") {
                emu += 1;
            }
            continue;
        }
        if s.ends_with("= enabled") || s.ends_with("= disabled") {
            tog += 1;
        }
    }
    emu >= 1 || tog >= 2
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
        slash_sections: 0,
        emucore_sections: 0,
        assigns: 0,
        bool_assigns: 0,
        toggle_keys: 0,
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
            if n.contains('/') {
                c.slash_sections += 1;
            }
            if n.starts_with("EmuCore") {
                c.emucore_sections += 1;
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
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'.' || ch == b'_' || ch == b'/')
        {
            continue;
        }
        c.assigns += 1;
        if v == "enabled" || v == "disabled" || v == "true" || v == "false" {
            c.bool_assigns += 1;
        }
        if k.starts_with("Enable") || k.starts_with("Disable") {
            c.toggle_keys += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[EmuCore/Speedhacks]\nEnableWaitLoop = enabled\nvuFlagHack = true\nintcSTATSlow = true\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.slash_sections, 1);
        assert_eq!(c.assigns, 3);
        assert_eq!(c.bool_assigns, 3);
        assert_eq!(c.toggle_keys, 1);
    }

    #[test]
    fn rejects_plain_ini() {
        assert!(parse(b"[server]\nhost = x\nport = 1\n").is_none());
    }
}

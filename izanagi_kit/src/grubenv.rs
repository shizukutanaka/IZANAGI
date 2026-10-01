//! GRUB 環境ブロック `grubenv` の検出・カウント。
//!
//! `# GRUB Environment Block` ヘッダ + `key=value` フラット。
//! `saved_entry`/`default`/`timeout`/`menu_auto_hide`/`kernelopts`/`boot_success`/
//! `boot_indeterminate`/`next_entry`/`feature_*`/`superusers`/`recordfail` 等。
//!
//! ```
//! let cfg = b"# GRUB Environment Block\n\
//!             saved_entry=0\n\
//!             boot_success=1\n\
//!             kernelopts=root=/dev/sda1 ro quiet\n";
//! assert!(izanagi_kit::grubenv::detect(cfg));
//! let c = izanagi_kit::grubenv::parse(cfg).unwrap();
//! assert_eq!(c.entries, 3);
//! ```

/// grubenv 既知キー。
const KNOWN_KEYS: &[&str] = &[
    "saved_entry",
    "default",
    "timeout",
    "timeout_style",
    "menu_auto_hide",
    "menu_hide_ok",
    "kernelopts",
    "blsfg",
    "boot_success",
    "boot_indeterminate",
    "boot_limit",
    "next_entry",
    "last_entry",
    "chosen",
    "feature_menu_show_all",
    "feature_timeout_style",
    "feature_timeout",
    "feature_all_video_module",
    "feature_chainloader_bpb",
    "feature_ntldr",
    "feature_platform_search_hint",
    "feature_default_font_path",
    "feature_nativedisk_cmd",
    "feature_200_final",
    "superusers",
    "recordfail",
    "recordfail_late",
    "extra_initrd",
    "fwsetup",
    "memtest_on",
    "serial",
    "gfxmode",
    "gfxpayload",
    "gfxterm_font",
    "lang",
    "locale_dir",
    "load_video",
    "have_grubenv",
    "boot_menu",
    "once",
    "prev_saved_entry",
    "savedefault",
    "mainmenu_saved_entry",
    "config_file",
    "prefix",
    "root",
    "device",
    "drivers",
    "netbootpath",
    "entry",
    "indeterminate",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key=value` エントリ総数。
    pub entries: usize,
    /// `saved_entry`/`next_entry`/`last_entry`/`chosen`/`prev_saved_entry`/`mainmenu_saved_entry`/`once`/`entry` エントリ選択数。
    pub selection: usize,
    /// `boot_success`/`boot_indeterminate`/`boot_limit`/`indeterminate`/`recordfail*` 起動状態数。
    pub bootstate: usize,
    /// `kernelopts`/`extra_initrd`/`config_file`/`root`/`device`/`prefix`/`drivers`/`netbootpath`/`blsfg`/`have_grubenv`/`superusers`/`fwsetup`/`lang`/`locale_dir`/`load_video`/`memtest_on` 等その他既知数。
    pub known: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が grubenv 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を grubenv として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        selection: 0,
        bootstate: 0,
        known: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            known += 1;
        } else {
            c.misc += 1;
            continue;
        }
        if matches!(
            key,
            "saved_entry"
                | "next_entry"
                | "last_entry"
                | "chosen"
                | "prev_saved_entry"
                | "mainmenu_saved_entry"
                | "once"
                | "entry"
        ) {
            c.selection += 1;
        } else if matches!(
            key,
            "boot_success"
                | "boot_indeterminate"
                | "boot_limit"
                | "indeterminate"
                | "recordfail"
                | "recordfail_late"
        ) {
            c.bootstate += 1;
        } else {
            c.known += 1;
        }
    }
    if known >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# GRUB Environment Block\n\
        saved_entry=4a4f8a36dd7a4bd9b0d3f5a6e0c9f5a1-6.1.0-18-amd64\n\
        kernelopts=root=/dev/mapper/debian-root ro quiet\n\
        boot_success=1\n\
        boot_indeterminate=0\n\
        menu_auto_hide=1\n\
        feature_menu_show_all=1\n";

    #[test]
    fn detects_grubenv() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 6);
        assert_eq!(c.selection, 1);
        assert_eq!(c.bootstate, 2);
        assert_eq!(c.known, 3);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_env() {
        assert!(!detect(b"FOO=1\nBAR=2\n"));
        assert!(!detect(b"saved_entry=0\n"));
    }
}

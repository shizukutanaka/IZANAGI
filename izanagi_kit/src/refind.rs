//! rEFInd `refind.conf` の検出・カウント。
//!
//! 空白区切り `keyword values…` (値は `+`/`-`/`,` 連結) + `menuentry "name" { … }`/
//! `submenuentry`/`disabled`/`dont_scan_*`/`scanfor`/`also_scan_dirs` 等。
//!
//! ```
//! let cfg = b"timeout 20\n\
//!             scanfor manual,external,optical\n\
//!             dont_scan_dirs EFI/boot\n\
//!             menuentry \"Linux\" {\n\
//!                 volume ESP\n\
//!                 loader /vmlinuz\n\
//!                 initrd /initrd.img\n\
//!                 options \"root=/dev/sda1 ro\"\n\
//!             }\n";
//! assert!(izanagi_kit::refind::detect(cfg));
//! let c = izanagi_kit::refind::parse(cfg).unwrap();
//! assert_eq!(c.menuentries, 1);
//! ```

/// グローバルオプション系キーワード。
const GLOBAL_KEYS: &[&str] = &[
    "timeout",
    "screensaver",
    "use_nvram",
    "use_graphics_for",
    "use_bgrt",
    "shutdown_after_timeout",
    "hideui",
    "icons_dir",
    "set_default_boot_item",
    "enable_and_lock_vm",
    "enable_touch",
    "enable_mouse",
    "write_systemd_vars",
    "scan_driver_dirs",
    "scan_code_page",
    "scan_delay",
    "also_scan_dirs",
    "default_selection",
    "menuentry_disabled",
    "textmode",
    "textonly",
    "quiet",
    "log_level",
    "resolution",
    "showtools",
    "csr_values",
    "install",
    "recovery_icons",
    "sync_codes",
    "big_icon_size",
    "small_icon_size",
    "selection_dir",
    "banner",
    "banner_scale",
    "font",
    "loadfonts",
    "icons",
    "windows_recovery_files",
    "kbd",
    "hint",
    "version",
    "os_icon",
    "include",
    "fold_linux_kernels",
    "scan_all_linux_kernels",
    "max_tags",
    "extra_kernel_version_strings",
    "spinner",
    "confirm_esp_exit",
    "token",
    "allow_tools_from_boot",
    "config_file_name",
];

/// スキャン・検出系キーワード。
const SCAN_KEYS: &[&str] = &[
    "scanfor",
    "dont_scan_files",
    "dont_scan_dirs",
    "dont_scan_tools",
    "dont_scan_firmware",
    "dont_scan_volumes",
    "also_scan_files",
    "also_scan_dirs",
    "dont_scan_last_change",
    "hidden_tags",
    "hidden_tags_always_show",
    "hidden_tags_external",
    "hide_ui_tag",
    "all_tags",
    "hidden_tags_show",
];

/// `menuentry`/`submenuentry`/`loader`/`initrd`/`options`/`volume`/`ostype`/`disabled`/`icon`/`graphics` ブロック内キーワード。
const MENU_KEYS: &[&str] = &[
    "menuentry",
    "submenuentry",
    "volume",
    "loader",
    "initrd",
    "options",
    "ostype",
    "icon",
    "graphics",
    "disabled",
    "external",
    "firmware_bootnum",
    "boot_code",
    "amd",
    "intel",
    "tok",
    "load_options",
    "mainoptions",
    "kernel",
    "initrds",
    "splash",
    "add_options",
    "substitution",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ステートメント行総数 (ブレース行含む)。
    pub entries: usize,
    /// `menuentry`/`submenuentry` 行数。
    pub menuentries: usize,
    /// グローバルオプション行数。
    pub globals: usize,
    /// `scanfor`/`dont_scan_*`/`also_scan_*`/`hidden_tags*` スキャン制御数。
    pub scans: usize,
    /// `volume`/`loader`/`initrd`/`options`/`ostype`/`icon`/`graphics`/`disabled` ブロック内行数。
    pub inner: usize,
    /// `{`/`}` 行数。
    pub braces: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が `refind.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2 && c.menuentries + c.scans + c.globals >= 2)
}

/// `b` を `refind.conf` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        menuentries: 0,
        globals: 0,
        scans: 0,
        inner: 0,
        braces: 0,
        misc: 0,
    };
    let mut known = 0usize;
    let mut in_menu = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if line == "{" {
            c.braces += 1;
            continue;
        }
        if line == "}" {
            c.braces += 1;
            in_menu = false;
            continue;
        }
        let head = line.split([' ', '\t']).next().unwrap_or("");
        if matches!(head, "menuentry" | "submenuentry") {
            c.menuentries += 1;
            known += 1;
            if line.contains('{') || !line.ends_with('{') {
                in_menu = true;
            }
        } else if in_menu || line.ends_with('{') && MENU_KEYS.contains(&head) {
            if MENU_KEYS.contains(&head) {
                c.inner += 1;
                known += 1;
            } else {
                c.misc += 1;
            }
        } else if SCAN_KEYS.contains(&head) {
            c.scans += 1;
            known += 1;
        } else if GLOBAL_KEYS.contains(&head) {
            c.globals += 1;
            known += 1;
        } else if MENU_KEYS.contains(&head) {
            c.inner += 1;
            known += 1;
        } else {
            c.misc += 1;
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

    const SAMPLE: &[u8] = b"# refind.conf\n\
        timeout 20\n\
        use_nvram false\n\
        hideui singleuser,hints\n\
        scanfor manual,external,optical\n\
        dont_scan_dirs EFI/boot,EFI/tools\n\
        dont_scan_files shim.efi,bootmgfw.efi\n\
        also_scan_dirs ESP2:EFI/linux\n\
        default_selection \"+,vmlinuz,Linux\"\n\
        showtools shell,memtest,gdisk,exit,firmware\n\
        resolution 1024 768\n\
        \n\
        menuentry \"Debian Linux\" {\n\
            icon EFI/refind/icons/os_debian.png\n\
            volume ESP\n\
            loader /vmlinuz\n\
            initrd /initrd.img\n\
            options \"root=/dev/sda1 ro quiet\"\n\
            submenuentry \"recovery mode\" {\n\
                options \"root=/dev/sda1 ro single\"\n\
            }\n\
        }\n\
        menuentry \"Windows\" {\n\
            icon EFI/refind/icons/os_win.png\n\
            loader EFI/Microsoft/Boot/bootmgfw.efi\n\
            disabled\n\
        }\n";

    #[test]
    fn detects_refind() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.menuentries, 3);
        assert_eq!(c.globals, 6);
        assert_eq!(c.scans, 4);
        assert_eq!(c.inner, 9);
        assert_eq!(c.braces, 3);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo bar\nbaz qux\n"));
        assert!(!detect(b"timeout 20\n"));
    }
}

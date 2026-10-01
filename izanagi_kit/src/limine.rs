//! Limine ブートローダ `limine.cfg` の検出・カウント。
//!
//! `KEY=value` 形式 + `:EntryName`/`::SubEntry` エントリ名行。
//! グローバル (`TIMEOUT`/`SERIAL`/`VERBOSE`/`KASLR`/`GRAPHICS`/`RESOLUTION`/`THEME_*`/
//! `INTERFACE_*`/`RANDOMISE_MEMORY`)、エントリ内ブートキー (`PROTOCOL`/`KERNEL_PATH`/
//! `CMDLINE`/`MODULE_PATH`/`IMAGE_PATH`/`INITRAMFS_PATH`/`ENTRY_PATH`/`KERNEL_CMDLINE`/
//! `ROOT_PARTITION`/`RESOLUTION`/`KASLR`/`MAKE_QLACS`) を分類。
//!
//! ```
//! let cfg = b"TIMEOUT=5\n\
//!             VERBOSE=yes\n\
//!             :Linux\n\
//!             PROTOCOL=limine\n\
//!             KERNEL_PATH=boot:///vmlinuz\n\
//!             CMDLINE=root=/dev/sda1\n";
//! assert!(izanagi_kit::limine::detect(cfg));
//! let c = izanagi_kit::limine::parse(cfg).unwrap();
//! assert_eq!(c.boot_entries, 1);
//! ```

/// グローバルオプションキー。
const GLOBAL_KEYS: &[&str] = &[
    "TIMEOUT",
    "SERIAL",
    "VERBOSE",
    "KASLR",
    "GRAPHICS",
    "RESOLUTION",
    "RANDOMISE_MEMORY",
    "RANDOMIZE_MEMORY",
    "TERM_WALLPAPER",
    "TERM_BACKDROP",
    "TERM_BACKGROUND",
    "TERM_FONT",
    "TERM_FONT_SCALE",
    "TERM_MARGIN",
    "TERM_MARGIN_GRADIENT",
    "TERM_PALETTE",
    "TERM_PALETTE_BRIGHT",
    "TERM_BACKGROUND_BRIGHTNESS",
    "TERM_FOREGROUND",
    "TERM_FOREGROUND_BRIGHT",
    "TERM_BACKGROUND_BRIGHT",
    "TERM_COLS",
    "TERM_ROWS",
    "THEME_MARGIN",
    "THEME_MARGIN_GRADIENT",
    "THEME_BACKDROP",
    "THEME_BACKGROUND",
    "THEME_FOREGROUND",
    "THEME_FOREGROUND_BRIGHT",
    "THEME_BACKGROUND_BRIGHT",
    "THEME_PALETTE",
    "THEME_PALETTE_BRIGHT",
    "INTERFACE_RESOLUTION",
    "INTERFACE_BRANDING",
    "INTERFACE_BRANDING_COLOR",
    "INTERFACE_BRANDING_COLOUR",
    "INTERFACE_HELP_HIDDEN",
    "WALLPAPER_STYLE",
    "CONFIG_LIMINE_DIR",
    "CONFIG_PATH",
    "CONFIG_LAST_EDITED",
    "CONFIG_LIMINE_VERSION",
    "CONFIG_HASH",
    "EDITOR_ENABLED",
    "EDITOR_NO_BUFFER",
    "EDITING",
    "DEFAULT_ENTRY",
    "DEFAULT_ENTRY_2",
    "COMMENTLINE",
    "QUIET",
];

/// エントリ内ブートキー。
const BOOT_KEYS: &[&str] = &[
    "PROTOCOL",
    "KERNEL_PATH",
    "KERNEL_CMDLINE",
    "CMDLINE",
    "MODULE_PATH",
    "MODULE_STRING",
    "IMAGE_PATH",
    "INITRAMFS_PATH",
    "ENTRY_PATH",
    "RSP_URL",
    "ROOT_PARTITION",
    "DRIVE",
    "KERNEL",
    "KERNEL_NAME",
    "DTB_PATH",
    "HARTID",
    "MAKE_QLACS",
    "KASLR2",
    "MODULES",
    "MODULE_ALIAS",
    "MODULE_CHAINED",
    "MODULE_GROUP",
    "CHAINS",
    "BIOS",
    "UEFI",
    "LIAND",
    "FIRMWARE",
    "MULTIBOOT",
    "VMLINUX",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key=value` エントリ総数。
    pub entries: usize,
    /// `:`/`::`/`/`/`//` 始まりのエントリ名行数。
    pub boot_entries: usize,
    /// グローバルオプション (`TIMEOUT`/`SERIAL`/`VERBOSE`/`GRAPHICS`/`TERM_*`/`THEME_*`/`INTERFACE_*`/`WALLPAPER_*`/`RESOLUTION`/`KASLR` ルート出現) 数。
    pub globals: usize,
    /// `PROTOCOL`/`KERNEL_PATH`/`CMDLINE`/`MODULE_PATH`/`IMAGE_PATH`/`INITRAMFS_PATH`/`ENTRY_PATH`/`DTB_PATH` ブートキー数。
    pub boot: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が `limine.cfg` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.boot_entries >= 1 || c.boot + c.globals >= 2)
}

/// `b` を `limine.cfg` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        boot_entries: 0,
        globals: 0,
        boot: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with(':') || line.starts_with('/') {
            c.boot_entries += 1;
            known += 1;
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
        if BOOT_KEYS.contains(&key) {
            c.boot += 1;
            known += 1;
        } else if GLOBAL_KEYS.contains(&key) {
            c.globals += 1;
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

    const SAMPLE: &[u8] = b"TIMEOUT=5\n\
        SERIAL=yes\n\
        VERBOSE=yes\n\
        RANDOMISE_MEMORY=yes\n\
        THEME_MARGIN=64\n\
        INTERFACE_RESOLUTION=1920x1080\n\
        \n\
        :Debian GNU/Linux\n\
        PROTOCOL=limine\n\
        KERNEL_PATH=boot:///vmlinuz-6.1\n\
        KERNEL_CMDLINE=root=UUID=abc ro quiet\n\
        \n\
        ::with initramfs\n\
        PROTOCOL=limine\n\
        KERNEL_PATH=boot:///vmlinuz-6.1\n\
        MODULE_PATH=boot:///initrd.img-6.1\n\
        CMDLINE=root=UUID=abc ro\n\
        \n\
        :Windows\n\
        PROTOCOL=chainload\n\
        IMAGE_PATH=hdd:///EFI/Microsoft/Boot/bootmgfw.efi\n";

    #[test]
    fn detects_limine() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.boot_entries, 3);
        assert_eq!(c.globals, 6);
        assert_eq!(c.boot, 9);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"FOO=1\nBAR=2\n"));
        assert!(!detect(b"TIMEOUT=5\n"));
    }
}

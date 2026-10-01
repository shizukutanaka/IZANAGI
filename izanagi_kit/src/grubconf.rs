//! GRUB 2 `grub.cfg` の検出・カウント。
//!
//! `menuentry '…' { … }`/`submenu`、`set key=value`、`linux`/`initrd`/`chainloader`/
//! `multiboot*`/`module*`/`boot` ブート行、`insmod`/`search`/`terminal*`/`load_env`/
//! `save_env`/`configfile`、シェル構文 `if`/`elif`/`else`/`fi`/`for`/`while`/`do`/`done`/
//! `function`/`export` を分類。
//!
//! ```
//! let cfg = b"set default=0\n\
//!             set timeout=5\n\
//!             menuentry 'Linux' {\n\
//!                 linux /vmlinuz root=/dev/sda1\n\
//!                 initrd /initrd.img\n\
//!             }\n";
//! assert!(izanagi_kit::grubconf::detect(cfg));
//! let c = izanagi_kit::grubconf::parse(cfg).unwrap();
//! assert_eq!(c.menuentries, 1);
//! ```

/// `set` 以外の既知トップレベル命令。
const KNOWN_KEYS: &[&str] = &[
    "insmod",
    "search",
    "terminal_input",
    "terminal_output",
    "terminal",
    "term",
    "load_env",
    "save_env",
    "configfile",
    "play",
    "background_image",
    "background_color",
    "normal",
    "normal_exit",
    "recordfail",
    "recordfail_late",
    "export",
    "source",
    "ls",
    "sleep",
    "echo",
    "halt",
    "reboot",
    "hiddenmenu",
    "fallback",
    "title",
    "root",
    "rootnoverify",
    "savedefault",
    "password",
    "password_pbkdf2",
    "drivemap",
    "parttool",
    "nativedisk",
    "true",
    "false",
    "unset",
    "serial",
    "efi",
    "load_video",
    "font",
    "gettext",
    "keymap",
    "gfxmode",
    "gfxpayload",
    "gfxterm",
    "theme",
];

/// ブート系命令。
const BOOT_KEYS: &[&str] = &[
    "linux",
    "linux16",
    "initrd",
    "initrd16",
    "chainloader",
    "boot",
    "kfreebsd",
    "kfreebsd_loadenv",
    "kfreebsd_module",
    "kfreebsd_module_elf",
    "knetbsd",
    "knetbsd_module",
    "knetbsd_module_elf",
    "kopenbsd",
    "kopenbsd_ramdisk",
    "multiboot",
    "multiboot2",
    "module",
    "module2",
    "xnu_kernel",
    "xnu_kernel64",
    "xnu_kext",
    "xnu_ramdisk",
    "xnu_resume",
    "linux_xen",
    "xen_hypervisor",
    "xen_module",
    "net_boot",
    "pxe_unload",
    "zfs_kernel",
    "zfsinfo",
    "zfskey",
];

/// シェル構文キーワード。
const SHELL_KEYS: &[&str] = &[
    "if",
    "elif",
    "else",
    "fi",
    "for",
    "while",
    "until",
    "do",
    "done",
    "function",
    "continue",
    "break",
    "return",
    "shift",
    "setparams",
    "eval",
    "then",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 命令・ステートメント行総数 (ブレース行含む)。
    pub entries: usize,
    /// `menuentry`/`submenu` 行数。
    pub menuentries: usize,
    /// `set key=value`/`unset` 行数。
    pub sets: usize,
    /// `linux`/`initrd`/`chainloader`/`boot`/`multiboot*`/`module*`/`xnu_*` ブート行数。
    pub boot: usize,
    /// `insmod`/`search`/`terminal*`/`load_env`/`save_env`/`configfile`/`export`/`source`/`echo` 等その他既知数。
    pub known: usize,
    /// `if`/`elif`/`else`/`fi`/`for`/`while`/`do`/`done`/`function`/`setparams` 構文行数。
    pub shell: usize,
    /// `{`/`}`/`} else {` 等ブレースのみ行数。
    pub braces: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が `grub.cfg` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| {
        c.menuentries >= 1 || (c.boot >= 1 && c.sets >= 1) || c.entries - c.misc >= 4
    })
}

/// `b` を `grub.cfg` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        menuentries: 0,
        sets: 0,
        boot: 0,
        known: 0,
        shell: 0,
        braces: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if line == "{"
            || line == "}"
            || line == "} else {"
            || line == "} else"
            || line.starts_with('}') && line.ends_with('{')
        {
            c.braces += 1;
            continue;
        }
        let head = line.split([' ', '\t']).next().unwrap_or("");
        if matches!(head, "menuentry" | "submenu" | "menuentry_id_option") {
            c.menuentries += 1;
            known += 1;
        } else if head.starts_with("set ") || head == "set" || head == "unset" {
            c.sets += 1;
            known += 1;
        } else if BOOT_KEYS.contains(&head) {
            c.boot += 1;
            known += 1;
        } else if SHELL_KEYS.contains(&head) {
            c.shell += 1;
            known += 1;
        } else if KNOWN_KEYS.contains(&head) || line.starts_with("insmod ") {
            c.known += 1;
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

    const SAMPLE: &[u8] = b"# grub.cfg\n\
        set default=0\n\
        set timeout=5\n\
        set timeout_style=menu\n\
        insmod gzio\n\
        insmod part_msdos\n\
        insmod ext2\n\
        search --no-floppy --fs-uuid --set=root abc-123\n\
        terminal_input console\n\
        terminal_output console\n\
        \n\
        menuentry 'Debian GNU/Linux' --class debian {\n\
            load_video\n\
            insmod gzio\n\
            linux /boot/vmlinuz-6.1 root=UUID=abc ro quiet\n\
            initrd /boot/initrd.img-6.1\n\
        }\n\
        submenu 'Advanced options' {\n\
            menuentry 'recovery' {\n\
                linux /boot/vmlinuz-6.1 root=UUID=abc ro single\n\
                initrd /boot/initrd.img-6.1\n\
            }\n\
        }\n\
        if [ \"$recordfail\" = 1 ]; then\n\
            set timeout=-1\n\
        else\n\
            set timeout=5\n\
        fi\n\
        chainloader /efi/boot/bootx64.efi\n";

    #[test]
    fn detects_grubconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.menuentries, 3);
        assert_eq!(c.boot, 5);
        assert_eq!(c.sets, 5);
        assert_eq!(c.shell, 3);
        assert_eq!(c.braces, 3);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"set timeout=5\n"));
    }
}

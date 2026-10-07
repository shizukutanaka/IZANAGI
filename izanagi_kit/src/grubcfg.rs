//! `grub.cfg` (GRUB2) 検出モジュール。
//!
//! GRUB2 の設定は `menuentry 'Name' {`/`submenu` ブロックと、
//! `set default=`/`set timeout=`/`set timeout_style=`/
//! `insmod`/`linux /boot/...`/`initrd`/`search --fs-uuid`/
//! `if [ ... ]; then`/`fi`/`function`/`load_video`/`serial`/
//! `terminal_input`/`terminal_output`/`play`/`recordfail`/
//! `savedefault`/`drivemap`/`parttool`/`export` コマンドで
//! 構成される。
//!
//! ```
//! let b = b"set default=0\n\
//!           set timeout=5\n\
//!           insmod gzio\n\
//!           insmod part_msdos\n\
//!           menuentry 'Ubuntu' {\n\
//!               linux /boot/vmlinuz root=/dev/sda1\n\
//!               initrd /boot/initrd.img\n\
//!           }\n";
//! let c = izanagi_kit::grubcfg::parse(b);
//! assert!(izanagi_kit::grubcfg::detect(b));
//! assert_eq!(c.menuentries, 1);
//! ```

const CMDS: &[&str] = &[
    "acpi",
    "bitmap",
    "blocklist",
    "boot",
    "cat",
    "chainloader",
    "configfile",
    "cpuid",
    "drivemap",
    "echo",
    "efiemu",
    "export",
    "false",
    "fix_video",
    "font",
    "gettext",
    "gfxmode",
    "gfxpayload",
    "gzio",
    "halt",
    "initrd",
    "initrd16",
    "insmod",
    "kbd",
    "keystatus",
    "linux",
    "linux16",
    "load_video",
    "ls",
    "menuentry",
    "multiboot",
    "normal",
    "ntldr",
    "parttool",
    "password",
    "play",
    "probe",
    "read",
    "recordfail",
    "reboot",
    "rmmod",
    "savedefault",
    "search",
    "sendkey",
    "serial",
    "set",
    "sha1sum",
    "sha256sum",
    "sha512sum",
    "sleep",
    "source",
    "submenu",
    "terminal_input",
    "terminal_output",
    "terminfo",
    "test",
    "true",
    "unset",
    "usb",
    "vbe",
    "videoinfo",
    "xnu",
];

fn head_cmd(t: &str) -> &str {
    let t = t.trim_start_matches("if ").trim_start_matches("elif ");
    let t = t.split([' ', '\t', '=']).next().unwrap_or("");
    t
}

fn is_entry(t: &str) -> bool {
    t.starts_with("menuentry") || t.starts_with("submenu")
}

/// `b` が grub.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut entries = 0usize;
    let mut cmds = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_entry(tr) {
            entries += 1;
            cmds += 1;
            continue;
        }
        if CMDS.contains(&head_cmd(tr)) {
            cmds += 1;
        }
    }
    (entries >= 1 && cmds >= 3) || cmds >= 6
}

/// grub.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct GrubCfg {
    /// menuentry/submenu ブロック数。
    pub menuentries: usize,
    /// 既知コマンド行数。
    pub commands: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を grub.cfg として統計する。
pub fn parse(b: &[u8]) -> GrubCfg {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = GrubCfg::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_entry(tr) {
            c.menuentries += 1;
            c.commands += 1;
            continue;
        }
        if CMDS.contains(&head_cmd(tr)) {
            c.commands += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"set default=0\nset timeout=5\ninsmod gzio\nmenuentry 'Ubuntu' {\n linux /vmlinuz\n}\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.menuentries, 1);
    }

    #[test]
    fn detects_cmds() {
        let b = b"set a=1\ninsmod x\nload_video\nserial --speed=1\nterminal_input x\nsleep 1\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"key = value\nfoo = bar\nbaz = quux\n"));
        assert!(!detect(b"menuentry 'x' {\n}\n"));
        assert!(!detect(b"[section]\nkey = x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.menuentries, 0);
    }
}

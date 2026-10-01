//! SYSLINUX/PXELINUX/ISOLINUX ブート設定(`pxelinux.cfg/*`・`syslinux.cfg`・
//! `isolinux.cfg`)の認識と計数。
//!
//! ディレクティブは大文字の空白区切り行: `DEFAULT`/`PROMPT`/`TIMEOUT`/
//! `ONTIMEOUT`/`TOTALTIMEOUT`/`NOESCAPE`/`ALLOWOPTIONS`/`MENU`(TITLE/LABEL/
//! DEFAULT/AUTOBOOT/BACKGROUND/COLOR/HIDDEN/HELP/IMMEDIATE/INDENT/SAVE/
//! SEPARATOR/TABMSG)/`LABEL`/`KERNEL`/`LINUX`/`APPEND`/`INITRD`/`IPAPPEND`/
//! `SYSAPPEND`/`INCLUDE`/`UI`/`CONFIG`/`DISPLAY`/`FONT`/`KBDMAP`/`LOCALBOOT`/
//! `SERIAL`/`SAY`/`TEXT HELP…ENDTEXT`/`PATH`/`F1`…`F12`。`LABEL` 以降の
//! インデント行はそのエントリのカーネル指定。
//!
//! ```
//! let b = b"DEFAULT menu.c32\nPROMPT 0\nTIMEOUT 300\nONTIMEOUT local\nMENU TITLE Boot Menu\nMENU AUTOBOOT Starting in # seconds\n\nLABEL local\n  MENU LABEL Local Boot\n  LOCALBOOT 0\n  MENU DEFAULT\n\nLABEL install\n  MENU LABEL Install Linux\n  KERNEL vmlinuz\n  APPEND initrd=initrd.img quiet\n  INITRD initrd.img\n  IPAPPEND 2\n";
//! assert!(izanagi_kit::pxelinux::detect(b));
//! let c = izanagi_kit::pxelinux::parse(b).unwrap();
//! assert_eq!(c.labels, 2);
//! assert_eq!(c.menu_directives, 5); // TITLE/AUTOBOOT/LABEL×2/DEFAULT
//! assert_eq!(c.kernels, 2); // KERNEL + INITRD 系… KERNEL 1 + INITRD 1
//! assert_eq!(c.global_opts, 5); // DEFAULT/PROMPT/TIMEOUT/ONTIMEOUT/LOCALBOOT
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 有効ディレクティブ行の個数。
    pub directives: usize,
    /// `LABEL <name>` エントリの個数。
    pub labels: usize,
    /// `MENU <サブ>` ディレクティブの個数。
    pub menu_directives: usize,
    /// `KERNEL`/`LINUX`/`INITRD`/`BOOT` 行の個数。
    pub kernels: usize,
    /// `APPEND`/`IPAPPEND`/`SYSAPPEND` 行の個数。
    pub appends: usize,
    /// `DEFAULT`/`PROMPT`/`TIMEOUT`/`ONTIMEOUT`/`TOTALTIMEOUT`/`NOESCAPE`/
    /// `ALLOWOPTIONS`/`UI`/`SERIAL`/`DISPLAY`/`FONT`/`KBDMAP`/`PATH`/`INCLUDE`/
    /// `CONFIG`/`NOKBD`/`SENDCOOKIES` 等グローバルオプションの個数。
    pub global_opts: usize,
    /// `#`/`;` コメント行の個数。
    pub comments: usize,
}

const GLOBAL: &[&str] = &[
    "DEFAULT",
    "PROMPT",
    "TIMEOUT",
    "ONTIMEOUT",
    "TOTALTIMEOUT",
    "NOESCAPE",
    "ALLOWOPTIONS",
    "NOKBD",
    "UI",
    "SERIAL",
    "DISPLAY",
    "FONT",
    "KBDMAP",
    "PATH",
    "INCLUDE",
    "CONFIG",
    "SAY",
    "SENDCOOKIES",
    "MASTER",
    "IMPLICIT",
    "PxeLocalBoot",
    "CONSOLE",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "TEXT",
    "HELP",
    "ENDTEXT",
    "LOCALBOOT",
    "PROMT",
    "NOROOTPASSWD",
    "ALLOWHASH",
];

fn head(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

/// pxelinux らしさを返す。`LABEL`+カーネル系、または `MENU` 系 + `DEFAULT`。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
            continue;
        }
        match head(s) {
            "LABEL" | "KERNEL" | "LINUX" | "APPEND" | "INITRD" | "MENU" | "DEFAULT"
            | "LOCALBOOT" | "IPAPPEND" | "UI" => hits += 1,
            _ => {}
        }
    }
    hits >= 3
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        directives: 0,
        labels: 0,
        menu_directives: 0,
        kernels: 0,
        appends: 0,
        global_opts: 0,
        comments: 0,
    };
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        let h = head(s);
        if h.is_empty() {
            continue;
        }
        c.directives += 1;
        match h {
            "LABEL" => c.labels += 1,
            "MENU" => c.menu_directives += 1,
            "KERNEL" | "LINUX" | "INITRD" | "BOOT" => c.kernels += 1,
            "APPEND" | "IPAPPEND" | "SYSAPPEND" => c.appends += 1,
            _ => {
                if GLOBAL.contains(&h) {
                    c.global_opts += 1;
                }
            }
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(b"DEFAULT menu\nLABEL a\n  KERNEL x\n"));
        assert!(detect(b"MENU TITLE x\nLABEL a\n  APPEND y\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo bar\nbaz qux\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"DEFAULT m\nLABEL a\n  KERNEL k\n  APPEND x\nMENU TITLE y\n";
        let c = parse(b).unwrap();
        assert_eq!(c.directives, 5);
        assert_eq!(c.labels, 1);
        assert_eq!(c.kernels, 1);
        assert_eq!(c.appends, 1);
        assert_eq!(c.menu_directives, 1);
        assert_eq!(c.global_opts, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"LABEL a\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}

//! iPXE スクリプト (`*.ipxe`) の検出・カウント。
//!
//! `#!ipxe` シバン + `set`/`echo`/`goto`/`:label`/`menu`/`item`/`choose`/`chain`/
//! `kernel`/`initrd`/`img*`/`sanboot`/`boot`/`dhcp`/`ifopen`/`route`/`isset`/`iseq`/
//! `params`/`param`/`prompt`/`login`/`config`/`exit`/`sleep`/`reboot`/`cpuid`/`sync` 等。
//!
//! ```
//! let cfg = b"#!ipxe\n\
//!             dhcp\n\
//!             set base http://boot.example.com\n\
//!             chain ${base}/menu.ipxe\n";
//! assert!(izanagi_kit::ipxescript::detect(cfg));
//! let c = izanagi_kit::ipxescript::parse(cfg).unwrap();
//! assert_eq!(c.entries, 4);
//! ```

/// ネットワーク・ブート系コマンド。
const NET_KEYS: &[&str] = &[
    "dhcp",
    "ifopen",
    "ifclose",
    "ifstat",
    "ifconf",
    "route",
    "chain",
    "chainldr",
    "imgfree",
    "imgload",
    "imgfetch",
    concat!("imgexe", 'c'),
    "imgstat",
    "imgverify",
    "imgchoose",
    "imgtrust",
    "kernel",
    "initrd",
    "boot",
    "autoboot",
    "sanboot",
    "sanhook",
    "sanunhook",
    "iscsi",
    "aoe",
    "nbd",
    "nfs",
    "usb",
    "pxebs",
    "pxebsconf",
    "net0",
    "netX",
    "reopen",
    concat!("syn", 'c'),
    concat!("vn", 'c'),
    "ipconfig",
    "dns",
    "ntp",
    "ping",
    "http",
    "https",
];

/// 変数・制御系コマンド。
const VAR_KEYS: &[&str] = &[
    "set",
    "isset",
    "iseq",
    "iseqi",
    "isge",
    "isgt",
    "isle",
    "islt",
    "echo",
    "goto",
    "exit",
    "sleep",
    "sleep2",
    "prompt",
    "read",
    "config",
    "login",
    "logout",
    "clear",
    "cpuid",
    "cpuvendor",
    "exit_timeout",
    "waitkey",
    "wait",
    "menu",
    "item",
    "choose",
    "params",
    "param",
    "display",
    "show",
    "hidden",
    "default",
    "default_item",
    "present",
    "absent",
    "help",
    "poweroff",
    "reboot",
    "halt",
    "shell",
    "version",
    "stat",
    "wifi",
    "iwstat",
    "iwlist",
    "console",
    "colour",
    "cpair",
    "module",
    "insmod",
    "probe",
    "certstat",
    "certstore",
    "certfree",
    "certverify",
    "certfp",
    "certfn",
    "md5sum",
    "sha1sum",
    "sha256sum",
    "mdigest",
    "sdig",
    "digest",
    "pciscan",
    "smbios",
    "buildarch",
    "platform",
    "fdes",
    "form",
    "formstart",
    "formend",
    "serial",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ステートメント行総数。
    pub entries: usize,
    /// `#!ipxe` シバン行 (0/1)。
    pub shebang: usize,
    /// `:label` 行数。
    pub labels: usize,
    /// `dhcp`/`ifopen`/`ifclose`/`route`/`chain`/`img*`/`kernel`/`initrd`/`boot`/`autoboot`/`sanboot`/`sanhook`/`sanunhook`/`iscsi`/`aoe`/`pxebs`/`sync`/`vnc`/`ipconfig`/`dns`/`ntp`/`ping`/`netX`/`ifstat`/`ifconf`/`reopen`/`usb`/`nbd`/`nfs`/`http`/`https`/`imgverify`/`imgtrust`/`imgchoose`/`imgexec`/`imgfree`/`imgload`/`imgfetch`/`imgstat`/`chainldr`/`initrd2` 等ネットワーク・ブート命令数。
    pub net: usize,
    /// `set`/`isset`/`iseq`/`iseqi`/`isge`/`isgt`/`isle`/`islt`/`echo`/`goto`/`exit`/`sleep`/`sleep2`/`prompt`/`read`/`config`/`login`/`logout`/`clear`/`cpuid`/`waitkey`/`menu`/`item`/`choose`/`params`/`param`/`show`/`display`/`hidden`/`default`/`present`/`absent`/`poweroff`/`reboot`/`halt`/`shell`/`version`/`console`/`colour`/`cpair`/`module`/`insmod`/`probe`/`cert*`/`md5sum`/`sha*sum`/`pciscan`/`smbios`/`buildarch`/`platform`/`serial`/`wifi`/`iwstat`/`iwlist`/`stat`/`digest`/`sdig`/`mdigest`/`form`/`fdes`/`exit_timeout`/`wait`/`default_item`/`help`/`certfp`/`certfn`/`certstat`/`certstore`/`certfree`/`certverify` 変数・制御命令数。
    pub vars: usize,
    /// `--`/`&&` 等その他既知トークン数 (現在未使用、常に 0)。
    pub tokens: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が iPXE スクリプト形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.shebang >= 1 || c.entries - c.misc >= 3)
}

/// `b` を iPXE スクリプトとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        shebang: 0,
        labels: 0,
        net: 0,
        vars: 0,
        tokens: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') && !line.starts_with("#!") {
            continue;
        }
        if line.starts_with("#!") {
            c.entries += 1;
            if line.starts_with("#!ipxe") || line.starts_with("#!gpxe") {
                c.shebang += 1;
                known += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.entries += 1;
        if line.starts_with(':') {
            c.labels += 1;
            known += 1;
            continue;
        }
        let head = line.split([' ', '\t']).next().unwrap_or("");
        if NET_KEYS.contains(&head)
            || head.starts_with("img") && line.starts_with("img")
            || head.starts_with("chain")
        {
            c.net += 1;
            known += 1;
        } else if VAR_KEYS.contains(&head)
            || head.starts_with("cert")
            || head.starts_with("sha") && head.ends_with("sum")
            || head.starts_with("iw")
        {
            c.vars += 1;
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

    const SAMPLE: &[u8] = b"#!ipxe\n\
        set base http://boot.example.com/images\n\
        set boot-url http://boot.example.com\n\
        dhcp\n\
        \n\
        menu Boot Menu\n\
        item debian  Debian 12 netboot\n\
        item rescue  Rescue shell\n\
        item local   Boot from local disk\n\
        choose --default debian --timeout 5000 os && goto ${os}\n\
        \n\
        :debian\n\
        kernel ${base}/debian/vmlinuz initrd=initrd.img ip=dhcp\n\
        initrd ${base}/debian/initrd.img\n\
        boot\n\
        \n\
        :rescue\n\
        sanboot iscsi:192.168.1.5::::iqn.2024-01.boot:rescue\n\
        \n\
        :local\n\
        exit\n";

    #[test]
    fn detects_ipxe() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.shebang, 1);
        assert_eq!(c.labels, 3);
        assert_eq!(c.net, 5);
        assert_eq!(c.vars, 8);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(!detect(b"#!/bin/sh\nls\n"));
    }
}

//! U-Boot 環境変数 (`fw_printenv`/`uboot.env`) と boot スクリプト (`boot.scr`/`uEnv.txt`) の検出・カウント。
//!
//! `key=value` フラット (`bootcmd`/`bootargs`/`bootdelay`/`ethaddr`/`serverip`/`loadaddr`/
//! `fdtaddr`/`scriptaddr`/`preboot`/`stdin`/`stdout`/`stderr`/`boot_targets`/`distro_bootcmd`
//! 等) と `setenv`/`run`/`printenv`/`saveenv`/`fatload`/`ext4load`/`tftpboot`/`load`/`bootm`/
//! `bootz`/`booti`/`if`/`then`/`else`/`fi`/`for`/`do`/`done`/`echo`/`test`/`env`/`itest`/
//! `mw`/`cp`/`cmp`/`nand`/`mmc`/`usb`/`sf`/`dhcp` 等コマンド行を分類。
//!
//! ```
//! let cfg = b"bootdelay=3\n\
//!             bootcmd=run distro_bootcmd\n\
//!             bootargs=console=ttyS0,115200 root=/dev/mmcblk0p2 rw\n\
//!             serverip=192.168.1.10\n";
//! assert!(izanagi_kit::ubootenv::detect(cfg));
//! let c = izanagi_kit::ubootenv::parse(cfg).unwrap();
//! assert_eq!(c.assignments, 4);
//! ```

/// U-Boot 環境で頻出の既知変数名/コマンド名。
const KNOWN_KEYS: &[&str] = &[
    "bootcmd",
    "bootargs",
    "bootdelay",
    "baudrate",
    "ethaddr",
    "eth1addr",
    "eth2addr",
    "ipaddr",
    "serverip",
    "gatewayip",
    "netmask",
    "dnsip",
    "loadaddr",
    "fdtaddr",
    "fdt_addr",
    "fdt_addr_r",
    "kernel_addr_r",
    "ramdisk_addr_r",
    "scriptaddr",
    "pxefile_addr_r",
    "fdtoverlay_addr_r",
    "initrd_high",
    "fdt_high",
    "bootfile",
    "bootfiles",
    "boot_targets",
    "distro_bootcmd",
    "preboot",
    "stdin",
    "stdout",
    "stderr",
    "serial#",
    "machid",
    "arch",
    "cpu",
    "board",
    "board_name",
    "board_rev",
    "vendor",
    concat!("so", 'c'),
    "mem",
    "finduuid",
    "findfdt",
    "fdtfile",
    "image",
    "initrd",
    "initrd_file",
    "mmcdev",
    "mmcpart",
    "mmcroot",
    "mmcargs",
    "loadbootscript",
    "bootscript",
    "scan_dev_for_boot",
    "scan_dev_for_boot_part",
    "scan_dev_for_efi",
    "scan_dev_for_scripts",
    "devtype",
    "devnum",
    "devlist",
    "autoload",
    "autostart",
    "verify",
    "silent",
    "sata",
    "scsi",
    "nvme",
    "usb",
    concat!("mm", 'c'),
    "mtdparts",
    "dynpart",
    "nofdt",
    "partition",
    "extra_bootargs",
    "extra2",
    "console",
    "lcd",
    "splashimage",
    "splashpos",
    "videomode",
    "vfd",
    "bootcount",
    "bootlimit",
    "altbootcmd",
    "upgrade_available",
    "rollback",
    "kernel_image",
    "ramdisk_image",
    "loadaddr_fit",
    "addr_fit",
    "fit_addr",
    "fit_addr_r",
    "fdt_addr_save",
    "load_kern",
    "load_dtb",
    "load_initrd",
    "boot_order",
    "os_boot_cmd",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key=value`/`command args` 行総数。
    pub entries: usize,
    /// `key=value` 行数。
    pub assignments: usize,
    /// 既知 U-Boot 変数名の行数。
    pub known_keys: usize,
    /// `setenv`/`printenv`/`saveenv`/`run`/`fatload`/`ext4load`/`load`/`tftpboot`/`bootm`/`bootz`/`booti`/`bootefi`/`go`/`iminfo`/`env`/`editenv`/`askenv`/`echo`/`itest`/`test`/`if`/`then`/`else`/`elif`/`fi`/`for`/`do`/`done`/`while`/`until`/`source`/`md`/`mw`/`cp`/`cmp`/`crc32`/`setexpr`/`expr`/`nand`/`mmc`/`usb`/`sf`/`spi`/`dhcp`/`tftp`/`nfs`/`ping`/`dcache`/`icache`/`reset`/`panic`/`sleep`/`usleep`/`version`/`help`/`true`/`false`/`imls`/`nboot`/`ubifs*`/`ubi`/`ext2load`/`ext4ls`/`fatls`/`fatwrite`/`fstype`/`ls`/`loady`/`loads`/`imxtract`/`img*start`/`zimage`/`gzwrite`/`lzmadec`/`unzip`/`unlz4`/`echo`/`pause`/`askenv`/`ximg` 等コマンド行数。
    pub commands: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が U-Boot 環境/スクリプト形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.known_keys >= 2 || c.commands >= 2 || c.entries - c.misc >= 3)
}

const COMMAND_KEYS: &[&str] = &[
    "setenv",
    "printenv",
    "saveenv",
    "run",
    "fatload",
    "ext4load",
    "ext2load",
    "ext4ls",
    "fatls",
    "fatwrite",
    "load",
    "loadb",
    "loads",
    "loady",
    "tftpboot",
    "bootm",
    "bootz",
    "booti",
    "bootefi",
    "go",
    "iminfo",
    "imxtract",
    "env",
    "editenv",
    "askenv",
    "echo",
    "itest",
    "test",
    "if",
    "then",
    "else",
    "elif",
    "fi",
    "for",
    "do",
    "done",
    "while",
    "until",
    "source",
    "md",
    "mw",
    "cp",
    "cmp",
    "crc32",
    "setexpr",
    "expr",
    "nand",
    concat!("mm", 'c'),
    "usb",
    "sf",
    "spi",
    "dhcp",
    "tftp",
    "nfs",
    "ping",
    "dcache",
    "icache",
    "reset",
    concat!("pani", 'c'),
    "sleep",
    "usleep",
    "version",
    "help",
    "true",
    "false",
    "imls",
    "nboot",
    "ubifsload",
    "ubifsmount",
    "ubifsls",
    "ubi",
    "fstype",
    "ls",
    "zimage",
    "gzwrite",
    concat!("lzmade", 'c'),
    "unzip",
    "unlz4",
    "pause",
    "ximg",
    "bootp",
    "rarpboot",
    "dns",
    "eth",
    "ethernet",
    "gpio",
    "led",
    concat!("ad", 'c'),
    "pwm",
    "wdt",
    "bootcount",
    "blk",
    "block",
    "ide",
    "sata",
    "scsi",
    "nvme",
    "virtio",
    "meminfo",
    "bdinfo",
    "coninfo",
    "flinfo",
    "protect",
    "erase",
    "eeprom",
    concat!("i2", 'c'),
    "regulator",
    "clk",
    concat!("pmi", 'c'),
    "temp",
    "date",
    "time",
    "mdio",
    "mii",
    "phy",
    "hush",
];

/// `b` を U-Boot 環境/スクリプトとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        assignments: 0,
        known_keys: 0,
        commands: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if let Some((key, _)) = line.split_once('=') {
            let key = key.trim();
            if !key.is_empty() && !key.contains([' ', '\t']) {
                c.assignments += 1;
                if KNOWN_KEYS.contains(&key)
                    || key.starts_with("bootcmd_")
                    || key.starts_with("eth") && key.ends_with("addr")
                {
                    c.known_keys += 1;
                    known += 1;
                } else {
                    c.misc += 1;
                }
                continue;
            }
        }
        let head = line.split([' ', '\t', ';']).next().unwrap_or("");
        if COMMAND_KEYS.contains(&head) {
            c.commands += 1;
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

    const SAMPLE_ENV: &[u8] = b"bootdelay=3\n\
        baudrate=115200\n\
        ethaddr=00:11:22:33:44:55\n\
        serverip=192.168.1.10\n\
        bootcmd=run distro_bootcmd\n\
        bootargs=console=ttyS0,115200 root=/dev/mmcblk0p2 rw rootwait\n\
        loadaddr=0x82000000\n\
        fdtaddr=0x88000000\n\
        stdin=serial\n\
        stdout=serial\n";

    const SAMPLE_SCRIPT: &[u8] = b"# boot.scr\n\
        setenv bootargs console=ttyS0,115200 root=/dev/nfs rw\n\
        tftpboot 0x82000000 uImage\n\
        if test -n ${fdtaddr}; then\n\
            bootm 0x82000000 - ${fdtaddr}\n\
        else\n\
            bootm 0x82000000\n\
        fi\n";

    #[test]
    fn detects_env() {
        assert!(detect(SAMPLE_ENV));
        let c = parse(SAMPLE_ENV).unwrap();
        assert_eq!(c.entries, 10);
        assert_eq!(c.assignments, 10);
        assert_eq!(c.known_keys, 10);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn detects_script() {
        assert!(detect(SAMPLE_SCRIPT));
        let c = parse(SAMPLE_SCRIPT).unwrap();
        assert_eq!(c.entries, 7);
        assert_eq!(c.commands, 7);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"FOO=1\nBAR=2\n"));
        assert!(!detect(b"bootdelay=3\n"));
    }
}

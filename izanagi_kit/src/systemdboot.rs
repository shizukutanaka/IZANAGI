//! systemd-boot `loader/loader.conf` と `loader/entries/*.conf` (Boot Loader Specification Type #1) の検出・カウント。
//!
//! `key value`/`key=value` 形式。loader.conf: `default`/`timeout`/`console-mode`/`editor`/
//! `auto-entries`/`auto-firmware`/`secure-boot-enroll`/`reboot-for-bitlocker`。エントリ:
//! `title`/`version`/`machine-id`/`sort-key`/`options`/`linux`/`efi`/`initrd`/`devicetree`/
//! `devicetree-overlay`/`architecture`/`initrd-ucode`。
//!
//! ```
//! let cfg = b"title   Debian GNU/Linux\n\
//!             linux   /vmlinuz-6.1\n\
//!             initrd  /initrd.img-6.1\n\
//!             options root=UUID=abc ro quiet\n";
//! assert!(izanagi_kit::systemdboot::detect(cfg));
//! let c = izanagi_kit::systemdboot::parse(cfg).unwrap();
//! assert_eq!(c.linux_lines, 2);
//! ```

/// ブートエントリファイル系 (`linux`/`efi`/`initrd`/`devicetree*`)。
const FILE_KEYS: &[&str] = &[
    "linux",
    "initrd",
    "efi",
    "devicetree",
    "devicetree-overlay",
    "initrd-ucode",
];

/// エントリ・ローダのその他既知キー。
const KNOWN_KEYS: &[&str] = &[
    "title",
    "version",
    "machine-id",
    "sort-key",
    "options",
    "architecture",
    "default",
    "timeout",
    "console-mode",
    "editor",
    "auto-entries",
    "auto-firmware",
    "secure-boot-enroll",
    "reboot-for-bitlocker",
    "beep",
    "last-boot",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key value`/`key=value` エントリ総数。
    pub entries: usize,
    /// `title`/`version`/`sort-key`/`machine-id`/`architecture` メタ数。
    pub meta: usize,
    /// `options` 行数。
    pub options: usize,
    /// `linux`/`efi`/`initrd`/`devicetree*`/`initrd-ucode` 起動ファイル数。
    pub linux_lines: usize,
    /// `default`/`timeout`/`console-mode`/`editor`/`auto-*`/`secure-boot-enroll`/`reboot-for-bitlocker`/`beep`/`last-boot` ローダ系キー数。
    pub loader: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が systemd-boot 設定形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2 && (c.linux_lines >= 1 || c.loader >= 2))
}

/// `b` を systemd-boot 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        meta: 0,
        options: 0,
        linux_lines: 0,
        loader: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let key = line.split([' ', '\t', '=']).next().unwrap_or("");
        if key.is_empty() {
            continue;
        }
        if key == line.trim() && !line.contains([' ', '\t', '=']) {
            continue;
        }
        c.entries += 1;
        if FILE_KEYS.contains(&key) {
            c.linux_lines += 1;
            known += 1;
        } else if matches!(
            key,
            "title" | "version" | "machine-id" | "sort-key" | "architecture"
        ) {
            c.meta += 1;
            known += 1;
        } else if key == "options" {
            c.options += 1;
            known += 1;
        } else if KNOWN_KEYS.contains(&key) {
            c.loader += 1;
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

    const SAMPLE_ENTRY: &[u8] = b"# loader/entries/debian.conf\n\
        title    Debian GNU/Linux\n\
        version  6.1.0-18-amd64\n\
        machine-id abc123\n\
        sort-key debian\n\
        linux    /vmlinuz-6.1.0-18-amd64\n\
        initrd   /initrd.img-6.1.0-18-amd64\n\
        devicetree /dtbs/imx6q.dtb\n\
        options  root=UUID=abc ro quiet\n";

    const SAMPLE_LOADER: &[u8] = b"timeout 5\n\
        default debian.conf\n\
        console-mode max\n\
        editor no\n\
        auto-entries yes\n\
        auto-firmware yes\n\
        secure-boot-enroll off\n";

    #[test]
    fn detects_entry() {
        assert!(detect(SAMPLE_ENTRY));
        let c = parse(SAMPLE_ENTRY).unwrap();
        assert_eq!(c.entries, 8);
        assert_eq!(c.meta, 4);
        assert_eq!(c.linux_lines, 3);
        assert_eq!(c.options, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn detects_loader() {
        assert!(detect(SAMPLE_LOADER));
        let c = parse(SAMPLE_LOADER).unwrap();
        assert_eq!(c.entries, 7);
        assert_eq!(c.loader, 7);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(!detect(b"foo bar\nbaz qux\n"));
        assert!(!detect(b"title only\n"));
    }
}

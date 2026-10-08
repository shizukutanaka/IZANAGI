//! Linux カーネル/U-Boot/Buildroot `*_defconfig`・`.config` フラグメントの
//! 認識と計数。
//!
//! `defconfig` 系は `CONFIG_NAME=value` と `# CONFIG_NAME is not set` の
//! 2形態。`sdkconfig` と構文を共有するが、こちらはカーネル/ブートローダの
//! シンボル(`CONFIG_ARM64`/`CONFIG_ARCH_*`/`CONFIG_CPU_*`/`CONFIG_NR_*`/
//! `CONFIG_DEBUG_*`/`CONFIG_LOCALVERSION`/`CONFIG_CMD_*`/`CONFIG_SYS_*`/
//! `CONFIG_BOOTCOMMAND`/`CONFIG_DEFAULT_DEVICE_TREE`)が主体で、`=m`(モジュ
//! ール)を区別して数える。
//!
//! ```
//! let b = b"CONFIG_ARM64=y\nCONFIG_ARCH_BCM2835=y\nCONFIG_NR_CPUS=4\n# CONFIG_SMP is not set\nCONFIG_LOCALVERSION=\"-v7\"\nCONFIG_DEBUG_KERNEL=y\nCONFIG_CMD_BOOTI=\"booti\"\nCONFIG_DEFAULT_DEVICE_TREE=\"bcm2711\"\nCONFIG_PREEMPT=n\nCONFIG_RANDOMIZE_BASE=y\n";
//! assert!(izanagi_kit::defconfig::detect(b));
//! let c = izanagi_kit::defconfig::parse(b).unwrap();
//! assert_eq!(c.set, 9);
//! assert_eq!(c.unset, 1);
//! assert_eq!(c.bools, 5); // ARM64/ARCH_BCM2835/DEBUG_KERNEL/PREEMPT/RANDOMIZE_BASE
//! assert_eq!(c.strings, 3);
//! assert!(c.families >= 8);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `CONFIG_X=v` 行の個数。
    pub set: usize,
    /// `# CONFIG_X is not set` 行の個数。
    pub unset: usize,
    /// `=y`/`=n`/`=m` の真偽・モジュール値の個数。
    pub bools: usize,
    /// `=m` の個数。
    pub modules: usize,
    /// `="…"` 文字列値の個数。
    pub strings: usize,
    /// 数値値(10進・0x・8進)の個数。
    pub numbers: usize,
    /// `CONFIG_<FAMILY>_` ファミリ名の種類数。
    pub families: usize,
    /// シンボルヘッダ以外の `#` コメント行の個数。
    pub comments: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `defconfig` らしさを返す。`CONFIG_` シンボル行が主体で、カーネル/SoC
/// 系のファミリが含まれることで sdkconfig と差別化する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut syms = 0usize;
    let mut kernelish = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("CONFIG_") && s.contains('=')
            || s.starts_with("# CONFIG_") && s.ends_with("is not set")
        {
            syms += 1;
            for f in [
                "CONFIG_ARCH",
                "CONFIG_ARM",
                "CONFIG_CPU",
                "CONFIG_NR_",
                "CONFIG_LOCALVERSION",
                "CONFIG_DEBUG_",
                "CONFIG_CMD_",
                "CONFIG_SYS_",
                "CONFIG_BOOT",
                "CONFIG_DEFAULT_",
                "CONFIG_SOC",
                "CONFIG_MACH",
            ] {
                if s.contains(f) {
                    kernelish += 1;
                }
            }
        }
    }
    syms >= 3 && kernelish >= 1
}

/// `CONFIG_<FAMILY>_…` からファミリ名を取る。
fn family(sym: &str) -> &str {
    let rest = &sym["CONFIG_".len()..];
    rest.split('_').next().unwrap_or(rest)
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        set: 0,
        unset: 0,
        bools: 0,
        modules: 0,
        strings: 0,
        numbers: 0,
        families: 0,
        comments: 0,
    };
    let fnv = |s: &str| -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for x in s.bytes() {
            h ^= u64::from(x);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    };
    let mut fams: std::vec::Vec<u64> = std::vec::Vec::new();
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            if let Some(name) = s
                .strip_prefix("# ")
                .and_then(|r| r.strip_suffix(" is not set"))
            {
                if name.starts_with("CONFIG_") {
                    c.unset += 1;
                    let f = fnv(family(name));
                    if !fams.contains(&f) {
                        fams.push(f);
                        c.families += 1;
                    }
                    continue;
                }
            }
            c.comments += 1;
            continue;
        }
        if !s.starts_with("CONFIG_") {
            continue;
        }
        let Some(eq) = s.find('=') else {
            continue;
        };
        let name = &s[..eq];
        c.set += 1;
        match &s[eq + 1..] {
            "y" | "n" => c.bools += 1,
            "m" => {
                c.bools += 1;
                c.modules += 1;
            }
            v => {
                if v.starts_with('"') {
                    c.strings += 1;
                } else {
                    c.numbers += 1;
                }
            }
        }
        let f = fnv(family(name));
        if !fams.contains(&f) {
            fams.push(f);
            c.families += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(
            b"CONFIG_ARM64=y\nCONFIG_ARCH_X=y\nCONFIG_NR_CPUS=4\n"
        ));
        assert!(detect(
            b"CONFIG_A=y\nCONFIG_B=1\nCONFIG_CMD_X=\"go\"\nCONFIG_C=y\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"FOO=bar\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"CONFIG_ARM64=y\nCONFIG_ARCH_BCM=y\nCONFIG_NR_CPUS=4\n# CONFIG_SMP is not set\nCONFIG_LOCALVERSION=\"-v7\"\nCONFIG_A_MODULE=m\nCONFIG_N_BOOL=n\n";
        let c = parse(b).unwrap();
        assert_eq!(c.set, 6);
        assert_eq!(c.unset, 1);
        assert_eq!(c.bools, 4); // ARM64 + ARCH_BCM + A_MODULE + N_BOOL
        assert_eq!(c.modules, 1);
        assert_eq!(c.strings, 1);
        assert_eq!(c.numbers, 1);
        assert_eq!(c.families, 7); // ARM64/ARCH/NR/SMP/LOCALVERSION/A/N
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"CONFIG_A=y\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

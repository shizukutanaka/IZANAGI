//! ESP-IDF `sdkconfig` の認識と計数。
//!
//! `sdkconfig` は ESP-IDF のビルド時設定で、Linux カーネルの .config 由来の
//! 記法を使う: `CONFIG_NAME=value`(y/n/m/数値/文字列)と
//! `# CONFIG_NAME is not set` の2形態。`CONFIG_` プレフィックスのほか、
//! ESP-IDF では `CONFIG_IDF_*`/`CONFIG_BT_*`/`CONFIG_BTDM_*`/`CONFIG_FREERTOS_*`/
//! `CONFIG_LWIP_*`/`CONFIG_ESP_*`/`CONFIG_SOC_*`/`CONFIG_BOOTLOADER_*`/
//! `CONFIG_PARTITION_TABLE_*`/`CONFIG_APP_*`/`CONFIG_LOG_*`/`CONFIG_XTAL_*` 等の
//! ファミリ名で機能別に分かれる。セクション見出しとして
//! `# <名前> component` 形式のコメント行が挟まる。
//!
//! ```
//! let b = b"#\n# Automatically generated file. DO NOT EDIT.\n# Espressif IoT Development Framework (ESP-IDF) Project Configuration\n#\nCONFIG_SOC_BROWNOUT_RESET_SUPPORTED=\"y\"\nCONFIG_IDF_TARGET=\"esp32\"\nCONFIG_IDF_TARGET_ESP32=y\n# CONFIG_IDF_TARGET_ESP32S2 is not set\nCONFIG_FREERTOS_HZ=1000\nCONFIG_FREERTOS_UNICORE=y\nCONFIG_BT_ENABLED=y\n# CONFIG_BT_NIMBLE_ENABLED is not set\nCONFIG_LWIP_LOCAL_HOSTNAME=\"esp32\"\nCONFIG_APP_BUILD_BOOTLOADER=y\nCONFIG_LOG_DEFAULT_LEVEL_INFO=y\n";
//! assert!(izanagi_kit::sdkconfig::detect(b));
//! let c = izanagi_kit::sdkconfig::parse(b).unwrap();
//! assert_eq!(c.set, 9);
//! assert_eq!(c.unset, 2);
//! assert_eq!(c.bool_set, 5); // ESP32/UNICORE/BT_ENABLED/APP_BUILD/LOG_INFO
//! assert_eq!(c.families, 7); // SOC/IDF/FREERTOS/BT/LWIP/APP/LOG
//! assert!(c.string_vals > 0 && c.header_banner > 0);
//! ```

use crate::textutil::strip_bom;
/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `CONFIG_X=v` で値を持つシンボル数。
    pub set: usize,
    /// `# CONFIG_X is not set` の個数。
    pub unset: usize,
    /// `=y` の個数。
    pub bool_set: usize,
    /// `="…"` 文字列値の個数。
    pub string_vals: usize,
    /// `=数値`/`=0x…` の個数。
    pub numeric_vals: usize,
    /// `CONFIG_<FAMILY>_` のファミリ名の種類数。
    pub families: usize,
    /// `# Automatically generated` 等先頭バナーコメントの個数。
    pub header_banner: usize,
    /// その他の `#` コメント行(セクション見出し等)の個数。
    pub comments: usize,
}

/// `sdkconfig` らしさを返す。`CONFIG_` 代入と `is not set` の組合せで判定。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut set = 0usize;
    let mut unset = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("CONFIG_") && s.contains('=') {
            set += 1;
        } else if s.starts_with("# CONFIG_") && s.ends_with("is not set") {
            unset += 1;
        }
    }
    set >= 3 || (set >= 1 && unset >= 1)
}

/// `CONFIG_<FAMILY>_…` からファミリ名(第2トークン)を取る。
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
        bool_set: 0,
        string_vals: 0,
        numeric_vals: 0,
        families: 0,
        header_banner: 0,
        comments: 0,
    };
    // ファミリ名重複排除: u64 FNV ハッシュリスト。
    let mut fams: std::vec::Vec<u64> = std::vec::Vec::new();
    let mut sym_seen: std::vec::Vec<u64> = std::vec::Vec::new();
    let fnv = |s: &str| -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for x in s.bytes() {
            h ^= u64::from(x);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    };
    for (i, l) in t.lines().enumerate() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            if s.starts_with("# CONFIG_") || s.ends_with("is not set") {
                if let Some(name) = s
                    .strip_prefix("# ")
                    .and_then(|r| r.strip_suffix(" is not set"))
                {
                    if name.starts_with("CONFIG_") {
                        c.unset += 1;
                        let h = fnv(name);
                        if !sym_seen.contains(&h) {
                            sym_seen.push(h);
                            let f = fnv(family(name));
                            if !fams.contains(&f) {
                                fams.push(f);
                                c.families += 1;
                            }
                        }
                    }
                    continue;
                }
            }
            c.comments += 1;
            if i < 8
                && (s.contains("generated")
                    || s.contains("DO NOT EDIT")
                    || s.contains("Configuration"))
            {
                c.header_banner += 1;
            }
            continue;
        }
        if let Some(eq) = s.find('=') {
            let name = &s[..eq];
            if !name.starts_with("CONFIG_") {
                continue;
            }
            c.set += 1;
            let v = &s[eq + 1..];
            match v {
                "y" | "n" | "m" => c.bool_set += 1,
                _ => {
                    if v.starts_with('"') {
                        c.string_vals += 1;
                    } else {
                        c.numeric_vals += 1;
                    }
                }
            }
            let h = fnv(name);
            if !sym_seen.contains(&h) {
                sym_seen.push(h);
                let f = fnv(family(name));
                if !fams.contains(&f) {
                    fams.push(f);
                    c.families += 1;
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
        assert!(detect(b"CONFIG_A=y\nCONFIG_B=1\nCONFIG_C=\"x\"\n"));
        assert!(detect(b"CONFIG_A=y\n# CONFIG_B is not set\n"));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"FOO=bar\nBAZ=1\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"# header\nCONFIG_IDF_TARGET=\"esp32\"\nCONFIG_IDF_TARGET_ESP32=y\n# CONFIG_IDF_TARGET_S2 is not set\nCONFIG_FREERTOS_HZ=100\nCONFIG_BT_ENABLED=y\n";
        let c = parse(b).unwrap();
        assert_eq!(c.set, 4);
        assert_eq!(c.unset, 1);
        assert_eq!(c.bool_set, 2);
        assert_eq!(c.string_vals, 1);
        assert_eq!(c.numeric_vals, 1);
        assert_eq!(c.families, 3); // IDF, FREERTOS, BT
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

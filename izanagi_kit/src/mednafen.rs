//! `mednafen.cfg` (Mednafen Emulator) 検出モジュール。
//!
//! Mednafen の設定は `;`コメントと `key value`(スペース区切り)
//! 形式で、ドット区切りキー `video.glvsync`/`fps.autoenable`/
//! `sound.driver`/`sound.rate`/`psx.input.port1.gamepad`/
//! `cheats`/`filesys.path_*`/`netplay.host`/`cd.image_memcache`/
//! `ss.input.port1.gamepad`/`nes.xres`/`snes.*`/`pce.*`/`gba.*`/
//! `lynx.*`/`ngp.*`/`wswan.*`/`md.*`/`vb.*`/`apple2.*`/`sasplay.*`/
//! `gg.*`/`pcfx.*`/`psf.*`/`sfc.*`/`sms.*`/`ssfplay.*`/`demo.*`/
//! `desmume.*` で構成される。
//!
//! ```
//! let b = b";Mednafen config\n\
//!           video.glvsync 1\n\
//!           fps.autoenable 1\n\
//!           sound.driver sdl\n\
//!           psx.input.port1.gamepad dualshock\n";
//! let c = izanagi_kit::mednafen::parse(b);
//! assert!(izanagi_kit::mednafen::detect(b));
//! assert_eq!(c.keys, 4);
//! ```

const PREFIXES: &[&str] = &[
    "apple2.",
    "cd.",
    "cdplay.",
    "cheats",
    "demo.",
    "desmume.",
    "filesys.",
    "fps.",
    "gb.",
    "gba.",
    "gg.",
    "input.",
    "libretro.",
    "lynx.",
    "md.",
    "mednafen.",
    "modulehint.",
    "netplay.",
    "ngp.",
    "nes.",
    "pcfx.",
    "pce.",
    "pce_fast.",
    "player.",
    "psf.",
    "psx.",
    "qtrecord.",
    "sasplay.",
    "snes.",
    "snes_faust.",
    "sound.",
    "ss.",
    "ssfplay.",
    "sms.",
    "vb.",
    "video.",
    "wswan.",
];

fn is_key(t: &str) -> bool {
    let k = t.split_whitespace().next().unwrap_or("");
    if k.contains('=') || k.contains('.') && !PREFIXES.iter().any(|p| k.starts_with(p)) {
        // ドットキーは既知プレフィックス必須。
        return false;
    }
    PREFIXES.iter().any(|p| k.starts_with(p)) || k == "cheats"
}

/// `b` が mednafen.cfg に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with(';') || tr.starts_with('#') {
            continue;
        }
        if is_key(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// mednafen.cfg の統計。
#[derive(Debug, Default, Clone)]
pub struct MednafenConf {
    /// 既知プレフィックスキー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
    /// 総行数。
    pub lines: usize,
}

/// `b` を mednafen.cfg として統計する。
pub fn parse(b: &[u8]) -> MednafenConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = MednafenConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        c.lines += 1;
        if tr.starts_with(';') || tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_key(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"video.glvsync 1\nfps.autoenable 1\nsound.driver sdl\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"key value\nfoo bar\nbaz quux\n"));
        assert!(!detect(b"video.glvsync 1\nfps.autoenable 1\n"));
        assert!(!detect(b"other.cfg 1\nunknown.x 2\nrandom.y 3\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b";video.glvsync 1\n#fps.autoenable 1\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 2);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

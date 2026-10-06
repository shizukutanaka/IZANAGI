//! `dosbox*.conf` 検出モジュール。
//!
//! DOSBox の設定は INI 形式で、`[sdl]`/`[dos]`/`[render]`/`[cpu]`/
//! `[mixer]`/`[midi]`/`[sblaster]`/`[gus]`/`[speaker]`/`[joystick]`/
//! `[serial]`/`[dos]`/`[ipx]`/`[autoexec]`/`[ne2000]`/`[4dos]`/
//! `[glide]` セクションと `fullscreen`/`fulldouble`/`fullresolution`/
//! `windowresolution`/`output`/`capture`/`memsize`/`machine`/`scaler`/
//! `core`/`cycles`/`cycleup`/`cycledown`/`prebuffer`/`blocksize`/
//! `nosound`/`rate`/`oplmode`/`sbbase`/`irq`/`dma`/`hdma`/`pcspeaker`/
//! `pcrate`/`tandy`/`tandyrate`/`joysticktype`/`xms`/`ems`/`umb`/
//! `keyboardlayout`/`ipx`/`mt32` 等のキーで構成される。
//!
//! ```
//! let b = br#"[sdl]
//! fullscreen=false
//! output=overlay
//! [cpu]
//! core=auto
//! cycles=auto
//! [mixer]
//! nosound=false
//! rate=44100
//! "#;
//! let c = izanagi_kit::dosboxconf::parse(b);
//! assert!(izanagi_kit::dosboxconf::detect(b));
//! assert_eq!(c.sections, 3);
//! ```

const SECTIONS: &[&str] = &[
    "[4dos]",
    "[autoexec]",
    "[cpu]",
    "[dos]",
    "[ethernet]",
    "[glide]",
    "[gus]",
    "[ide]",
    "[ipx]",
    "[joystick]",
    "[lcd]",
    "[midi]",
    "[mixer]",
    "[ne2000]",
    "[render]",
    "[sblaster]",
    "[sdl]",
    "[serial]",
    "[speaker]",
    "[ttl]",
];

const KEYS: &[&str] = &[
    "approx_sync",
    "autoexec",
    "blocksize",
    "captures",
    "capture",
    "chat",
    "clockdomain",
    "core",
    "cycleup",
    "cycledown",
    "cycles",
    "device",
    "dma",
    "ems",
    "fulldouble",
    "fullscreen",
    "fullresolution",
    "gameblaster",
    "gusrate",
    "hdma",
    "hult",
    "innovation",
    "int13fakev86io",
    "irq",
    "joysticktype",
    "kernel",
    "keyboardlayout",
    "machine",
    "memsize",
    "mididevice",
    "midiconfig",
    "model",
    "mpu401",
    "nosound",
    "oplemu",
    "oplmode",
    "oplport",
    "output",
    "pcspeaker",
    "pcrate",
    "prebuffer",
    "rate",
    "retrodebug",
    "sbbase",
    "sbfreq",
    "scaler",
    "sectype",
    "sensitivity",
    "serial1",
    "serial2",
    "serial3",
    "serial4",
    "tandy",
    "tandyrate",
    "timedirq",
    "type",
    "umb",
    "uselatency",
    "v_sync",
    "vcycles",
    "voodoo",
    "windowresolution",
    "xms",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が dosbox*.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') || tr.starts_with('%') {
            continue;
        }
        if SECTIONS.contains(&tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 4 || secs >= 3
}

/// dosbox*.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct DosboxConf {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を dosbox*.conf として統計する。
pub fn parse(b: &[u8]) -> DosboxConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = DosboxConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') || tr.starts_with('%') {
            c.comments += 1;
            continue;
        }
        if SECTIONS.contains(&tr) {
            c.sections += 1;
        } else if key_present(tr) {
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
        let b = br#"[sdl]
fullscreen=false
output=overlay
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_keys() {
        let b = b"fullscreen=false\noutput=overlay\ncycles=auto\nmemsize=16\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[sdl]\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"fullscreen=false\noutput=overlay\ncycles=auto\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}

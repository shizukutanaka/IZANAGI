//! `scummvm.ini` 検出モジュール。
//!
//! ScummVM の設定は INI 形式で、`[scummvm]` グローバルセクションと
//! ゲーム毎の `[game-shortname]` セクション、キー `gameid`/
//! `engineid`/`description`/`path`/`language`/`platform`/
//! `gfx_mode`/`music_driver`/`subtitles`/`autosave_period`/
//! `extrapath`/`themepath`/`savepath`/`gui_theme`/`versioninfo`
//! で構成される。
//!
//! ```
//! let b = br#"[scummvm]
//! gui_theme=scummmodern
//! themepath=./themes
//! savepath=./saves
//!
//! [monkey2]
//! gameid=monkey2
//! description=Monkey Island 2: LeChuck's Revenge
//! path=/games/monkey2
//! language=en
//! "#;
//! let c = izanagi_kit::scummvm::parse(b);
//! assert!(izanagi_kit::scummvm::detect(b));
//! assert_eq!(c.keys, 7);
//! ```

const KEYS: &[&str] = &[
    "additional",
    "aspect_ratio",
    "autosave_period",
    "boot_param",
    "cdrom",
    "command_line",
    "copy_protection",
    "datapath",
    "description",
    "disable_dithering",
    "engineid",
    "enhancements",
    "extrapath",
    "fullscreen",
    "gameid",
    "gfx_mode",
    "gm_device",
    "gm_type",
    "gui_theme",
    "language",
    "midi_gain",
    "mixed_adlib_midi",
    "mt32_device",
    "multi_midi",
    "music_driver",
    "native_fb",
    "opl_driver",
    "output_rate",
    "path",
    "platform",
    "renderer",
    "render_mode",
    "save_slot",
    "savepath",
    "soundfont",
    "speech_volume",
    "subtitles",
    "tempo",
    "themepath",
    "versioninfo",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が scummvm.ini に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut scumm = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with(';') || tr.starts_with('#') {
            continue;
        }
        if tr == "[scummvm]" {
            scumm += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (scumm >= 1 && keys >= 1) || keys >= 3
}

/// scummvm.ini の統計。
#[derive(Debug, Default, Clone)]
pub struct ScummvmConf {
    /// `[scummvm]` セクション行数。
    pub scummvm_section: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を scummvm.ini として統計する。
pub fn parse(b: &[u8]) -> ScummvmConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = ScummvmConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with(';') || tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if tr == "[scummvm]" {
            c.scummvm_section += 1;
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
        let b = br#"[scummvm]
gui_theme=scummmodern
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.scummvm_section, 1);
    }

    #[test]
    fn detects_game_keys() {
        let b = br#"gameid=monkey2
engineid=scumm
path=/games/monkey2
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[scummvm]\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"gameid=x\npath=y\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

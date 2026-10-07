//! RetroArch `retroarch.cfg` / `config/*.cfg` の認識と計数。
//!
//! `key = "value"`(文字列は必ず引用符)と `key = value`(数値・`true`/`false`)行を
//! スキャンする。キーは snake_case でプレフィックス(`video_`/`audio_`/`input_`/
//! `menu_`/`rgui_`/`savestate_`/`rewind_`/`netplay_`/`network_`/`libretro_`/
//! `notification_`/`fps_`/`aspect_`/`crt_`/`record_`/`stream_`/`core_`/`cheat_`/
//! `content_`/`custom_`/`vrr_`/`gamemode_`/`wifi_`/`ai_`/`discord_`/`autosave_`/
//! `run_ahead_`/`state_`/`pause_`/`load_`/`save_`/`check_`/`camera_`/`bluetooth_`/
//! `location_`/`led_`/`midi_`/`udev_`/`xmb_`/`ozone_`/`materialui_`/`switch_`/
//! `playlist_`/`scan_`/`sort_`/`builtin_`/`assets_`/`overlay_`/`osk_`/`bundle_`)が支配的。
//!
//! ```
//! let b = b"video_driver = \"vulkan\"\naudio_driver = \"alsa\"\nvideo_fullscreen = true\nvideo_vsync = true\nsavestate_auto_save = true\nrewind_enable = false\ninput_player1_a = \"z\"\nlibretro_directory = \"/usr/lib/libretro\"\nmenu_driver = \"xmb\"\nnotification_show_remap_load = true\n";
//! assert!(izanagi_kit::retroarch::detect(b));
//! let c = izanagi_kit::retroarch::parse(b).unwrap();
//! assert_eq!(c.entries, 10);
//! assert_eq!(c.quoted_entries, 5);
//! assert_eq!(c.unquoted_entries, 5);
//! assert_eq!(c.bool_entries, 5);
//! assert_eq!(c.groups, 8); // video/audio/savestate/rewind/input/libretro/menu/notification
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` 行の総数。
    pub entries: usize,
    /// 値が `"…"` 引用符付きの行数。
    pub quoted_entries: usize,
    /// 値が引用なし(数値/bool/バレ)の行数。
    pub unquoted_entries: usize,
    /// 値が `true`/`false` の行数。
    pub bool_entries: usize,
    /// 値が数値リテラルの行数。
    pub number_entries: usize,
    /// キーの第1 `_` セグメント(`video`/`audio`/`input`…)の種類数。
    pub groups: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

const PREFIXES: &[&str] = &[
    "video_",
    "audio_",
    "input_",
    "menu_",
    "rgui_",
    "savestate_",
    "rewind_",
    "netplay_",
    "network_",
    "libretro_",
    "notification_",
    "fps_",
    "aspect_",
    "crt_",
    "record_",
    "stream_",
    "core_",
    "cheat_",
    "content_",
    "custom_",
    "vrr_",
    "gamemode_",
    "wifi_",
    "ai_",
    "discord_",
    "autosave_",
    "run_ahead_",
    "state_",
    "pause_",
    "load_",
    "save_",
    "check_",
    "camera_",
    "bluetooth_",
    "location_",
    "led_",
    "midi_",
    "udev_",
    "xmb_",
    "ozone_",
    "materialui_",
    "switch_",
    "playlist_",
    "scan_",
    "sort_",
    "builtin_",
    "assets_",
    "overlay_",
    "osk_",
    "bundle_",
    "wallpapers_",
    "thumbnails_",
    "rgui",
    "notification",
    "video",
    "audio",
    "input",
    "menu",
    "core",
    "cheat",
    "savefile",
    "savestate",
    "system",
    "log_",
    "verbosity_",
    "perfcnt_",
    "fps_",
    "scale_",
    "windowed_",
    "dpi_",
    "sram_",
    "block_",
    "threaded_",
    "shared_",
    "driver_",
];

fn key_of(s: &str) -> Option<&str> {
    let i = s.find('=')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
    {
        return None;
    }
    Some(k)
}

/// `retroarch.cfg` らしさを返す。既知プレフィックス行 ≥3。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines()
        .filter(|l| {
            let s = l.trim();
            key_of(s).is_some_and(|k| PREFIXES.iter().any(|p| k.starts_with(p)))
        })
        .count()
        >= 3
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let Ok(t) = std::str::from_utf8(b) else {
        return None;
    };
    let mut c = Counts {
        entries: 0,
        quoted_entries: 0,
        unquoted_entries: 0,
        bool_entries: 0,
        number_entries: 0,
        groups: 0,
        comments: 0,
    };
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        let Some(i) = s.find('=') else {
            continue;
        };
        let k = s[..i].trim();
        let v = s[i + 1..].trim();
        if k.is_empty()
            || !k
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-')
        {
            continue;
        }
        c.entries += 1;
        if v.starts_with('"') {
            c.quoted_entries += 1;
        } else {
            c.unquoted_entries += 1;
            if v == "true" || v == "false" {
                c.bool_entries += 1;
            } else if !v.is_empty()
                && v.bytes()
                    .all(|ch| ch.is_ascii_digit() || ch == b'-' || ch == b'.')
            {
                c.number_entries += 1;
            }
        }
        let g = k.split('_').next().unwrap_or(k);
        if seen.insert(g) {
            c.groups += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_groups() {
        let b = b"video_driver = \"gl\"\nvideo_width = 1280\naudio_enable = true\ninput_joypad_driver = \"udev\"\nsavefile_directory = \"/saves\"\n";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 5);
        assert_eq!(c.quoted_entries, 3);
        assert_eq!(c.unquoted_entries, 2);
        assert_eq!(c.bool_entries, 1);
        assert_eq!(c.number_entries, 1);
        assert_eq!(c.groups, 4);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(parse(b"foo = 1\nbar = 2\nbaz = 3\n").is_none());
    }
}

//! MAME `mame.ini` / `ui.ini` / `plugin.ini` の認識と計数。
//!
//! MAME の INI は空白区切りの `key value` 行(`=` なし、`#` コメント)で、
//! `rompath`/`hashpath`/`samplepath`/`artpath`/`ctrlrpath`/`inipath`/`fontpath`/
//! `cheatpath`/`crosshairpath`/`pluginspath`/`languagepath`/`swpath`/`cfg_directory`/
//! `nvram_directory`/`input_directory`/`state_directory`/`snapshot_directory`/
//! `diff_directory`/`comment_directory`/`video`/`numscreens`/`window`/`maximize`/
//! `waitvsync`/`syncrefresh`/`monitorprovider`/`prescale`/`filter`/`unevenstretch`/
//! `keepaspect`/`rotate`/`resolution`/`view`/`switchres`/`autosize`/`refreshspeed`/
//! `screen`/`aspect`/`brightness`/`contrast`/`gamma`/`volume`/`sound`/`samplerate`/
//! `mouse`/`joystick`/`lightgun`/`cheat`/`skip_gameinfo`/`uifont`/`plugin`/
//! `language`/`verbose`/`oslog`/`debug` 等の既知キーで構成される。
//!
//! ```
//! let b = b"# mame.ini\nrompath                  roms\nsamplepath               samples\nartpath                  artwork\nvideo                    opengl\nwindow                   1\nkeepaspect               1\nfilter                   1\nvolume                   0\ncheat                    1\n";
//! assert!(izanagi_kit::mameconf::detect(b));
//! let c = izanagi_kit::mameconf::parse(b).unwrap();
//! assert_eq!(c.entries, 9);
//! assert_eq!(c.known_entries, 9);
//! assert_eq!(c.path_entries, 3); // rompath/samplepath/artpath
//! assert_eq!(c.bool_entries, 5); // window/keepaspect/filter/volume/cheat
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key value` 行の総数。
    pub entries: usize,
    /// 既知 MAME オプション名の行数。
    pub known_entries: usize,
    /// `*path`/`*_directory` パス系キーの行数。
    pub path_entries: usize,
    /// 値が `0`/`1` の行数。
    pub bool_entries: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

const KNOWN: &[&str] = &[
    "rompath",
    "hashpath",
    "samplepath",
    "artpath",
    "ctrlrpath",
    "inipath",
    "fontpath",
    "cheatpath",
    "crosshairpath",
    "pluginspath",
    "languagepath",
    "swpath",
    "historypath",
    "cabinets_path",
    "cpanel_path",
    "marquees_path",
    "flyers_path",
    "titles_path",
    "snaps_path",
    "bosses_path",
    "logo_path",
    "pcbs_path",
    "howto_path",
    "select_path",
    "icons_path",
    "scores_path",
    "versuss_path",
    "gameover_path",
    "ends_path",
    "warning_path",
    "cfg_directory",
    "nvram_directory",
    "input_directory",
    "state_directory",
    "snapshot_directory",
    "diff_directory",
    "comment_directory",
    "share_directory",
    "autofire_directory",
    "video",
    "numscreens",
    "window",
    "maximize",
    "keepaspect",
    "waitvsync",
    "syncrefresh",
    "monitorprovider",
    "prescale",
    "filter",
    "unevenstretch",
    "rotate",
    "ror",
    "rol",
    "autoror",
    "autorol",
    "flipx",
    "flipy",
    "resolution",
    "view",
    "switchres",
    "changeres",
    "autosize",
    "refreshspeed",
    "screen",
    "aspect",
    "brightness",
    "contrast",
    "gamma",
    "pause_brightness",
    "effect",
    "volume",
    "sound",
    "samplerate",
    "audio_latency",
    "coin_lockout",
    "mouse",
    "joystick",
    "lightgun",
    "multikeyboard",
    "multimouse",
    "steadykey",
    "ui_active",
    "offscreen_reload",
    "joystick_map",
    "joystick_deadzone",
    "joystick_saturation",
    "natural",
    "ui_key",
    "ui_text",
    "ui_bg",
    "ui_gfx",
    "inifolder",
    "iniarea",
    "plugin",
    "noplugin",
    "language",
    "console",
    "debugger",
    "log",
    "oslog",
    "verbose",
    "update_in_pause",
    "debug",
    "debugscript",
    "autofireprojectile",
    "cheat",
    "skip_gameinfo",
    "uifont",
    "ui",
    "ramsize",
    "confirm_quit",
    "ui_mouse",
    "provider",
    "bilinear",
    "beam_width_min",
    "beam_width_max",
    "beam_intensity_weight",
    "vector_beam_smooth",
    "vector_flicker",
    "vector_length_scale",
    "vector_length_ratio",
    "vector_beam",
    "hlslpath",
    "overhead",
    "backdrop",
    "overlay",
    "bezel",
    "artwork_crop",
    "fallback_artwork",
    "override_artwork",
    "bgfx_path",
    "bgfx_backend",
    "bgfx_debug",
    "bgfx_screen_chains",
    "bgfx_shadow_mask",
    "lut",
    "resolutionx",
    "resolutiony",
    "colors",
    "frameskip",
    "autoframeskip",
    "throttle",
    "sleep",
    "speed",
    "refresh",
    "scanlines",
    "antialias",
    "translucency",
    "beam",
    "flicker",
    "seconds_to_run",
    "pause",
    "exit",
    "bios",
    "cheatpath",
];

fn key_of(s: &str) -> Option<(&str, &str)> {
    let mut it = s.splitn(2, [' ', '\t']);
    let k = it.next()?;
    let v = it.next().unwrap_or("").trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
    {
        return None;
    }
    Some((k, v))
}

/// `mame.ini` らしさを返す。`=` なしの既知 `key value` 行 ≥3。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines()
        .filter(|l| {
            let s = l.trim();
            !s.contains('=') && key_of(s).is_some_and(|(k, v)| KNOWN.contains(&k) && !v.is_empty())
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
        known_entries: 0,
        path_entries: 0,
        bool_entries: 0,
        comments: 0,
    };
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.contains('=') {
            continue;
        }
        let Some((k, v)) = key_of(s) else {
            continue;
        };
        if v.is_empty() {
            continue;
        }
        c.entries += 1;
        if KNOWN.contains(&k) {
            c.known_entries += 1;
        }
        if k.ends_with("path") || k.ends_with("_directory") || k.ends_with("folder") {
            c.path_entries += 1;
        }
        if v == "0" || v == "1" {
            c.bool_entries += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"rompath   roms\ncheat     0\nvideo     bgfx\nwindow    1\nui        cabinet\n";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 5);
        assert_eq!(c.known_entries, 5);
        assert_eq!(c.path_entries, 1);
        assert_eq!(c.bool_entries, 2);
    }

    #[test]
    fn rejects_key_eq_value() {
        assert!(parse(b"rompath = roms\nvideo = opengl\n").is_none());
    }
}

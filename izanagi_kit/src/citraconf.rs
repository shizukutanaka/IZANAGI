//! `qt-config.ini` (Citra Emulator) 検出モジュール。
//!
//! Citra の設定は Qt INI 形式で、`[General]`/`[Core]`/`[System]`/
//! `[Camera]`/`[Data Storage]`/`[Web Service]`/`[Renderer]`/
//! `[Layout]`/`[Audio]`/`[Controls]`/`[UI]`/`[Shortcuts]`/
//! `[Multiplayer]`/`[Debugging]` セクションと `use_cpu_jit`/
//! `hw_renderer`/`hw_shader`/`shaders_accurate_mul`/
//! `upright_screen`/`filter_mode`/`pp_shader_name`/`tex_filter_name`/
//! `custom_textures`/`factor_3d`/`use_hw_shader`/`use_vsync_new`/
//! `resolution_factor`/`bg_red`/`bg_green`/`bg_blue`/`swap_area`/
//! `frame_limit`/`use_frame_limit`/`mic_input_type`/`audio_emulation`/
//! `volume`/`enable_dsp_lle`/`dump_audio`/`profile`/`loadNDS`/
//! `init_clock`/`init_time`/`region_value`/`plugin_loader`/
//! `allow_plugin_loader`/`nso_offset`/`custom_rtc`/`lle_modules`/
//! `screenshot`/`use_newcomer`/`use_artic_base_server` 等のキーで
//! 構成される。
//!
//! ```
//! let b = br#"[Core]
//! use_cpu_jit=true
//! hw_shader=false
//! [Renderer]
//! resolution_factor=1
//! use_vsync_new=true
//! filter_mode=true
//! "#;
//! let c = izanagi_kit::citraconf::parse(b);
//! assert!(izanagi_kit::citraconf::detect(b));
//! assert_eq!(c.sections, 2);
//! ```

const SECTIONS: &[&str] = &[
    "[Audio]",
    "[Camera]",
    "[Controls]",
    "[Core]",
    "[Data Storage]",
    "[Debugging]",
    "[General]",
    "[Layout]",
    "[Miscellaneous]",
    "[Multiplayer]",
    "[Renderer]",
    "[Shortcuts]",
    "[System]",
    "[UI]",
    "[Web Service]",
    "[WebService]",
];

const KEYS: &[&str] = &[
    "allow_plugin_loader",
    "aspect_ratio",
    "async_presentation",
    "audio_emulation",
    "bg_blue",
    "bg_green",
    "bg_red",
    "check_for_update_periodically",
    "city_name",
    "confirmClose",
    "country",
    "cpu_ticks",
    "custom_rtc",
    "custom_textures",
    "dump_audio",
    "enable_dsp_lle",
    "factor_3d",
    "filter_mode",
    "frame_limit",
    "game_list_columns",
    "geometry_hiding",
    "hw_renderer",
    "hw_shader",
    "init_clock",
    "init_time",
    "language",
    "layout_option",
    "lle_modules",
    "loadNDS",
    "mic_input_type",
    "mute_audio",
    "nso_offset",
    "output_type",
    "plugin_loader",
    "pp_shader_name",
    "preload_textures",
    "profile",
    "region_value",
    "resolution_factor",
    "screenshot",
    "shaders_accurate_mul",
    "single_window_mode",
    "spirv_shader_gen",
    "swap_area",
    "tex_filter_name",
    "theme",
    "upright_screen",
    "use_artic_base_server",
    "use_cpu_jit",
    "use_custom_textures",
    "use_frame_limit",
    "use_hw_shader",
    "use_newcomer",
    "use_online",
    "use_sample_rate",
    "use_vsync_new",
    "volume",
    "web_api_url",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が qt-config.ini に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if SECTIONS.contains(&tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 3 || secs >= 3
}

/// qt-config.ini の統計。
#[derive(Debug, Default, Clone)]
pub struct CitraConf {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を qt-config.ini として統計する。
pub fn parse(b: &[u8]) -> CitraConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = CitraConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
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
        let b = b"[Core]\nuse_cpu_jit=true\nhw_shader=false\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_sections() {
        let b = b"[Renderer]\n[Layout]\n[Audio]\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[Core]\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"use_cpu_jit=true\nhw_shader=false\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}

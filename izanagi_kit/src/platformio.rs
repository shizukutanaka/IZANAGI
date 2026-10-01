//! PlatformIO `platformio.ini` プロジェクト設定の認識と計数。
//!
//! `platformio.ini` は INI 形式で、`[platformio]` セクションに
//! `default_envs`/`project_dir`/`workspace_dir`/`lib_dir`/`data_dir` 等の
//! グローバル設定を置き、`[env:<name>]` セクションにボード別の
//! `platform`/`board`/`framework`/`board_build`/`build_flags`/`lib_deps`/
//! `upload_speed`/`monitor_speed`/`build_type`/`targets`/`check_tool`/
//! `debug_tool`/`test_filter` 等を書く。値は継続行(インデントされた
//! 追加行)でリストを綴ることも多い。
//!
//! ```
//! let b = b"[platformio]\ndefault_envs =\n    uno\n    esp32\n\n[env:uno]\nplatform = atmelavr\nboard = uno\nframework = arduino\n\n[env:esp32]\nplatform = espressif32\nboard = esp32dev\nframework = arduino, esp-idf\nbuild_flags =\n    -DVERSION=13\n    -Os\nlib_deps =\n    bblanchon/ArduinoJson@^7\n    256dpi/MQTT@^2.5\nmonitor_speed = 115200\n";
//! assert!(izanagi_kit::platformio::detect(b));
//! let c = izanagi_kit::platformio::parse(b).unwrap();
//! assert_eq!(c.assigns, 10);
//! assert_eq!(c.envs, 2);
//! assert_eq!(c.continuations, 6); // 継続行(= 含み行もこちら)
//! assert_eq!(c.board_keys, 6); // platform×2 + board×2 + framework×2
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key = value` 代入行の個数。
    pub assigns: usize,
    /// `[env:<name>]` セクションの個数。
    pub envs: usize,
    /// `[platformio]`/`[env]`/`[common]`/`[env:<x>:<y>]` 等その他の
    /// セクション見出しの個数。
    pub other_sections: usize,
    /// 継続行(値リストの追加分)の個数。
    pub continuations: usize,
    /// `platform`/`board`/`framework` 系ハード指定キーの個数。
    pub board_keys: usize,
    /// `build_flags`/`lib_deps`/`lib_ignore`/`lib_extra_dirs`/`upload_*`/
    /// `monitor_*`/`debug_*`/`check_*`/`test_*` 系オプションキーの個数。
    pub option_keys: usize,
    /// `;`/`#` コメント行の個数。
    pub comments: usize,
}

const BOARD_KEYS: &[&str] = &["platform", "board", "framework", "extends"];

const OPTION_KEYS: &[&str] = &[
    "build_flags",
    "build_unflags",
    "build_src_flags",
    "build_src_filter",
    "lib_deps",
    "lib_ignore",
    "lib_extra_dirs",
    "lib_ldf_mode",
    "lib_compat_mode",
    "lib_archive",
    "lib_symlink_dirs",
    "upload_speed",
    "upload_port",
    "upload_protocol",
    "upload_flags",
    "upload_resetmethod",
    "monitor_speed",
    "monitor_port",
    "monitor_filters",
    "monitor_rts",
    "monitor_dtr",
    "monitor_echo",
    "monitor_eol",
    "monitor_raw",
    "debug_tool",
    "debug_speed",
    "debug_init_break",
    "debug_build_flags",
    "debug_load_cmds",
    "debug_server",
    "debug_svd_path",
    "check_tool",
    "check_flags",
    "check_skip_packages",
    "check_src_filters",
    "check_patterns",
    "check_severity",
    "test_filter",
    "test_ignore",
    "test_port",
    "test_speed",
    "test_build_flags",
    "test_build_src",
    "test_transport",
    "test_framework",
    "targets",
    "board_build",
    "board_upload",
    "board_debug",
    "board_monitor",
    "board_check",
    "board_test",
    "board_lib_deps",
    "extra_scripts",
    "lib_deps_extra",
    "custom_name",
    "custom_version",
    "description",
    "default_envs",
    "project_dir",
    "workspace_dir",
    "lib_dir",
    "data_dir",
    "include_dir",
    "src_dir",
    "boards_dir",
    "test_dir",
    "core_dir",
    "framework_dir",
    "packages_dir",
    "globallib_dir",
    "build_cache_dir",
    "shared_dir",
];

/// `platformio.ini` らしさを返す。`[env:` セクションか `platform`/`board`/
/// `framework` の組合せで判定。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    let mut env_sec = false;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("[env:") || s.starts_with("[env]") || s.starts_with("[platformio]") {
            env_sec = true;
        }
        for key in [
            "platform",
            "board",
            "framework",
            "lib_deps",
            "build_flags",
            "upload_speed",
            "monitor_speed",
        ] {
            if s.starts_with(key) && (s[key.len()..].trim_start().starts_with('=')) {
                hits += 1;
            }
        }
    }
    env_sec && hits >= 2 || hits >= 3
}

/// 行が `key = value` 代入ならキー名を返す。
fn assign_key(s: &str) -> Option<&str> {
    let eq = s.find('=')?;
    let k = s[..eq].trim_end();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
    {
        return None;
    }
    Some(k)
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        assigns: 0,
        envs: 0,
        other_sections: 0,
        continuations: 0,
        board_keys: 0,
        option_keys: 0,
        comments: 0,
    };
    for l in t.lines() {
        if l.trim().is_empty() {
            continue;
        }
        let ind = l.len() - l.trim_start().len();
        let s = l.trim();
        if s.starts_with(';') || s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') {
            if s.starts_with("[env:") {
                c.envs += 1;
            } else {
                c.other_sections += 1;
            }
            continue;
        }
        if ind == 0 {
            if let Some(k) = assign_key(s) {
                c.assigns += 1;
                if BOARD_KEYS.contains(&k) {
                    c.board_keys += 1;
                } else if OPTION_KEYS.contains(&k)
                    || k.starts_with("board_")
                    || k.starts_with("upload_")
                    || k.starts_with("monitor_")
                    || k.starts_with("debug_")
                    || k.starts_with("check_")
                    || k.starts_with("test_")
                    || k.starts_with("custom_")
                {
                    c.option_keys += 1;
                }
                continue;
            }
        }
        if ind > 0 {
            c.continuations += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pio() {
        assert!(detect(
            b"[env:uno]\nplatform = atmelavr\nboard = uno\nframework = arduino\n"
        ));
        assert!(detect(
            b"[platformio]\ndefault_envs = a\n[env:a]\nplatform = espressif32\nboard = x\nlib_deps = y\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"[main]\nfoo = bar\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"[platformio]\ndefault_envs = uno\n\n[env:uno]\nplatform = atmelavr\nboard = uno\nframework = arduino\nbuild_flags =\n    -DA=1\n    -DB=2\nlib_deps = x\n";
        let c = parse(b).unwrap();
        assert_eq!(c.assigns, 6);
        assert_eq!(c.envs, 1);
        assert_eq!(c.other_sections, 1);
        assert_eq!(c.continuations, 2);
        assert_eq!(c.board_keys, 3);
        assert_eq!(c.option_keys, 3); // default_envs + build_flags + lib_deps
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"[env:a]\nplatform = x\nboard = y\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}

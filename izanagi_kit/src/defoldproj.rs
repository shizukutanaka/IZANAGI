//! Defold `game.project` の検出と構造カウント。
//!
//! `[project]`/`[bootstrap]`/`[render]` 等の既知セクション + `key = value`。
//!
//! ```
//! let c = izanagi_kit::defoldproj::parse(
//!     b"[project]\ntitle = MyGame\nversion = 1.0\n[bootstrap]\nmain_collection = /main/main.collectionc\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert!(izanagi_kit::defoldproj::detect(b"[project]\ntitle = x\n[display]\nwidth = 1\n"));
//! ```

/// 既知 Defold セクション。
const SECTIONS: &[&str] = &[
    "bootstrap",
    "collection_proxy",
    "collectionfactory",
    "compile",
    "compute",
    "display",
    "factory",
    "font",
    "graphics",
    "gui",
    "html5",
    "input",
    "ios",
    "label",
    "library",
    "liveupdate",
    "lua",
    "macos",
    "model",
    "native_extension",
    "network",
    "particle_fx",
    "physics",
    "profiler",
    "project",
    "render",
    "resource",
    "rig",
    "script",
    "shader",
    "sound",
    "sprite",
    "tilemap",
    "tracking",
    "windows",
    "android",
    "dynamodb",
    "firebase",
    "gpg",
    "iap",
    "push",
    "review",
    "share",
    "websocket",
];
/// `project` セクション等の既知キー。
const KEYS: &[&str] = &[
    "archive_location_prefix",
    "author",
    "bundle_resources",
    "channels",
    "custom_resources",
    "dependencies",
    "display_profiles",
    "exclude_dirs",
    "extra_packages",
    "field_blocks",
    "main_collection",
    "publisher",
    "splash_image",
    "title",
    "version",
    "write_launcher_ini",
    "app_icon",
    "appmanifest",
    "check_integrity",
    "config",
    "debug",
    "default_height",
    "default_width",
    "dynamic_texture",
    "engine",
    "entry",
    "exclude_bundles",
    "fonts",
    "frame_cap",
    "fullscreen",
    "gravity_x",
    "gravity_y",
    "height",
    "icon",
    "input_binding",
    "joypad_mapping",
    "launch_image",
    "local_server",
    "lua_debugging",
    "material",
    "max_count",
    "max_height",
    "max_width",
    "memory_size",
    "models",
    "open_url",
    "output",
    "profiles",
    "script",
    "shared_state",
    "show_fps",
    "simulated_location",
    "taskbar_icon",
    "texture_profiles",
    "track_asyn\u{63}",
    "url",
    "use_acceleration",
    "view",
    "visual_debug",
    "world_count",
    "width",
];

/// Defold 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[section]`。
    pub sections: usize,
    /// `key = value` 既知キー。
    pub options: usize,
    /// `dependencies`/`include_dirs` 系の複数値行。
    pub lists: usize,
    /// `#`/`;` コメント。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が Defold game.project かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut secs = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && SECTIONS.contains(&&t[1..t.len() - 1]) {
            secs += 1;
        }
    }
    secs >= 2
}

/// game.project の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        options: 0,
        lists: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') || t.starts_with("--") {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            if SECTIONS.contains(&&t[1..t.len() - 1]) {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        let key = t[..pos].trim();
        if key == "dependencies" || key.ends_with("dependencies") {
            c.lists += 1;
        } else if KEYS.contains(&key) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[project]\ntitle = MyGame\nversion = 1.0.0\ndependencies = https://github.com/defold/extension-x\npublisher = Me\n\n[bootstrap]\nmain_collection = /main/main.collectionc\n\n[display]\nwidth = 960\nheight = 640\n\n[input]\ngame_binding = /input/game.input_bindingc\n\n[render]\nclear_color_red = 0.1\n";

    #[test]
    fn defoldproj() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.options, 6);
        assert_eq!(c.lists, 1);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_defold() {
        assert!(!detect(b"[main]\nkey = v\n"));
        assert!(!detect(b"hello\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

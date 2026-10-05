//! LÖVE 2D `conf.lua`/`main.lua` の検出と構造カウント。
//!
//! `function love.conf(t)` と `t.<group>.<key>`/`t.<key>` 代入・
//! `function love.<cb>` コールバックを分類する。
//!
//! ```
//! let c = izanagi_kit::loveconf::parse(
//!     b"function love.conf(t)\n  t.identity = \"game\"\n  t.window.width = 800\n  t.modules.joystick = false\nend\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::loveconf::detect(b"function love.conf(t)\nend\n"));
//! ```

/// `t.<group>` の既知グループ。
const GROUPS: &[&str] = &[
    "accelerometerjoystick",
    "appendidentity",
    "archive",
    "audio",
    "console",
    "externalstorage",
    "gammacorrect",
    "highdpi",
    "identity",
    "modules",
    "release",
    "saveidentity",
    "screen",
    "version",
    "window",
    "world",
];
/// `t.window.*`/`t.audio.*`/`t.modules.*` 配下の既知キー代表。
const SUBKEYS: &[&str] = &[
    "aa",
    "audio",
    "borderless",
    "centered",
    "data",
    "depth",
    "display",
    "event",
    "filesystem",
    "font",
    "fullscreen",
    "graphics",
    "height",
    "image",
    "joystick",
    "keyboard",
    "math",
    "minheight",
    "minwidth",
    "mouse",
    "msaa",
    "physics",
    "resizable",
    "sound",
    "stencil",
    "system",
    "thread",
    "timer",
    "touch",
    "video",
    "vsyn\u{63}",
    "width",
    "window",
    "x",
    "y",
];
/// `love.*` コールバック。
const CALLBACKS: &[&str] = &[
    "audio",
    "conf",
    "directorydropped",
    "draw",
    "errhand",
    "errorhandler",
    "exited",
    "filedropped",
    "focus",
    "gamepadaxis",
    "gamepadpressed",
    "gamepadreleased",
    "joystickadded",
    "joystickaxis",
    "joystickhat",
    "joystickpressed",
    "joystickreleased",
    "joystickremoved",
    "keypressed",
    "keyreleased",
    "load",
    "lowmemory",
    "mousefocus",
    "mousemoved",
    "mousepressed",
    "mousereleased",
    "quit",
    "resize",
    "run",
    "textedited",
    "textinput",
    "threaderror",
    "touchmoved",
    "touchpressed",
    "touchreleased",
    "update",
    "visible",
    "wheelmoved",
];

/// LÖVE conf 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `function love.*` 宣言。
    pub functions: usize,
    /// `t.*` 代入。
    pub options: usize,
    /// `local`/`other = ` 等の一般代入。
    pub assignments: usize,
    /// `--` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が LÖVE conf/main かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    if text.contains("love.conf") {
        return true;
    }
    let mut cb = 0;
    for &w in CALLBACKS {
        if w != "conf" && text.contains("love.") && text.contains(w) {
            cb += 1;
        }
    }
    cb >= 3
}

/// conf.lua 相当の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        functions: 0,
        options: 0,
        assignments: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("--") {
            c.comments += 1;
            continue;
        }
        if t.starts_with("function love.") || t.contains("= function(") {
            c.functions += 1;
            continue;
        }
        if let Some(rest) = t.strip_prefix("t.") {
            let name = rest
                .split(|ch: char| ch == '.' || ch == '=' || ch.is_whitespace())
                .next()
                .unwrap_or("");
            let sub = rest
                .split('.')
                .nth(1)
                .and_then(|s| s.split(|ch: char| ch == '=' || ch.is_whitespace()).next());
            if GROUPS.contains(&name) || sub.is_some_and(|k| SUBKEYS.contains(&k)) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if t.contains('=') && !t.contains("==") {
            c.assignments += 1;
            continue;
        }
        if matches!(t, "end" | "else" | "then") || t.ends_with('{') {
            continue;
        }
        c.misc += 1;
    }
    (c.functions + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"-- love conf\nfunction love.conf(t)\n  t.identity = \"mygame\"\n  t.version = \"11.4\"\n  t.console = false\n  t.window.title = \"My Game\"\n  t.window.width = 800\n  t.window.height = 600\n  t.window.vsync = 1\n  t.modules.joystick = false\n  t.modules.physics = false\nend\n";

    #[test]
    fn loveconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.functions, 1);
        assert_eq!(c.options, 9);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_love() {
        assert!(!detect(b"function main()\nend\n"));
        assert!(!detect(b"key = value\n"));
    }
}

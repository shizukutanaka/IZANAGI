//! Ren'Py `.rpy` script parser.
//!
//! Line census: `label`, `define`, `screen`, `image`, `menu`, `jump`,
//! `call`, `init`/`init python`, `scene`, `show`, `transform`,
//! `style`, `python` blocks, dialogue lines (`char "..."`) and `#`
//! comments.
//!
//! ```
//! use izanagi_kit::rpy::Rpy;
//! let src = b"define e = Character('Eileen')\n\nlabel start:\n    scene bg\n    show eileen\n    e \"Hello\"\n    jump end\n\nlabel end:\n    return\n";
//! assert!(izanagi_kit::rpy::detect(src));
//! let r = Rpy::parse(src).unwrap();
//! assert_eq!(r.labels, 2);
//! assert_eq!(r.defines, 1);
//! assert_eq!(r.jumps, 1);
//! assert_eq!(r.dialogue_lines, 1);
//! ```

/// Parsed census of an `.rpy` script.
#[derive(Debug, Clone)]
pub struct Rpy {
    /// `label name:` blocks.
    pub labels: usize,
    /// `define name = expr` statements.
    pub defines: usize,
    /// `define ... = Character(...)` character definitions.
    pub characters: usize,
    /// `screen name(...)` blocks.
    pub screens: usize,
    /// `image name = ...` statements.
    pub images: usize,
    /// `menu:` blocks.
    pub menus: usize,
    /// `jump target` statements.
    pub jumps: usize,
    /// `call label` statements.
    pub calls: usize,
    /// `init`/`init python` blocks.
    pub init_blocks: usize,
    /// `python`/`$` inline python.
    pub python: usize,
    /// `scene`/`show`/`hide`/`with` display statements.
    pub display: usize,
    /// `transform`/`style`/`audio`/`voice`/`translate`/`testcase` blocks.
    pub misc_blocks: usize,
    /// `name "..."` dialogue lines.
    pub dialogue_lines: usize,
    /// `return`/`pause`/`stop`/`play`/`queue`/`nvl`/`window` statements.
    pub statements: usize,
    /// `#` comment lines.
    pub comments: usize,
}
fn code_has(t: &str, needle: &str) -> bool {
    // `#` コメント行内の言及は証拠にしない。
    t.lines()
        .any(|l| !l.trim_start().starts_with('#') && l.contains(needle))
}

/// Returns `true` when `b` looks like a Ren'Py script.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    code_has(t, "label ")
        && (code_has(t, "define ")
            || code_has(t, "screen ")
            || code_has(t, "init python")
            || code_has(t, "image ")
            || code_has(t, "Character("))
}

impl Rpy {
    /// Parses an `.rpy` file; `None` without `label`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let mut r = Self {
            labels: 0,
            defines: 0,
            characters: 0,
            screens: 0,
            images: 0,
            menus: 0,
            jumps: 0,
            calls: 0,
            init_blocks: 0,
            python: 0,
            display: 0,
            misc_blocks: 0,
            dialogue_lines: 0,
            statements: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                r.comments += 1;
                continue;
            }
            let kw = l
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(':');
            match kw {
                "label" => r.labels += 1,
                "define" => {
                    r.defines += 1;
                    if l.contains("Character(") {
                        r.characters += 1;
                    }
                }
                "screen" => r.screens += 1,
                "image" => r.images += 1,
                "menu" => r.menus += 1,
                "jump" => r.jumps += 1,
                "call" => r.calls += 1,
                "init" => r.init_blocks += 1,
                "python" | "$" => r.python += 1,
                "scene" | "show" | "hide" | "with" => r.display += 1,
                "transform" | "style" | "audio" | "voice" | "translate" | "testcase" => {
                    r.misc_blocks += 1
                }
                "return" | "pause" | "stop" | "play" | "queue" | "nvl" | "window" => {
                    r.statements += 1
                }
                _ => {
                    // dialogue: `name "text"` (single token + quoted string)
                    if l.len() > 3
                        && !l.starts_with('"')
                        && l.contains('"')
                        && l.split_whitespace().count() >= 2
                        && l.split_whitespace()
                            .nth(1)
                            .is_some_and(|w| w.starts_with('"'))
                    {
                        r.dialogue_lines += 1;
                    }
                }
            }
        }
        (r.labels > 0).then_some(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# label start:\n# define e = 1\n"));
    }

    #[test]
    fn detects_rpy() {
        assert!(detect(b"label start:\n    define x = 1\n"));
        assert!(!detect(b"label x\nnothing\n"));
    }

    #[test]
    fn counts_kinds() {
        let src = b"label a:\n    screen s():\n    menu:\n        \"opt\":\n            call b\n    scene x\n    show y\n    return\n";
        let r = Rpy::parse(src).unwrap();
        assert_eq!(r.labels, 1);
        assert_eq!(r.screens, 1);
        assert_eq!(r.menus, 1);
        assert_eq!(r.calls, 1);
        assert_eq!(r.display, 2);
        assert_eq!(r.statements, 1);
    }
}

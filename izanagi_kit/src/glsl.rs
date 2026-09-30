//! GLSL shader source: `#version NNN [es|core|compatibility]`
//! directive plus GLSL builtins/keywords (`uniform`, `layout`,
//! `gl_Position`, `gl_FragColor`, `void main(`, qualifiers).
//!
//! ```
//! let src = b"#version 330 core\n\
//!             layout(location = 0) in vec3 pos;\n\
//!             uniform mat4 mvp;\n\
//!             void main() { gl_Position = mvp * vec4(pos, 1.0); }\n";
//! let p = izanagi_kit::glsl::parse(src).unwrap();
//! assert_eq!(p.version, 330);
//! assert_eq!(p.profile, izanagi_kit::glsl::Profile::Core);
//! assert!(izanagi_kit::glsl::detect(src));
//! ```

/// Shading profile after `#version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Desktop OpenGL default (no profile word).
    Default,
    /// `core` profile.
    Core,
    /// `compatibility` profile.
    Compatibility,
    /// OpenGL ES (`es`).
    Es,
}

/// Census of a GLSL source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Glsl {
    /// `#version` number (e.g. 330, 450, 300 for ES).
    pub version: u16,
    /// Shading profile.
    pub profile: Profile,
    /// `#extension` directives.
    pub extensions: u32,
    /// `uniform` declarations.
    pub uniforms: u32,
    /// `in` declarations (vertex inputs / varyings).
    pub ins: u32,
    /// `out` declarations.
    pub outs: u32,
    /// `varying`/`attribute` legacy declarations.
    pub legacy_io: u32,
    /// `layout(` qualifiers.
    pub layouts: u32,
    /// `sampler*`/`image*` uniforms.
    pub samplers: u32,
    /// `gl_` builtin references.
    pub builtins: u32,
    /// `void main(` present.
    pub has_main: bool,
    /// `//` comment lines.
    pub comments: u32,
    /// Preprocessor directives other than version/extension.
    pub directives: u32,
    /// Lines seen.
    pub lines: u32,
}

fn at_word(b: &[u8], pat: &[u8]) -> bool {
    b.windows(pat.len()).any(|w| w == pat)
}

fn version_of(b: &[u8]) -> Option<(u16, Profile)> {
    let i = b.windows(8).position(|w| w == *b"#version")?;
    let mut n = 0u16;
    let mut j = i + 8;
    while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
        j += 1;
    }
    let mut digits = false;
    while j < b.len() && b[j].is_ascii_digit() {
        n = n.saturating_mul(10).saturating_add((b[j] - b'0') as u16);
        digits = true;
        j += 1;
    }
    if !digits {
        return None;
    }
    let rest_end = b[j..]
        .iter()
        .position(|c| *c == b'\n')
        .map(|e| j + e)
        .unwrap_or(b.len());
    let rest = &b[j..rest_end];
    let profile = if at_word(rest, b"es") {
        Profile::Es
    } else if at_word(rest, b"core") {
        Profile::Core
    } else if at_word(rest, b"compatibility") {
        Profile::Compatibility
    } else {
        Profile::Default
    };
    Some((n, profile))
}

/// `true` on a `#version` line plus a GLSL tell (`gl_` builtin,
/// `void main`, `uniform`, or `layout(`).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    version_of(b).is_some()
        && (at_word(b, b"gl_")
            || at_word(b, b"void main")
            || at_word(b, b"uniform")
            || at_word(b, b"layout"))
}

/// Census; `None` without a `#version` directive.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Glsl> {
    let (version, profile) = version_of(b)?;
    let mut g = Glsl {
        version,
        profile,
        extensions: 0,
        uniforms: 0,
        ins: 0,
        outs: 0,
        legacy_io: 0,
        layouts: 0,
        samplers: 0,
        builtins: 0,
        has_main: false,
        comments: 0,
        directives: 0,
        lines: 0,
    };
    for line in b.split(|c| *c == b'\n') {
        g.lines += 1;
        let s = line
            .iter()
            .position(|c| !c.is_ascii_whitespace())
            .map_or(&[][..], |i| &line[i..]);
        if s.starts_with(b"//") {
            g.comments += 1;
            continue;
        }
        if s.starts_with(b"#") {
            if s.starts_with(b"#extension") {
                g.extensions += 1;
            } else if !s.starts_with(b"#version") {
                g.directives += 1;
            }
        }
        if at_word(s, b"uniform") {
            g.uniforms += 1;
        }
        if at_word(s, b"layout") {
            g.layouts += 1;
        }
        if at_word(s, b"varying") || at_word(s, b"attribute") {
            g.legacy_io += 1;
        } else {
            // modern in/out — count on declaration-ish lines only
            if !s.starts_with(b"#") && !s.starts_with(b"//") {
                if starts_decl(s, b"in") || (at_word(s, b" in ") || at_word(s, b" in(")) {
                    g.ins += 1;
                }
                if starts_decl(s, b"out") || at_word(s, b" out ") {
                    g.outs += 1;
                }
            }
        }
        for pat in [&b"sampler"[..], b"image", b"subpassInput"] {
            if at_word(s, pat) {
                g.samplers += 1;
                break;
            }
        }
        if at_word(s, b"gl_") {
            g.builtins += 1;
        }
        if at_word(s, b"main") {
            g.has_main = true;
        }
    }
    Some(g)
}

fn starts_decl(s: &[u8], kw: &[u8]) -> bool {
    s.starts_with(kw) && s.get(kw.len()).map_or(true, |c| c.is_ascii_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERT: &[u8] = b"#version 330 core\n\
        // vertex shader\n\
        layout(location = 0) in vec3 pos;\n\
        uniform mat4 mvp;\n\
        out vec2 uv;\n\
        void main() { gl_Position = mvp * vec4(pos, 1.0); }\n";

    const FRAG_ES: &[u8] = b"#version 300 es\n\
        precision mediump float;\n\
        uniform sampler2D tex;\n\
        in vec2 uv;\n\
        out vec4 color;\n\
        void main() { color = texture(tex, uv); }\n";

    #[test]
    fn detect_works() {
        assert!(detect(VERT));
        assert!(detect(FRAG_ES));
        assert!(!detect(b"#version only, no shader body\n"));
        assert!(!detect(b"void main() {}"));
    }

    #[test]
    fn parses_vertex() {
        let p = parse(VERT).unwrap();
        assert_eq!(p.version, 330);
        assert_eq!(p.profile, Profile::Core);
        assert_eq!(p.uniforms, 1);
        assert_eq!(p.layouts, 1);
        assert_eq!(p.ins, 1);
        assert_eq!(p.outs, 1);
        assert_eq!(p.builtins, 1);
        assert!(p.has_main);
        assert_eq!(p.comments, 1);
    }

    #[test]
    fn parses_es_fragment() {
        let p = parse(FRAG_ES).unwrap();
        assert_eq!(p.version, 300);
        assert_eq!(p.profile, Profile::Es);
        assert_eq!(p.samplers, 1);
        assert_eq!(p.directives, 0); // precision line is not '#'
        assert!(p.ins >= 1);
        assert!(p.outs >= 1);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"#include <stdio.h>").is_none());
    }
}

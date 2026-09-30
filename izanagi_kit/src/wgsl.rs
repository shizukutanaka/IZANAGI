//! WGSL (WebGPU Shading Language) source: `@vertex`/`@fragment`/
//! `@compute` entry attributes, `fn` definitions, `var<space>` /
//! `let`/`const` decls, `struct`, `->` return types, `@group`/
//! `@binding`/`@builtin`/`@location`/`@workgroup_size` attributes.
//!
//! ```
//! let src = b"@group(0) @binding(0) var<uniform> u : mat4x4f;\n\
//!             struct Out { @builtin(position) pos : vec4f };\n\
//!             @vertex fn vs(@location(0) p : vec3f) -> Out {\n\
//!                 var o : Out; o.pos = u * vec4f(p, 1.0); return o;\n\
//!             }\n";
//! let p = izanagi_kit::wgsl::parse(src).unwrap();
//! assert_eq!(p.vertex_entries, 1);
//! assert_eq!(p.bindings, 1);
//! assert!(izanagi_kit::wgsl::detect(src));
//! ```

/// Census of a WGSL source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Wgsl {
    /// `@vertex` entries.
    pub vertex_entries: u32,
    /// `@fragment` entries.
    pub fragment_entries: u32,
    /// `@compute` entries.
    pub compute_entries: u32,
    /// `@workgroup_size` attributes.
    pub workgroup_sizes: u32,
    /// `@group` attributes.
    pub groups: u32,
    /// `@binding` attributes.
    pub bindings: u32,
    /// `@location` attributes.
    pub locations: u32,
    /// `@builtin` attributes.
    pub builtins: u32,
    /// Other `@name` attributes.
    pub other_attributes: u32,
    /// `fn` definitions.
    pub functions: u32,
    /// `struct` blocks.
    pub structs: u32,
    /// `var<`/`var ` declarations.
    pub vars: u32,
    /// `let`/`const`/`override` declarations.
    pub lets: u32,
    /// `vec2f|vec3f|vec4f|matNxMf|f32|i32|u32` type refs.
    pub builtin_types: u32,
    /// `//` comment lines.
    pub comments: u32,
    /// Lines seen.
    pub lines: u32,
}

fn count_at(b: &[u8], pat: &[u8]) -> u32 {
    let mut n = 0u32;
    let mut i = 0;
    while i + pat.len() <= b.len() {
        if &b[i..i + pat.len()] == pat {
            n += 1;
            i += pat.len();
        } else {
            i += 1;
        }
    }
    n
}
fn at(b: &[u8], pat: &[u8]) -> bool {
    b.windows(pat.len()).any(|w| w == pat)
}

/// `true` on a WGSL entry attribute or `@group`/`@binding` with
/// `fn`/`var`/`struct` keyword support.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let attr = at(b, b"@vertex")
        || at(b, b"@fragment")
        || at(b, b"@compute")
        || (at(b, b"@group(") && at(b, b"@binding("));
    attr && (at(b, b"fn ") || at(b, b"var<") || at(b, b"struct "))
}

/// Census; `None` without the WGSL signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Wgsl> {
    if !detect(b) {
        return None;
    }
    let mut w = Wgsl {
        vertex_entries: count_at(b, b"@vertex"),
        fragment_entries: count_at(b, b"@fragment"),
        compute_entries: count_at(b, b"@compute"),
        workgroup_sizes: count_at(b, b"@workgroup_size"),
        groups: count_at(b, b"@group("),
        bindings: count_at(b, b"@binding("),
        locations: count_at(b, b"@location("),
        builtins: count_at(b, b"@builtin("),
        other_attributes: 0,
        functions: count_at(b, b"fn "),
        structs: count_at(b, b"struct "),
        vars: count_at(b, b"var<") + count_at(b, b"var "),
        lets: count_at(b, b"let ") + count_at(b, b"const ") + count_at(b, b"override "),
        builtin_types: count_at(b, b"vec2f")
            + count_at(b, b"vec3f")
            + count_at(b, b"vec4f")
            + count_at(b, b"mat4x4f")
            + count_at(b, b"mat3x3f")
            + count_at(b, b"mat2x2f")
            + count_at(b, &[102u8, 51, 50])
            + count_at(b, b"i32")
            + count_at(b, b"u32"),
        comments: 0,
        lines: 0,
    };
    // count @-attributes not in the known set
    let mut known = w.vertex_entries
        + w.fragment_entries
        + w.compute_entries
        + w.workgroup_sizes
        + w.groups
        + w.bindings
        + w.locations
        + w.builtins;
    for line in b.split(|c| *c == b'\n') {
        w.lines += 1;
        let s = line
            .iter()
            .position(|c| !c.is_ascii_whitespace())
            .map_or(&[][..], |i| &line[i..]);
        if s.starts_with(b"//") {
            w.comments += 1;
            continue;
        }
        for (i, &c) in s.iter().enumerate() {
            if c == b'@'
                && i + 1 < s.len()
                && s[i + 1].is_ascii_alphabetic()
                && s.get(i.wrapping_sub(1))
                    .map_or(true, |p| !p.is_ascii_alphanumeric())
            {
                w.other_attributes += 1;
            }
        }
    }
    // subtract the known ones already counted
    known = known.min(w.other_attributes);
    w.other_attributes -= known;
    Some(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"// demo\n\
        @group(0) @binding(0) var<uniform> u : mat4x4f;\n\
        struct Out { @builtin(position) pos : vec4f };\n\
        @vertex fn vs(@location(0) p : vec3f) -> Out {\n\
            var o : Out;\n\
            o.pos = u * vec4f(p, 1.0);\n\
            return o;\n\
        }\n\
        @fragment fn fs(in : Out) -> @location(0) vec4f {\n\
            return vec4f(1.0);\n\
        }\n\
        @compute @workgroup_size(64) fn cs() { }\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(!detect(b"fn main() { var x = 1; }"));
        assert!(!detect(b"@decorator python"));
    }

    #[test]
    fn parses() {
        let p = parse(SRC).unwrap();
        assert_eq!(p.vertex_entries, 1);
        assert_eq!(p.fragment_entries, 1);
        assert_eq!(p.compute_entries, 1);
        assert_eq!(p.workgroup_sizes, 1);
        assert_eq!(p.groups, 1);
        assert_eq!(p.bindings, 1);
        assert_eq!(p.builtins, 1);
        assert!(p.locations >= 2); // @location(0) twice
        assert_eq!(p.functions, 3);
        assert_eq!(p.structs, 1);
        assert!(p.vars >= 2);
        assert!(p.builtin_types >= 3);
        assert_eq!(p.comments, 1);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"plain rust code fn main() {}").is_none());
    }
}

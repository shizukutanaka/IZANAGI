//! HLSL shader source (Direct3D): no magic — detected by
//! `register(tN`/`cbuffer`/`SV_` semantics plus `floatN`/`Texture2D`
//! types; census counts constant buffers, registers, semantics and
//! entry points.
//!
//! ```
//! let src = b"cbuffer CB : register(b0) { float4x4 mvp; }\n\
//!             Texture2D tex : register(t0);\n\
//!             float4 PS(float4 p : SV_Position) : SV_Target {\n\
//!                 return tex.Load(int3(0,0,0));\n\
//!             }\n";
//! let p = izanagi_kit::hlsl::parse(src).unwrap();
//! assert_eq!(p.cbuffers, 1);
//! assert_eq!(p.registers, 2);
//! assert!(izanagi_kit::hlsl::detect(src));
//! ```

/// Census of an HLSL source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Hlsl {
    /// `cbuffer`/`tbuffer` blocks.
    pub cbuffers: u32,
    /// `register(xN)`/`packoffset` bindings.
    pub registers: u32,
    /// `SV_*` system-value semantics.
    pub sv_semantics: u32,
    /// `: TEXCOORDN`/`: COLORN`/other `: NAME` semantics.
    pub user_semantics: u32,
    /// `TextureN`/`RWTexture`/`Buffer` declarations.
    pub resources: u32,
    /// `SamplerState`/`SamplerComparisonState`.
    pub samplers: u32,
    /// `[numthreads`/`[maxvertexcount]`/`[WaveSize]` attributes.
    pub attributes: u32,
    /// `floatN`/`intN`/`uintN`/`halfN`/`matrix`/`floatNxM` uses.
    pub vector_types: u32,
    /// `struct`/`typedef` declarations.
    pub structs: u32,
    /// `technique`/`pass` blocks (legacy fx).
    pub techniques: u32,
    /// Lines ending in `(`...`)` function-def shapes → entries.
    pub entries: u32,
    /// `//` comment lines.
    pub comments: u32,
    /// Lines seen.
    pub lines: u32,
}

fn at(b: &[u8], pat: &[u8]) -> bool {
    b.windows(pat.len()).any(|w| w == pat)
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

/// `true` when HLSL-specific constructs appear: `register(`,
/// `cbuffer`, `SV_` semantics, or `Texture2D`/`RWStructuredBuffer`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let sig = at(b, b"register(")
        || at(b, b"cbuffer")
        || at(b, b": SV_")
        || at(b, b"SV_Target")
        || at(b, b"RWStructuredBuffer")
        || (at(b, b"Texture2D") && at(b, b"sampler"));
    sig && at(b, b"float")
}

/// Census; `None` without the HLSL signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Hlsl> {
    if !detect(b) {
        return None;
    }
    let mut h = Hlsl {
        cbuffers: count_at(b, b"cbuffer") + count_at(b, b"tbuffer"),
        registers: count_at(b, b"register(") + count_at(b, b"packoffset("),
        sv_semantics: count_at(b, b"SV_"),
        user_semantics: count_at(b, b": TEXCOORD")
            + count_at(b, b": COLOR")
            + count_at(b, b": NORMAL")
            + count_at(b, b": POSITION"),
        resources: count_at(b, b"Texture2D")
            + count_at(b, b"Texture3D")
            + count_at(b, b"TextureCube")
            + count_at(b, b"Texture1D")
            + count_at(b, b"RWTexture")
            + count_at(b, b"RWStructuredBuffer")
            + count_at(b, b"StructuredBuffer")
            + count_at(b, b"ByteAddressBuffer")
            + count_at(b, b"Buffer<"),
        samplers: count_at(b, b"SamplerState") + count_at(b, b"SamplerComparisonState"),
        attributes: count_at(b, b"[numthreads")
            + count_at(b, b"[maxvertexcount")
            + count_at(b, b"[WaveSize")
            + count_at(b, b"[earlydepthstencil"),
        vector_types: count_at(b, b"float2")
            + count_at(b, b"float3")
            + count_at(b, b"float4")
            + count_at(b, b"int2")
            + count_at(b, b"int3")
            + count_at(b, b"int4")
            + count_at(b, b"uint2")
            + count_at(b, b"uint3")
            + count_at(b, b"uint4")
            + count_at(b, b"half")
            + count_at(b, b"matrix"),
        structs: count_at(b, b"struct ") + count_at(b, b"typedef "),
        techniques: count_at(b, b"technique") + count_at(b, b"pass "),
        entries: 0,
        comments: 0,
        lines: 0,
    };
    for line in b.split(|c| *c == b'\n') {
        h.lines += 1;
        let s = line
            .iter()
            .position(|c| !c.is_ascii_whitespace())
            .map_or(&[][..], |i| &line[i..]);
        if s.starts_with(b"//") {
            h.comments += 1;
            continue;
        }
        // crude entry-point shape: `type NAME(...) [: sem] {`
        if !s.starts_with(b"#")
            && at(s, b"(")
            && at(s, b")")
            && (at(s, b"{") || at(s, b":"))
            && !at(s, b";")
        {
            h.entries += 1;
        }
    }
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"// demo shader\n\
        cbuffer CB : register(b0) { float4x4 mvp; }\n\
        Texture2D tex : register(t0);\n\
        SamplerState samp : register(s0);\n\
        float4 PS(float4 p : SV_Position, float2 uv : TEXCOORD0) : SV_Target {\n\
            return tex.Sample(samp, uv);\n\
        }\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(!detect(b"float4 plain() { return 0; }"));
        assert!(!detect(b"cbuffer alone"));
    }

    #[test]
    fn parses() {
        let p = parse(SRC).unwrap();
        assert_eq!(p.cbuffers, 1);
        assert_eq!(p.registers, 3);
        assert_eq!(p.sv_semantics, 2); // SV_Position + SV_Target
        assert_eq!(p.user_semantics, 1); // TEXCOORD0
        assert_eq!(p.resources, 1);
        assert_eq!(p.samplers, 1);
        assert_eq!(p.entries, 1);
        assert_eq!(p.comments, 1);
        assert!(p.vector_types >= 3);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"int main() { return 0; }").is_none());
    }
}

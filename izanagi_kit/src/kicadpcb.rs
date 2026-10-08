//! KiCad PCB `.kicad_pcb` (S式) の検出・カウント。
//!
//! `(kicad_pcb (version …) …)` ルート。
//! `footprint`/`module` フットプリント、`pad` パッド、`segment`/`via`/`arc` 配線、
//! `zone` ゾーン、`net`/`net_class` ネット、`gr_*`/`dimension`/`target`/`fp_*` 図形を分類する。
//!
//! ```
//! let cfg = b"(kicad_pcb (version 20211014) (generator pcbnew)\n\
//!              (layers (0 \"F.Cu\" signal)) (net 0 \"\")\n\
//!              (footprint \"R_0603\" (pad \"1\" smd rect (at 0 0))))";
//! assert!(izanagi_kit::kicadpcb::detect(cfg));
//! let c = izanagi_kit::kicadpcb::parse(cfg).unwrap();
//! assert_eq!(c.footprints, 1);
//! ```

use crate::textutil::strip_bom;
/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// S式ヘッド名総数。
    pub entries: usize,
    /// `footprint`/`module` フットプリント数。
    pub footprints: usize,
    /// `pad` パッド数。
    pub pads: usize,
    /// `segment`/`via`/`arc`/`track` 配線数。
    pub tracks: usize,
    /// `zone`/`keepout` ゾーン数。
    pub zones: usize,
    /// `net`/`net_class` ネット数。
    pub nets: usize,
    /// `gr_*`/`fp_line`/`fp_circle`/`fp_arc`/`fp_text`/`fp_poly`/`fp_rect`/`dimension`/`target`/`stroke`/`fill`/`pts`/`xy`/`xyz`/`at`/`size`/`layers`/`layer` 図形・補助数。
    pub misc: usize,
}

/// `b` が `.kicad_pcb` かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let text = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let text = strip_bom(text);
    text.trim_start().starts_with("(kicad_pcb")
}

/// `b` を `.kicad_pcb` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    if !detect(text.as_bytes()) {
        return None;
    }
    let bytes = text.as_bytes();
    let mut c = Counts {
        entries: 0,
        footprints: 0,
        pads: 0,
        tracks: 0,
        zones: 0,
        nets: 0,
        misc: 0,
    };
    let mut i = 0usize;
    let mut first = true;
    while i < bytes.len() {
        if bytes[i] != b'(' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < bytes.len()
            && (bytes[j].is_ascii_alphanumeric() || matches!(bytes[j], b'_' | b'-' | b'.'))
        {
            j += 1;
        }
        i = j;
        if j == start {
            continue;
        }
        let name = &text[start..j];
        if first {
            first = false;
            continue;
        }
        c.entries += 1;
        if matches!(name, "footprint" | "module") {
            c.footprints += 1;
        } else if name == "pad" {
            c.pads += 1;
        } else if matches!(name, "segment" | "via" | "arc" | "track") {
            c.tracks += 1;
        } else if matches!(name, "zone" | "keepout") {
            c.zones += 1;
        } else if matches!(name, "net" | "net_class") {
            c.nets += 1;
        } else {
            c.misc += 1;
        }
    }
    if c.entries >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"(kicad_pcb (version 20211014) (generator pcbnew)\n\
        (general (thickness 1.6)) (paper \"A4\")\n\
        (layers (0 \"F.Cu\" signal) (31 \"B.Cu\" signal))\n\
        (net 0 \"\") (net 1 \"GND\") (net 2 \"VCC\")\n\
        (net_class \"Default\" (add_net \"GND\"))\n\
        (footprint \"Resistor_SMD:R_0603\" (layer \"F.Cu\") (tedit 0)\n\
            (at 100 100) (descr \"R\") (attr smd)\n\
            (fp_text reference \"R1\" (at 0 -1)) (fp_text value \"10k\" (at 0 1))\n\
            (fp_line (start -0.5 -0.25) (end 0.5 -0.25) (layer \"F.SilkS\") (width 0.12))\n\
            (fp_circle (center 0 0) (end 0.3 0)) (fp_arc (start 0 0) (mid 1 0) (end 0 1))\n\
            (fp_poly (pts (xy 0 0) (xy 1 0))) (fp_rect (start 0 0) (end 1 1))\n\
            (pad \"1\" smd rect (at -0.75 0) (size 0.8 0.9) (layers \"F.Cu\" \"F.Paste\" \"F.Mask\") (net 1))\n\
            (pad \"2\" smd rect (at 0.75 0) (size 0.8 0.9) (net 2)))\n\
        (footprint \"Capacitor_SMD:C_0603\" (pad \"1\" smd rect (at 0 0)))\n\
        (module \"old_mod\" (layer \"F.Cu\"))\n\
        (segment (start 0 0) (end 10 0) (width 0.25) (layer \"F.Cu\") (net 1))\n\
        (segment (start 10 0) (end 10 10) (width 0.25) (net 2))\n\
        (via (at 10 10) (size 0.8) (drill 0.4) (layers \"F.Cu\" \"B.Cu\") (net 1))\n\
        (arc (start 0 0) (mid 5 5) (end 10 0) (width 0.2) (layer \"F.Cu\"))\n\
        (zone (net 1) (layer \"F.Cu\") (tstamp 0) (hatch edge 0.5)\n\
            (connect_pads (clearance 0.2)) (polygon (pts (xy 0 0) (xy 10 0) (xy 10 10))))\n\
        (gr_line (start 0 0) (end 5 0) (layer \"Edge.Cuts\") (width 0.1))\n\
        (gr_rect (start 0 0) (end 5 5) (layer \"Edge.Cuts\"))\n\
        (gr_circle (center 0 0) (end 5 0) (layer \"Edge.Cuts\"))\n\
        (gr_arc (start 0 0) (mid 5 5) (end 10 0) (layer \"Edge.Cuts\"))\n\
        (gr_text \"pcb\" (at 0 0) (layer \"F.SilkS\"))\n\
        (gr_poly (pts (xy 0 0) (xy 1 0)) (layer \"Edge.Cuts\"))\n\
        (gr_curve (pts (xy 0 0) (xy 1 1)) (layer \"Edge.Cuts\"))\n\
        (gr_bbox (start 0 0) (end 1 1))\n\
        (dimension (type aligned) (layer \"Dwgs.User\") (tstamp 0)\n\
            (gr_text \"10mm\" (at 0 0)) (feature1 (pts (xy 0 0) (xy 1 0))))\n\
        (target (shape plus) (at 0 0) (size 1) (width 0.15) (layer \"Dwgs.User\"))\n";

    #[test]
    fn detects_kicad_pcb() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.footprints, 3);
        assert_eq!(c.pads, 3);
        assert_eq!(c.tracks, 4);
        assert_eq!(c.zones, 1);
        assert_eq!(c.nets, 10);
        assert!(c.misc >= 20);
    }

    #[test]
    fn rejects_other_sexp() {
        assert!(!detect(b"(kicad_sch (version 1))"));
        assert!(!detect(b"(foo bar)"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

//! KiCad 回路図 `.kicad_sch` (S式) の検出・カウント。
//!
//! `(kicad_sch (version …) (generator …) …)` ルート。
//! `symbol`/`power`/`hierarchical_sheet` シンボル、`wire`/`bus`/`bus_entry` 配線、
//! `label`/`global_label`/`hierarchical_label` ラベル、`junction`/`no_connect` 接続、
//! `polyline`/`text`/`image` 図形、`property`/`pin` 属性を分類する。
//!
//! ```
//! let cfg = b"(kicad_sch (version 20211123) (generator eeschema)\n\
//!              (paper \"A4\") (lib_symbols (symbol \"Device:R\"))\n\
//!              (junction (at 10 10)) (wire (pts (xy 0 0) (xy 10 0))))";
//! assert!(izanagi_kit::kicadsch::detect(cfg));
//! let c = izanagi_kit::kicadsch::parse(cfg).unwrap();
//! assert_eq!(c.wires, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// S式ヘッド名総数。
    pub entries: usize,
    /// `symbol`/`power`/`hierarchical_sheet`/`lib_symbol`/`lib_symbols` シンボル数。
    pub symbols: usize,
    /// `wire`/`bus`/`bus_entry` 配線数。
    pub wires: usize,
    /// `label`/`global_label`/`hierarchical_label` ラベル数。
    pub labels: usize,
    /// `junction`/`no_connect` 接点数。
    pub junctions: usize,
    /// `property`/`pin`/`uuid` 属性・ピン数。
    pub properties: usize,
    /// 図形・その他名前付き S式数。
    pub misc: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `b` が `.kicad_sch` かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let text = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let text = strip_bom(text);
    text.trim_start().starts_with("(kicad_sch")
}

/// `b` を `.kicad_sch` として解析する。
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
        symbols: 0,
        wires: 0,
        labels: 0,
        junctions: 0,
        properties: 0,
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
        if matches!(
            name,
            "symbol" | "power" | "hierarchical_sheet" | "lib_symbol" | "lib_symbols"
        ) {
            c.symbols += 1;
        } else if matches!(name, "wire" | "bus" | "bus_entry") {
            c.wires += 1;
        } else if matches!(name, "label" | "global_label" | "hierarchical_label") {
            c.labels += 1;
        } else if matches!(name, "junction" | "no_connect") {
            c.junctions += 1;
        } else if matches!(name, "property" | "pin" | "uuid") {
            c.properties += 1;
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

    const SAMPLE: &[u8] = b"(kicad_sch (version 20211123) (generator eeschema)\n\
        (paper \"A4\") (title_block (title \"t\"))\n\
        (lib_symbols (symbol \"Device:R\" (property \"Reference\" \"R\")))\n\
        (symbol (lib_id \"Device:R\") (at 100 100 0) (uuid \"a\")\n\
            (property \"Reference\" \"R1\") (property \"Value\" \"10k\")\n\
            (pin \"1\" (uuid \"p1\")) (pin \"2\" (uuid \"p2\")))\n\
        (power (at 50 50 0) (lib_id \"power:GND\"))\n\
        (hierarchical_sheet \"sub1\" (at 200 200 80 60))\n\
        (wire (pts (xy 0 0) (xy 10 0)) (stroke (width 0)))\n\
        (bus (pts (xy 0 20) (xy 10 20)))\n\
        (bus_entry (at 10 10) (size 2.54))\n\
        (junction (at 10 10)) (no_connect (at 30 30))\n\
        (label \"n1\" (at 10 0 0)) (global_label \"vcc\" (at 10 0 0))\n\
        (hierarchical_label \"sig\" (at 20 0 0))\n\
        (polyline (pts (xy 0 0) (xy 1 1)))\n\
        (text \"hi\" (at 0 0 0)) (image (at 0 0))\n";

    #[test]
    fn detects_kicad_sch() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.symbols, 5);
        assert_eq!(c.wires, 3);
        assert_eq!(c.labels, 3);
        assert_eq!(c.junctions, 2);
        assert_eq!(c.properties, 8);
        assert!(c.misc >= 7);
    }

    #[test]
    fn rejects_other_sexp() {
        assert!(!detect(b"(kicad_pcb (version 4))"));
        assert!(!detect(b"(foo bar)"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

//! LTspice 回路図 `.asc` の検出・カウント。
//!
//! 行指向コマンド: `SHEET`/`WIRE`/`FLAG`/`IOPIN`/`SYMBOL`/`SYMATTR`/`WINDOW`/
//! `TEXT`/`LINE`/`RECTANGLE`/`CIRCLE`/`ARC`/`NORMAL`/`ROTATED`。
//!
//! ```
//! let cfg = b"SHEET 1 880 680\nWIRE 240 160 240 128\nSYMBOL res 240 144 R0\n\
//!              SYMATTR InstName R1\nSYMATTR Value 10k\nFLAG 240 128 0\n";
//! assert!(izanagi_kit::ltsconf::detect(cfg));
//! let c = izanagi_kit::ltsconf::parse(cfg).unwrap();
//! assert_eq!(c.symbols, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// コマンド行総数。
    pub entries: usize,
    /// `WIRE` 行数。
    pub wires: usize,
    /// `SYMBOL` シンボル配置数。
    pub symbols: usize,
    /// `SYMATTR`/`WINDOW` 属性数。
    pub attrs: usize,
    /// `FLAG`/`IOPIN` フラグ・ピン数。
    pub flags: usize,
    /// `LINE`/`RECTANGLE`/`CIRCLE`/`ARC`/`NORMAL`/`ROTATED` 図形数。
    pub shapes: usize,
    /// `TEXT`/`SHEET` テキスト・シート数。
    pub texts: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が `.asc` かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 3)
}

/// `b` を `.asc` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        wires: 0,
        symbols: 0,
        attrs: 0,
        flags: 0,
        shapes: 0,
        texts: 0,
        misc: 0,
    };
    let mut sheet = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let head = line.split([' ', '\t']).next().unwrap_or("");
        match head {
            "SHEET" => {
                sheet = true;
                c.entries += 1;
                c.texts += 1;
            }
            "WIRE" => {
                c.entries += 1;
                c.wires += 1;
            }
            "SYMBOL" => {
                c.entries += 1;
                c.symbols += 1;
            }
            "SYMATTR" | "WINDOW" => {
                c.entries += 1;
                c.attrs += 1;
            }
            "FLAG" | "IOPIN" => {
                c.entries += 1;
                c.flags += 1;
            }
            "LINE" | "RECTANGLE" | "CIRCLE" | "ARC" | "NORMAL" | "ROTATED" => {
                c.entries += 1;
                c.shapes += 1;
            }
            "TEXT" => {
                c.entries += 1;
                c.texts += 1;
            }
            _ => c.misc += 1,
        }
    }
    if sheet && c.entries >= 3 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"Version 4\nSHEET 1 880 680\n\
        WIRE 240 160 240 128\nWIRE 240 256 240 288\n\
        FLAG 240 128 0\nFLAG 240 288 0\nIOPIN 240 96 In\nIOPIN 240 320 Out\n\
        SYMBOL res 240 144 R0\nSYMATTR InstName R1\nSYMATTR Value 10k\n\
        WINDOW 0 24 40 Left 2\nWINDOW 3 24 56 Left 2\n\
        SYMBOL voltage 112 144 R0\nSYMATTR InstName V1\n\
        TEXT 280 96 Left 2 ;.tran 1m\nTEXT 300 200 Left 2 !.options noopiter\n\
        RECTANGLE Normal 400 300 32 24\nLINE Normal 400 300 432 300\n\
        CIRCLE Normal 350 250 370 270\nARC Normal 100 100 200 200 150 150 180 120\n";

    #[test]
    fn detects_ltsconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.wires, 2);
        assert_eq!(c.symbols, 2);
        assert_eq!(c.attrs, 5);
        assert_eq!(c.flags, 4);
        assert_eq!(c.shapes, 4);
        assert_eq!(c.texts, 3);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"SYMBOL res 0 0 R0\nSYMATTR InstName R1\n"));
        assert!(!detect(b"just text\n"));
    }
}

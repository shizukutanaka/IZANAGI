//! gEDA gschem 回路図 `.sch` の検出・カウント。
//!
//! `v YYYYMMDD N` バージョン行の後、オブジェクト行が続く:
//! `C` 部品、`N`/`U` ネット/バス、`P` ピン、`B`/`V`/`A`/`H`/`M`/`T`/`G`/`F`/`L`
//! 図形、`{`〜`}` 属性ブロック (内部に `name=value`)。
//!
//! ```
//! let cfg = b"v 20221002 2\nC 40000 46100 1 0 0 resistor-1.sym\n\
//!             {\nT 40400 46400 5 10 1 1 0 0 1\nrefdes=R1\n}\n\
//!             N 41900 46900 41900 47500 4\n";
//! assert!(izanagi_kit::gedasch::detect(cfg));
//! let c = izanagi_kit::gedasch::parse(cfg).unwrap();
//! assert_eq!(c.components, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// オブジェクト行 + バージョン行総数。
    pub entries: usize,
    /// `C` 部品 (component) 数。
    pub components: usize,
    /// `N`/`U` ネット・バス数。
    pub nets: usize,
    /// `P` ピン数。
    pub pins: usize,
    /// `B`/`V`/`A`/`H`/`M`/`T`/`G`/`F`/`L`/`O` 図形・テキスト数。
    pub graphics: usize,
    /// `{`/`}` 属性ブロック行 + ブロック内 `name=value` 行数。
    pub attribs: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が gEDA `.sch` かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.components + c.nets + c.graphics >= 2)
}

/// `b` を gEDA `.sch` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        components: 0,
        nets: 0,
        pins: 0,
        graphics: 0,
        attribs: 0,
        misc: 0,
    };
    let mut version_seen = false;
    let mut in_attrib = false;
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }
        if in_attrib {
            if line.trim_start().starts_with('}') {
                c.entries += 1;
                in_attrib = false;
            }
            c.attribs += 1;
            continue;
        }
        if !version_seen {
            version_seen = true;
            let head = line.trim_start();
            if !(head.starts_with('v') && head.chars().nth(1) == Some(' ')) {
                return None;
            }
            c.entries += 1;
            continue;
        }
        let head = line.trim_start();
        let code = head.chars().next()?;
        c.entries += 1;
        match code {
            'C' => c.components += 1,
            'N' | 'U' => c.nets += 1,
            'P' => c.pins += 1,
            'B' | 'V' | 'A' | 'H' | 'M' | 'T' | 'G' | 'F' | 'L' | 'O' | 'E' => {
                c.graphics += 1;
            }
            '{' => {
                c.attribs += 1;
                in_attrib = true;
            }
            _ => c.misc += 1,
        }
    }
    if c.entries >= 3 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"v 20221002 2\n\
        C 40000 46100 1 0 0 resistor-1.sym\n\
        {\nT 40400 46400 5 10 1 1 0 0 1\nrefdes=R1\n}\n\
        {\nT 40400 46100 5 10 0 0 0 0 1\nvalue=10k\n}\n\
        C 42000 45000 1 90 0 capacitor-1.sym\n\
        N 41900 46900 41900 47500 4\n\
        N 41900 47500 42300 47500 4\n\
        U 42300 47500 42300 47900 4\n\
        P 42600 47500 42600 47100 1 0 0\n\
        B 40500 44000 2000 1000 3 0 0 0 -1 -1 0 -1 -1 -1 -1 -1\n\
        V 41000 44500 150 3 0 0 0 -1 -1 0 -1 -1 -1 -1 -1\n\
        A 41500 45000 100 0 180 3 0 0 0 -1 -1\n\
        T 40000 48000 8 10 1 1 0 0 1\n\
        Title text\n\
        M 43000 44000 2000 1000 0 0 logo.png\n\
        G 42500 43500 43000 43500\n";

    #[test]
    fn detects_gedasch() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.components, 2);
        assert_eq!(c.nets, 3);
        assert_eq!(c.pins, 1);
        assert_eq!(c.graphics, 7);
        assert_eq!(c.attribs, 8);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"C 1 2 3 4 5 6 7\nN 1 2 3 4 5\n"));
        assert!(!detect(b"random\ntext\n"));
    }
}

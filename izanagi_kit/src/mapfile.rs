//! MapServer `.map` マップファイルの検出・カウント。
//!
//! `MAP`/`WEB`/`LAYER`/`CLASS`/`STYLE`/`LABEL`/`PROJECTION`/`END` 等の
//! ブロックキーワードと `KEY value` ステートメントからなる大文字構文。
//!
//! ```
//! let map = b"MAP\n NAME \"m\"\n WEB\n  METADATA\n  END\n END\n LAYER\n  NAME \"l\"\n  TYPE POLYGON\n  DATA \"x\"\n END\nEND\n";
//! assert!(izanagi_kit::mapfile::detect(map));
//! let c = izanagi_kit::mapfile::parse(map).unwrap();
//! assert_eq!(c.blocks, 4);
//! ```

/// ブロックキーワード (`END` 対応)。
const BLOCKS: &[&str] = &[
    "MAP",
    "WEB",
    "LAYER",
    "CLASS",
    "CLASSGROUP",
    "STYLE",
    "LABEL",
    "PROJECTION",
    "SCALEBAR",
    "LEGEND",
    "QUERYMAP",
    "REFERENCE",
    "OUTPUTFORMAT",
    "SYMBOL",
    "METADATA",
    "VALIDATION",
    "JOIN",
    "GRID",
    "CLUSTER",
    "FEATURE",
    "POINTS",
    "PATTERN",
    "COMPOSITE",
    "LEADER",
];

/// 既知ステートメントキーワード。
const KEYWORDS: &[&str] = &[
    "NAME",
    "TYPE",
    "STATUS",
    "DATA",
    "TEMPLATE",
    "TOLERANCE",
    "TOLERANCEUNITS",
    "COLOR",
    "OUTLINECOLOR",
    "SIZE",
    "ANGLE",
    "POSITION",
    "OFFSET",
    "WIDTH",
    "MINWIDTH",
    "MAXWIDTH",
    "MINSIZE",
    "MAXSIZE",
    "OPACITY",
    "TRANSPARENT",
    "TRANSFORM",
    "UNITS",
    "EXTENT",
    "MINSCALEDENOM",
    "MAXSCALEDENOM",
    "MINSCALE",
    "MAXSCALE",
    "SYMBOLSCALEDENOM",
    "LABELITEM",
    "LABELSIZEITEM",
    "LABELANGLEITEM",
    "CLASSITEM",
    "EXPRESSION",
    "FILTER",
    "FILTERITEM",
    "CONNECTION",
    "CONNECTIONTYPE",
    "PROCESSING",
    "PROJECTIONTEXT",
    "TEXT",
    "FONT",
    "FONTSIZE",
    "FONTSET",
    "IMAGECOLOR",
    "IMAGEMODE",
    "IMAGETYPE",
    "IMAGEPATH",
    "IMAGEURL",
    "LOG",
    "ERROR",
    "EMPTY",
    "FOOTER",
    "HEADER",
    "POSTLABELCACHE",
    "GROUP",
    "REQUIRES",
    "LABELREQUIRES",
    "DEBUG",
    "SHAPEPATH",
    "FONTSETFILE",
    "SYMBOLSET",
    "ENCRYPTED",
    "BUFFER",
    "GEOMTRANSFORM",
    "ALIGNTOP",
    "MASK",
    "MARKER",
    "GAP",
    "LINECAP",
    "LINEJOIN",
    "PATTERNFILE",
    "POINT",
    "RANGEITEM",
    "SCALEBACK",
    "SIZEX",
    "SIZEY",
    "STYLEITEM",
    "TITLE",
    "WRAP",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ステートメント・ブロック宣言・`END` の総数。
    pub entries: usize,
    /// `MAP`/`WEB`/`LAYER`/`CLASS`/`STYLE` 等ブロック開始数。
    pub blocks: usize,
    /// `END` 行数。
    pub ends: usize,
    /// `NAME`/`TYPE`/`COLOR`/`DATA` 等既知ステートメント数。
    pub keywords: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が MapServer mapfile かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.blocks >= 2 && c.ends >= 1 && c.blocks >= c.ends)
}

/// `b` を `.map` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        blocks: 0,
        ends: 0,
        keywords: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        c.entries += 1;
        let end = t.find([' ', '\t']).unwrap_or(t.len());
        let word = &t[..end];
        if BLOCKS.contains(&word) {
            c.blocks += 1;
        } else if word == "END" {
            c.ends += 1;
        } else if KEYWORDS.contains(&word) {
            c.keywords += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.blocks >= 1 && c.ends >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn mapfile() {
        let map = b"MAP\n NAME \"demo\"\n STATUS ON\n EXTENT 0 0 1 1\n IMAGECOLOR 255 255 255\n WEB\n  IMAGEPATH \"/tmp/\"\n  METADATA\n   \"wms_title\" \"x\"\n  END\n END\n PROJECTION\n  \"init=epsg:4326\"\n END\n LAYER\n  NAME \"roads\"\n  TYPE LINE\n  STATUS DEFAULT\n  DATA \"roads.shp\"\n  CLASS\n   NAME \"r\"\n   STYLE\n    COLOR 0 0 255\n   END\n  END\n END\nEND\n";
        let c = parse(map).unwrap();
        assert_eq!(c.blocks, 7);
        assert_eq!(c.ends, 7);
        assert_eq!(c.keywords, 11);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_mapfile() {
        assert!(parse(b"foo bar\nbaz\n").is_none());
    }
}

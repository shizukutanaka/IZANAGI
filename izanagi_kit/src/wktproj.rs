//! `.prj` / WKT (Well-Known Text) 座標系定義 検出モジュール。
//!
//! ESRI shapefile の `.prj` 等に使われる WKT CRS 定義は
//! `PROJCS["name",GEOGCS["name",DATUM[...]]]` や
//! `GEOGCS["name",DATUM["name",SPHEROID["name",6378137,298.257]]]`
//! の入れ子ブラケット構造で構成される。
//!
//! ```
//! let b = br#"PROJCS["WGS_1984_UTM_Zone_52N",
//!     GEOGCS["GCS_WGS_1984",
//!         DATUM["D_WGS_1984",
//!             SPHEROID["WGS_1984",6378137.0,298.257223563]],
//!         PRIMEM["Greenwich",0.0],
//!         UNIT["Degree",0.0174532925199433]],
//!     PROJECTION["Transverse_Mercator"],
//!     PARAMETER["False_Easting",500000.0],
//!     UNIT["Meter",1.0]]"#;
//! let c = izanagi_kit::wktproj::parse(b);
//! assert!(izanagi_kit::wktproj::detect(b));
//! assert!(c.keywords >= 6);
//! ```

const KEYWORDS: &[&str] = &[
    "ANGLEUNIT",
    "AREAUNIT",
    "AUTHORITY",
    "AXIS",
    "BASEGEOGCRS",
    "BOUNDCRS",
    "BOUNDGEOGCRS",
    "COMPD_CS",
    "CONVERSION",
    "CS",
    "DATUM",
    "DERIVINGCONVERSION",
    "EDATUM",
    "ELLIPSOID",
    "ENGINEERINGDATUM",
    "ENSEMBLE",
    "GEOGCS",
    "GEOGRAPHICCRS",
    "GEOIDMODEL",
    "GEOM",
    "GEOMGEOCRS",
    "ID",
    "INVERSE",
    "LENGTHUNIT",
    "MAPGRID",
    "METHOD",
    "PARAMETER",
    "PARAMETRFILE",
    "PRIMEM",
    "PROJCS",
    "PROJECTEDCRS",
    "PROJECTION",
    "PROJSTEP",
    "QUANTITY",
    "REMARK",
    "SCALEUNIT",
    "SOURCECRS",
    "SPHEROID",
    "TARGETCRS",
    "TIMEUNIT",
    "TOWGS84",
    "UNIT",
    "VERTCS",
    "VERTCRS",
    "VERT_DATUM",
];

fn kw_count(t: &str) -> usize {
    // `KEYWORD[` パターンを数える。
    KEYWORDS
        .iter()
        .filter(|k| t.contains(&format!("{k}[")))
        .count()
}

/// `b` が WKT CRS 定義に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let n = kw_count(t);
    let anchored = t.trim_start().starts_with("PROJCS[")
        || t.trim_start().starts_with("GEOGCS[")
        || t.trim_start().starts_with("PROJCRS[")
        || t.trim_start().starts_with("GEODCRS[")
        || t.trim_start().starts_with("GEOGCRS[")
        || t.trim_start().starts_with("GEODCRS[")
        || t.trim_start().starts_with("VERTCRS[")
        || t.trim_start().starts_with("BOUNDCRS[");
    (anchored && n >= 3) || n >= 5
}

/// WKT CRS 定義の統計。
#[derive(Debug, Default, Clone)]
pub struct WktProj {
    /// 既知 WKT キーワード出現数(種類数)。
    pub keywords: usize,
    /// 開きブラケット数。
    pub brackets: usize,
}

/// `b` を WKT CRS 定義として統計する。
pub fn parse(b: &[u8]) -> WktProj {
    let t = std::str::from_utf8(b).unwrap_or("");
    WktProj {
        keywords: kw_count(t),
        brackets: t.matches('[').count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"GEOGCS["GCS_WGS_1984",
    DATUM["D_WGS_1984",
        SPHEROID["WGS_1984",6378137.0,298.257223563]],
    PRIMEM["Greenwich",0.0],
    UNIT["Degree",0.0174532925199433]]"#;
        assert!(detect(b));
        let c = parse(b);
        assert!(c.keywords >= 4);
    }

    #[test]
    fn detects_vert() {
        let b = br#"VERTCS["NAVD88",
    VERT_DATUM["North American Vertical Datum 1988",2005],
    UNIT["metre",1.0]]"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"PROJCS[\"x\"]"));
        assert!(!detect(b"POINT[1,2]\nLINESTRING[3,4]"));
        assert!(!detect(b"DATUM[x]\nSPHEROID[y]\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keywords, 0);
    }
}

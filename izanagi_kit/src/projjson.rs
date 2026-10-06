//! PROJJSON (OGC GeoCRS JSON) 検出モジュール。
//!
//! PROJJSON は WKT2 の JSON 形式で、トップレベル
//! `"type"` が `"GeographicCRS"`/`"ProjectedCRS"`/`"VerticalCRS"`/
//! `"GeodeticReferenceFrame"`/`"DynamicGeodeticReferenceFrame"`/
//! `"VerticalReferenceFrame"`/`"BoundCRS"`/`"DerivedGeographicCRS"`
//! 等で、`"name"`/`"datum"`/`"coordinate_system"`/`"conversion"`/
//! `"ellipsoid"`/`"id"` 等のキーを持つ。
//!
//! ```
//! let b = br#"{
//!   "type": "GeographicCRS",
//!   "name": "WGS 84",
//!   "datum": {
//!     "type": "GeodeticReferenceFrame",
//!     "name": "World Geodetic System 1984",
//!     "ellipsoid": {"name": "WGS 84", "semi_major_axis": 6378137}
//!   },
//!   "coordinate_system": {"subtype": "ellipsoidal"}
//! }
//! "#;
//! let c = izanagi_kit::projjson::parse(b);
//! assert!(izanagi_kit::projjson::detect(b));
//! assert_eq!(c.crs_keys, 2);
//! ```

const TYPES: &[&str] = &[
    "GeographicCRS",
    "GeodeticCRS",
    "ProjectedCRS",
    "DerivedGeographicCRS",
    "DerivedGeodeticCRS",
    "DerivedProjectedCRS",
    "DerivedVerticalCRS",
    "VerticalCRS",
    "BoundCRS",
    "TemporalCRS",
    "EngineeringCRS",
    "ParametricCRS",
    "CoordinateMetadata",
    "GeodeticReferenceFrame",
    "DynamicGeodeticReferenceFrame",
    "VerticalReferenceFrame",
    "DynamicVerticalReferenceFrame",
    "TemporalReferenceFrame",
    "SingleOperation",
    "ConcatenatedOperation",
];

/// `"type": "Value"` の Value をすべて返す。
fn type_values(t: &str) -> impl Iterator<Item = &str> + '_ {
    t.match_indices("\"type\"").filter_map(|(i, _)| {
        let rest = &t[i + 6..];
        let rest = rest.trim_start().strip_prefix(':')?.trim_start();
        let rest = rest.strip_prefix('"')?;
        let e = rest.find('"')?;
        Some(&rest[..e])
    })
}

/// `b` が PROJJSON に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    type_values(t).any(|v| TYPES.contains(&v))
}

/// PROJJSON の統計。
#[derive(Debug, Default, Clone)]
pub struct ProjJson {
    /// `"type": "*CRS"` 等の既知型行数。
    pub crs_keys: usize,
}

/// `b` を PROJJSON として統計する。
pub fn parse(b: &[u8]) -> ProjJson {
    let t = std::str::from_utf8(b).unwrap_or("");
    ProjJson {
        crs_keys: type_values(t).filter(|v| TYPES.contains(v)).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"{
  "type": "GeographicCRS",
  "name": "WGS 84"
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.crs_keys, 1);
    }

    #[test]
    fn detects_bound() {
        let b = br#"{"type": "BoundCRS"}"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\"type\": \"FeatureCollection\"}"));
        assert!(!detect(b"{\"type\": \"Topology\"}"));
        assert!(!detect(b"type: GeographicCRS\nname: x\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.crs_keys, 0);
    }
}

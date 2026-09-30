//! Esri `.prj` — a WKT1/WKT2 coordinate-reference-system definition.
//!
//! ```
//! let d = b"PROJCS[\"Web_Mercator\",GEOGCS[\"WGS 84\"],PROJECTION[\"Mercator_Auxiliary_Sphere\"],UNIT[\"metre\",1],AUTHORITY[\"EPSG\",\"3857\"]]";
//! let p = izanagi_kit::prj::parse(d).unwrap();
//! assert_eq!(p.name, "Web_Mercator");
//! assert_eq!(p.epsg, Some(3857));
//! ```

/// Parsed `.prj` / WKT CRS text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prj {
    /// Root keyword: `PROJCS`, `GEOGCS`, `PROJCRS`, `GEODCRS`, …, verbatim.
    pub kind: String,
    /// Quoted name right after the root keyword.
    pub name: String,
    /// `AUTHORITY["EPSG","…"]` code, first occurrence.
    pub epsg: Option<u32>,
    /// Number of `PARAMETER[` entries.
    pub params: usize,
    /// Number of `AXIS[` entries.
    pub axes: usize,
    /// `UNIT[…]` names in order (e.g. `metre`, `degree`).
    pub units: Vec<String>,
}

const ROOTS: [&str; 6] = [
    "PROJCS", "GEOGCS", "PROJCRS", "GEODCRS", "GEOGCRS", "VERTCRS",
];

/// First quoted string after a `NAME[` keyword.
fn quoted<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let pos = s.find(key)?;
    let rest = &s[pos + key.len()..];
    let open = rest.find('"')? + 1;
    let close = rest[open..].find('"')? + open;
    Some(&rest[open..close])
}

/// Parse a `.prj` file; `None` without a recognised root keyword.
pub fn parse(d: &[u8]) -> Option<Prj> {
    let s = std::str::from_utf8(d).ok()?.trim();
    let kind = ROOTS
        .iter()
        .find(|r| s.starts_with(*r) && s[r.len()..].trim_start().starts_with('['))?;
    let name = quoted(s, kind)?.to_string();
    let epsg = s.find("AUTHORITY[\"EPSG\",\"").and_then(|i| {
        let rest = &s[i + 18..]; // after AUTHORITY["EPSG","
        let close = rest.find('"')?;
        rest[..close].parse().ok()
    });
    let params = s.matches("PARAMETER[").count();
    let axes = s.matches("AXIS[").count();
    let units = s
        .match_indices("UNIT[")
        .filter_map(|(i, _)| quoted(&s[i..], "UNIT").map(str::to_string))
        .collect();
    Some(Prj {
        kind: kind.to_string(),
        name,
        epsg,
        params,
        axes,
        units,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgs84_geogcs() {
        let d = b"GEOGCS[\"GCS_WGS_1984\",DATUM[\"D_WGS_1984\",SPHEROID[\"WGS_1984\",6378137,298.257]],UNIT[\"degree\",0.017],AXIS[\"Lon\",EAST],AXIS[\"Lat\",NORTH],AUTHORITY[\"EPSG\",\"4326\"]]";
        let p = parse(d).unwrap();
        assert_eq!(p.kind, "GEOGCS");
        assert_eq!(p.name, "GCS_WGS_1984");
        assert_eq!(p.epsg, Some(4326));
        assert_eq!(p.axes, 2);
        assert_eq!(p.units, vec!["degree"]);
    }

    #[test]
    fn projcrs_wkt2() {
        let d = b"PROJCRS[\"ETRS89 / UTM zone 32N\",BASEGEOGCRS[\"x\"],PARAMETER[\"k\",0],UNIT[\"metre\",1]]";
        let p = parse(d).unwrap();
        assert_eq!(p.kind, "PROJCRS");
        assert_eq!(p.params, 1);
        assert_eq!(p.epsg, None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"LOCAL_CS[\"x\"]").is_none()); // unsupported root
        assert!(parse(b"PROJCS[\"no close").is_none()); // unterminated quote
    }
}

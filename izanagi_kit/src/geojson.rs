//! Minimal reader for GeoJSON (RFC 7946): a JSON object whose `type` is
//! `Feature`/`FeatureCollection`/a geometry name. Built on [`crate::json`];
//! coordinates are kept as raw `Json` arrays (integer subset).
//!
//! ```
//! use izanagi_kit::geojson::parse;
//!
//! let g = parse(
//!     br#"{"type":"Feature","geometry":{"type":"Point","coordinates":[139,35]},
//!         "properties":{"name":"tokyo"}}"#,
//! )
//! .unwrap();
//! assert_eq!(g.kind.as_deref(), Some("Feature"));
//! assert_eq!(g.features.len(), 1);
//! assert_eq!(g.features[0].geometry_kind.as_deref(), Some("Point"));
//! ```

use crate::json::{parse as parse_json, Json};

/// One `Feature` (also used for a bare geometry at the root).
#[derive(Debug)]
pub struct Feature {
    /// `geometry.type` (`Point`, `LineString`, `Polygon`, ...).
    pub geometry_kind: Option<String>,
    /// `geometry.coordinates` verbatim `Json` (array nesting varies).
    pub coordinates: Option<Json>,
    /// `properties` verbatim `Json`.
    pub properties: Option<Json>,
}

/// A parsed GeoJSON document.
#[derive(Debug)]
pub struct GeoJson {
    /// Top-level `type` (`Feature`, `FeatureCollection`, geometry, ...).
    pub kind: Option<String>,
    /// `FeatureCollection.features`, or a single synthesized entry for a
    /// bare `Feature` / geometry root.
    pub features: Vec<Feature>,
    /// `bbox` verbatim when present.
    pub bbox: Option<Json>,
}

fn obj(j: &Json) -> Option<&std::collections::BTreeMap<String, Json>> {
    match j {
        Json::Obj(m) => Some(m),
        _ => None,
    }
}

fn get<'a>(j: &'a Json, k: &str) -> Option<&'a Json> {
    obj(j)?.get(k)
}

fn s(j: &Json) -> Option<&str> {
    match j {
        Json::Str(v) => Some(v),
        _ => None,
    }
}

fn feature(j: &Json) -> Option<Feature> {
    let m = obj(j)?;
    if m.get("type").and_then(s) != Some("Feature") {
        return None;
    }
    let geom = m.get("geometry");
    let (geometry_kind, coordinates) = match geom {
        Some(g) => (
            get(g, "type").and_then(s).map(str::to_string),
            get(g, "coordinates").cloned(),
        ),
        None => (None, None),
    };
    Some(Feature {
        geometry_kind,
        coordinates,
        properties: m.get("properties").cloned(),
    })
}

fn geometry(j: &Json) -> Option<Feature> {
    let kind = get(j, "type").and_then(s)?;
    if !matches!(
        kind,
        "Point"
            | "LineString"
            | "Polygon"
            | "MultiPoint"
            | "MultiLineString"
            | "MultiPolygon"
            | "GeometryCollection"
    ) {
        return None;
    }
    Some(Feature {
        geometry_kind: Some(kind.to_string()),
        coordinates: get(j, "coordinates").cloned(),
        properties: None,
    })
}

/// Parse a GeoJSON document. `None` on invalid JSON or a non-object root
/// without a GeoJSON `type`.
pub fn parse(data: &[u8]) -> Option<GeoJson> {
    let j = parse_json(data).ok()?;
    let m = obj(&j)?;
    let kind = m.get("type").and_then(s).map(str::to_string);
    let bbox = m.get("bbox").cloned();
    match kind.as_deref() {
        Some("FeatureCollection") => {
            let mut features = Vec::new();
            if let Some(Json::Arr(feats)) = m.get("features") {
                for f in feats {
                    features.push(feature(f)?);
                }
            }
            Some(GeoJson {
                kind,
                features,
                bbox,
            })
        }
        Some("Feature") => Some(GeoJson {
            kind,
            features: vec![feature(&j)?],
            bbox,
        }),
        _ => Some(GeoJson {
            kind: kind.clone(),
            features: geometry(&j).into_iter().collect(),
            bbox,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature() {
        let g = parse(
            br#"{"type":"Feature","geometry":{"type":"Point","coordinates":[139,35]},
                "properties":{"name":"x"},"bbox":[139,35,140,36]}"#,
        )
        .unwrap();
        let f = &g.features[0];
        assert_eq!(f.geometry_kind.as_deref(), Some("Point"));
        assert!(matches!(f.coordinates, Some(Json::Arr(_))));
        assert!(f.properties.is_some());
        assert!(g.bbox.is_some());
    }

    #[test]
    fn collection_and_geometry() {
        let c = parse(
            br#"{"type":"FeatureCollection","features":[
                {"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":null}]}"#,
        )
        .unwrap();
        assert_eq!(c.features.len(), 1);
        let g = parse(br#"{"type":"LineString","coordinates":[[0,0],[1,1]]}"#).unwrap();
        assert_eq!(g.features.len(), 1);
        assert_eq!(g.features[0].geometry_kind.as_deref(), Some("LineString"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[]").is_none()); // array root — not an object
                                         // Feature whose member isn't a Feature
        assert!(parse(
            br#"{"type":"FeatureCollection","features":[{"type":"Point","coordinates":[0,0]}]}"#
        )
        .is_none());
    }
}

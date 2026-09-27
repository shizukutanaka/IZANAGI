//! Minimal reader for TopoJSON: a JSON object with `"type":"Topology"`,
//! `objects` (each `GeometryCollection`/typed geometry), `arcs` (arc
//! position arrays), and an optional `transform{scale:[sx,sy],
//! translate:[tx,ty]}` — all values kept as raw `Json`.
//! Built on [`crate::json`].
//!
//! ```
//! use izanagi_kit::topojson::parse;
//!
//! let t = parse(
//!     br#"{"type":"Topology","objects":{"a":{"type":"Point","coordinates":[1,2]}},
//!          "arcs":[[[0,0],[1,1]]],"transform":{"scale":[1,1],"translate":[0,0]}}"#,
//! )
//! .unwrap();
//! assert_eq!(t.object_names, vec!["a"]);
//! assert_eq!(t.arc_count, 1);
//! assert!(t.transformed);
//! ```

use crate::json::{parse as parse_json, Json};
use std::collections::BTreeMap;

/// A parsed TopoJSON topology.
#[derive(Debug)]
pub struct TopoJson {
    /// Names of `objects` members, in map order.
    pub object_names: Vec<String>,
    /// `objects` verbatim `Json` for each name.
    pub objects: Vec<(String, Json)>,
    /// Number of `arcs` entries.
    pub arc_count: usize,
    /// `transform` present (delta-coded arcs then apply).
    pub transformed: bool,
    /// `transform.scale` verbatim.
    pub scale: Option<Json>,
    /// `transform.translate` verbatim.
    pub translate: Option<Json>,
    /// `bbox` verbatim when present.
    pub bbox: Option<Json>,
}

fn obj(j: &Json) -> Option<&BTreeMap<String, Json>> {
    match j {
        Json::Obj(m) => Some(m),
        _ => None,
    }
}

/// Parse a TopoJSON topology. `None` on invalid JSON, a non-object root,
/// or `type` other than `Topology`.
pub fn parse(data: &[u8]) -> Option<TopoJson> {
    let j = parse_json(data).ok()?;
    let m = obj(&j)?;
    match m.get("type") {
        Some(Json::Str(t)) if t == "Topology" => {}
        _ => return None,
    }
    let mut object_names = Vec::new();
    let mut objects = Vec::new();
    if let Some(Json::Obj(objs)) = m.get("objects") {
        for (k, v) in objs {
            object_names.push(k.clone());
            objects.push((k.clone(), v.clone()));
        }
    }
    let arc_count = match m.get("arcs") {
        Some(Json::Arr(a)) => a.len(),
        _ => 0,
    };
    let (transformed, scale, translate) = match m.get("transform").and_then(obj) {
        Some(t) => (true, t.get("scale").cloned(), t.get("translate").cloned()),
        None => (false, None, None),
    };
    Some(TopoJson {
        object_names,
        objects,
        arc_count,
        transformed,
        scale,
        translate,
        bbox: m.get("bbox").cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let t = parse(
            br#"{"type":"Topology","objects":{"coast":{"type":"LineString","arcs":[0]}},
                "arcs":[[[0,0],[5,0]]],"bbox":[0,0,5,5],
                "transform":{"scale":[1,1],"translate":[2,3]}}"#,
        )
        .unwrap();
        assert_eq!(t.object_names, vec!["coast"]);
        assert_eq!(t.arc_count, 1);
        assert!(t.transformed);
        assert!(t.scale.is_some());
        assert!(t.bbox.is_some());
    }

    #[test]
    fn no_transform() {
        let t = parse(br#"{"type":"Topology","objects":{},"arcs":[]}"#).unwrap();
        assert!(!t.transformed);
        assert_eq!(t.arc_count, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[]").is_none());
        assert!(parse(br#"{"type":"Feature"}"#).is_none());
        assert!(parse(br#"{"objects":{}}"#).is_none()); // no type
    }
}

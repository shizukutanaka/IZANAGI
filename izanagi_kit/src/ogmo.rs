//! Ogmo Editor 3 level JSON — `ogmoVersion`, `width`/`height`,
//! `offsetX`/`offsetY`, then a `layers` array whose elements carry
//! `name`, `data`, `entities`, `grid`, `tiles` or `decals`/`points`
//! depending on the layer kind.
//!
//! ```
//! let d = b"{\"ogmoVersion\":\"3\",\"width\":320,\"height\":240,\"layers\":[\
//! {\"name\":\"e\",\"entities\":[{\"x\":1}]},{\"name\":\"g\",\"grid\":[[0]]}]}";
//! let o = izanagi_kit::ogmo::parse(d).unwrap();
//! assert_eq!(o.width, Some(320));
//! assert_eq!(o.layers, 2);
//! assert_eq!(o.entity_layers, 1);
//! assert_eq!(o.grid_layers, 1);
//! assert!(izanagi_kit::ogmo::detect(d));
//! ```

/// Census of an Ogmo Editor level file.
#[derive(Debug, Clone)]
pub struct Ogmo {
    /// `width` from the header.
    pub width: Option<u32>,
    /// `height` from the header.
    pub height: Option<u32>,
    /// Elements in `layers` (counted via `"name"` keys after `layers`).
    pub layers: usize,
    /// Layers carrying `entities`.
    pub entity_layers: usize,
    /// Layers carrying `grid` cell data.
    pub grid_layers: usize,
    /// Layers carrying `tiles`.
    pub tile_layers: usize,
    /// Layers carrying `decals`.
    pub decal_layers: usize,
    /// Layers carrying `points`.
    pub point_layers: usize,
    /// `values` key occurrences (level/layer custom values).
    pub values: usize,
}

fn num_after(line: &str, key: &str) -> Option<u32> {
    let at = line.find(key)? + key.len();
    let rest = line.get(at..)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

/// Detects Ogmo JSON: `"ogmoVersion"` plus a `layers` array.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"ogmoVersion\"") && t.contains("\"layers\"")
}

/// Parses an Ogmo level; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ogmo> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let layers_at = t.find("\"layers\"")?;
    let body = &t[layers_at..];
    let o = Ogmo {
        width: num_after(t, "\"width\":"),
        height: num_after(t, "\"height\":"),
        layers: body.matches("\"name\"").count(),
        entity_layers: body.matches("\"entities\"").count(),
        grid_layers: body.matches("\"grid\"").count(),
        tile_layers: body.matches("\"tiles\"").count(),
        decal_layers: body.matches("\"decals\"").count(),
        point_layers: body.matches("\"points\"").count(),
        values: body.matches("\"values\"").count(),
    };
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"{\"ogmoVersion\":\"3\",\"width\":320,\"height\":240,\"offsetX\":0,\"offsetY\":0,\"layers\":[{\"name\":\"e\",\"entities\":[{\"x\":1,\"values\":{}}],\"gridCellWidth\":16},{\"name\":\"t\",\"tiles\":[{\"x\":0}],\"gridCellWidth\":16},{\"name\":\"g\",\"grid\":[[0,1]],\"gridCellWidth\":16}]}";

    #[test]
    fn parses() {
        let o = parse(D).unwrap();
        assert_eq!(o.width, Some(320));
        assert_eq!(o.height, Some(240));
        assert_eq!(o.layers, 3);
        assert_eq!(o.entity_layers, 1);
        assert_eq!(o.tile_layers, 1);
        assert_eq!(o.grid_layers, 1);
        assert_eq!(o.values, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"{\"layers\":[]}"));
        assert!(!detect(b"plain"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
    }
}

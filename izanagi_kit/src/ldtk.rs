//! LDtk (Level Designer Toolkit) `.ldtk` JSON — a `__header` block
//! (`fileType`/`app`/`appVersion`), then levels with `layerInstances`,
//! `entityInstances`, `tileInstances` and `intGridValues`, in either a
//! `levels` array or `worlds[]` multi-world layout.
//!
//! ```
//! let d = b"{\"__header\":{\"fileType\":\"LDtk\",\"app\":\"LDtk\",\"appVersion\":\"1\"},\"worlds\":[],\"levels\":[{\"layerInstances\":[{\"entityInstances\":[],\"tileInstances\":[],\"intGridValues\":[],\"gridSize\":8}]}]}";
//! let l = izanagi_kit::ldtk::parse(d).unwrap();
//! assert_eq!(l.levels, 1);
//! assert_eq!(l.grid_size, Some(8));
//! assert!(izanagi_kit::ldtk::detect(d));
//! ```

/// Census of an `.ldtk` level file.
#[derive(Debug, Clone)]
pub struct Ldtk {
    /// `__header` block present.
    pub header: bool,
    /// `worlds` array present (multi-world layout).
    pub worlds: usize,
    /// Levels counted via `layerInstances` keys.
    pub levels: usize,
    /// `layerInstances` occurrences.
    pub layer_instances: usize,
    /// `entityInstances` occurrences.
    pub entity_instances: usize,
    /// `tileInstances` occurrences.
    pub tile_instances: usize,
    /// `intGridValues` occurrences.
    pub int_grid: usize,
    /// `autoLayerTiles` occurrences.
    pub auto_tiles: usize,
    /// First `gridSize` value.
    pub grid_size: Option<u32>,
}

fn count_key(t: &str, key: &str) -> usize {
    t.matches(key).count()
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

/// Detects `.ldtk`: `"fileType":"LDtk"` or a `__header` naming the app.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"LDtk\"") || t.contains("\"LDtk ")) && t.contains("__header")
}

/// Parses an `.ldtk`; `None` on non-UTF-8 or missing LDtk markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ldtk> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let layer = count_key(t, "\"layerInstances\"");
    Some(Ldtk {
        header: t.contains("__header"),
        worlds: count_key(t, "\"worlds\""),
        levels: layer,
        layer_instances: layer,
        entity_instances: count_key(t, "\"entityInstances\""),
        tile_instances: count_key(t, "\"tileInstances\""),
        int_grid: count_key(t, "\"intGridValues\""),
        auto_tiles: count_key(t, "\"autoLayerTiles\""),
        grid_size: num_after(t, "\"gridSize\":"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"{\"__header\":{\"fileType\":\"LDtk\",\"app\":\"LDtk\",\"appVersion\":\"1\"},\"worlds\":[],\"levels\":[{\"iid\":\"a\",\"layerInstances\":[{\"entityInstances\":[{\"x\":1}],\"tileInstances\":[{\"t\":2}],\"intGridValues\":[],\"autoLayerTiles\":[],\"gridSize\":8},{\"entityInstances\":[],\"tileInstances\":[],\"intGridValues\":[{\"v\":1}],\"gridSize\":8}]},{\"layerInstances\":[]}]";

    #[test]
    fn parses() {
        let l = parse(D).unwrap();
        assert!(l.header);
        assert_eq!(l.worlds, 1);
        assert_eq!(l.levels, 2);
        assert_eq!(l.entity_instances, 2);
        assert_eq!(l.tile_instances, 2);
        assert_eq!(l.int_grid, 2);
        assert_eq!(l.auto_tiles, 1);
        assert_eq!(l.grid_size, Some(8));
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"{\"worlds\":[]}"));
        assert!(!detect(b"plain"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
    }
}

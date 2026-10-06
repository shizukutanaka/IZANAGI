//! `.mml` (TileMill/Mapbox Studio classic プロジェクト) 検出モジュール。
//!
//! TileMill プロジェクトの `.mml` は JSON で、`"Stylesheet"`/
//! `"Layer"`/`"interactivity"`/`"srs"`/`"Datasource"`/`"bounds"`/
//! `"center"`/`"minzoom"`/`"maxzoom"`/`"format"`/`"attribution"`/
//! `"template"` 等のキーを持つ。
//!
//! ```
//! let b = br#"{
//!     "srs": "+proj=merc +a=6378137 +b=6378137 +lat_ts=0.0",
//!     "Stylesheet": ["style.mss"],
//!     "Layer": [
//!         {"id": "world", "Datasource": {"type": "shape"}}
//!     ],
//!     "interactivity": {"layer": "world"},
//!     "format": "png"
//! }
//! "#;
//! let c = izanagi_kit::mmlstyle::parse(b);
//! assert!(izanagi_kit::mmlstyle::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "attribution",
    "bounds",
    "center",
    "Datasource",
    "description",
    "format",
    "interactivity",
    "Layer",
    "legend",
    "maxzoom",
    "metatile",
    "minzoom",
    "paths",
    "properties",
    "srs",
    "Stylesheet",
    "template",
    "tilemill",
];

const HINTS: &[&str] = &[
    "Stylesheet",
    "Layer",
    "interactivity",
    "Datasource",
    "srs",
    "tilemill",
];

/// 行内の最初の `"key":` を返す。
fn jkey(line: &str) -> Option<&str> {
    let s = line.trim_start();
    let s = s.strip_prefix('"')?;
    let e = s.find('"')?;
    let key = &s[..e];
    if s[e + 1..].trim_start().starts_with(':') {
        Some(key)
    } else {
        None
    }
}

/// `b` が TileMill .mml に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut hints = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                keys += 1;
            }
            if HINTS.contains(&k) {
                hints += 1;
            }
        }
    }
    (hints >= 1 && keys >= 2) || keys >= 4
}

/// TileMill .mml の統計。
#[derive(Debug, Default, Clone)]
pub struct MmlStyle {
    /// 既知キー行数。
    pub keys: usize,
    /// .mml 特有キー行数。
    pub hint_keys: usize,
}

/// `b` を TileMill .mml として統計する。
pub fn parse(b: &[u8]) -> MmlStyle {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = MmlStyle::default();
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                c.keys += 1;
            }
            if HINTS.contains(&k) {
                c.hint_keys += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"{
    "Stylesheet": ["style.mss"],
    "Layer": []
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn detects_srs() {
        let b = br#"{
    "srs": "+proj=merc",
    "format": "png",
    "bounds": [-180,-85,180,85]
}
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\"Stylesheet\": []}"));
        assert!(!detect(
            b"{\"name\": \"x\", \"version\": 1, \"bounds\": []}"
        ));
        assert!(!detect(b"Stylesheet=x\nLayer=y\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

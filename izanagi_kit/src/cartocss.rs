//! `.mss` / `.mml` CartoCSS 検出モジュール。
//!
//! CartoCSS (Mapbox/TileMill 地図スタイル) は CSS 派生形式で、
//! `#layer`/`Map`/`@var` セレクタと `line-color`/`polygon-fill`/
//! `marker-file`/`text-name`/`text-face-name`/`text-size`/
//! `polygon-gamma`/`line-join`/`building-fill`/`raster-opacity`
//! 等の地図プロパティで構成される。
//!
//! ```
//! let b = br#"#world {
//!   polygon-fill: #eee;
//!   line-color: #ccc;
//!   line-width: 0.5;
//! }
//! Map {
//!   background-color: #b8dee6;
//! }
//! "#;
//! let c = izanagi_kit::cartocss::parse(b);
//! assert!(izanagi_kit::cartocss::detect(b));
//! assert_eq!(c.properties, 4);
//! ```

const PROPS: &[&str] = &[
    "background-color",
    "buffer-size",
    "building-fill",
    "building-fill-opacity",
    "character-spacing",
    "comp-op",
    "composite",
    "face-name",
    "file",
    "fontset-name",
    "horizontal-alignment",
    "image-filters",
    "line-cap",
    "line-color",
    "line-dasharray",
    "line-gamma",
    "line-join",
    "line-opacity",
    "line-pattern-file",
    "line-simplify",
    "line-simplify-algorithm",
    "line-smooth",
    "line-width",
    "marker-allow-overlap",
    "marker-comp-op",
    "marker-file",
    "marker-fill",
    "marker-fill-opacity",
    "marker-height",
    "marker-line-color",
    "marker-line-opacity",
    "marker-line-width",
    "marker-multi-policy",
    "marker-opacity",
    "marker-placement",
    "marker-spacing",
    "marker-type",
    "marker-width",
    "opacity",
    "pattern-alignment",
    "pattern-file",
    "point-file",
    "point-opacity",
    "point-transform",
    "polygon-fill",
    "polygon-gamma",
    "polygon-opacity",
    "polygon-pattern-file",
    "polygon-smooth",
    "raster-colorizer-default-color",
    "raster-comp-op",
    "raster-filter-factor",
    "raster-mesh-size",
    "raster-opacity",
    "raster-scaling",
    "shield-dx",
    "shield-dy",
    "shield-face-name",
    "shield-file",
    "shield-name",
    "shield-size",
    "text-allow-overlap",
    "text-comp-op",
    "text-dx",
    "text-dy",
    "text-face-name",
    "text-fill",
    "text-halo-fill",
    "text-halo-radius",
    "text-label-position-tolerance",
    "text-line-spacing",
    "text-max-char-angle-delta",
    "text-min-distance",
    "text-name",
    "text-opacity",
    "text-orientation",
    "text-placements",
    "text-size",
    "text-spacing",
    "text-transform",
    "text-wrap-width",
];

fn is_prop_line(t: &str) -> bool {
    let k = t.split(':').next().unwrap_or("").trim();
    PROPS.contains(&k) && t.contains(':')
}

/// `b` が CartoCSS に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut props = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with("//") || tr.starts_with("/*") {
            continue;
        }
        if is_prop_line(tr) {
            props += 1;
        }
    }
    props >= 2
}

/// CartoCSS の統計。
#[derive(Debug, Default, Clone)]
pub struct CartoCss {
    /// 既知地図プロパティ行数。
    pub properties: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を CartoCSS として統計する。
pub fn parse(b: &[u8]) -> CartoCss {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = CartoCss::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with("//") || tr.starts_with("/*") {
            c.comments += 1;
            continue;
        }
        if is_prop_line(tr) {
            c.properties += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"#world {
  polygon-fill: #eee;
  line-color: #ccc;
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.properties, 2);
    }

    #[test]
    fn detects_markers() {
        let b = br#".points {
  marker-file: url(icon.svg);
  marker-width: 12;
}
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"polygon-fill: #eee;\n"));
        assert!(!detect(b"color: red;\nbackground: blue;\nwidth: 10px;\n"));
        assert!(!detect(b"polygon-fill #eee\nline-color #ccc\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.properties, 0);
    }
}

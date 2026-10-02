//! Planetiler `config.yml` の検出・カウント。
//!
//! `sources`/`layers`/`args`/`archive`/`download`/`tilemap`/`output`
//! 等 Planetiler 固有トップレベルセクションを持つ YAML 設定。
//!
//! ```
//! let cfg = b"sources:\n  osm:\n    type: osm\n    url: x\nlayers:\n  archipelago: {}\nargs:\n  area: monaco\noutput:\n  type: mbtiles\n";
//! assert!(izanagi_kit::planetilerconf::detect(cfg));
//! let c = izanagi_kit::planetilerconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 4);
//! ```

/// トップレベルセクション。
const SECTIONS: &[&str] = &[
    "sources",
    "layers",
    "args",
    "archive",
    "download",
    "tilemap",
    "output",
    "schema",
    "monitors",
    "stats",
    "definition",
    "feature_post_process",
];

/// layers 配下の既知レイヤ名。
const LAYERS: &[&str] = &[
    "archipelago",
    "boundaries",
    "buildings",
    "landcover",
    "landuse",
    "natural_earth",
    "places",
    "transportation",
    "water",
    "aeroway",
    "barriers",
    "housenumbers",
    "mountain_peak",
    "park",
    "poi",
    "power",
    "roads",
    "water_polygons",
    "waterways",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// キー・リスト項目の総数。
    pub entries: usize,
    /// `sources`/`layers`/`args`/`archive`/`output` トップセクション数。
    pub sections: usize,
    /// `layers:` 配下のレイヤ名定義数。
    pub layers: usize,
    /// `sources:` 配下のソース名定義数。
    pub sources: usize,
    /// `args:`/`download:`/`tilemap:`/`output:` 配下のオプションキー数。
    pub args: usize,
    /// その他項目数。
    pub misc: usize,
}

/// `b` が Planetiler 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2)
}

/// `b` を `config.yml` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        sections: 0,
        layers: 0,
        sources: 0,
        args: 0,
        misc: 0,
    };
    let mut section = "";
    let mut sindent = 0usize;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        c.entries += 1;
        if indent == 0 {
            let end = t.find([':', ' ']).unwrap_or(t.len());
            let key = &t[..end];
            if SECTIONS.contains(&key) {
                c.sections += 1;
                section = key;
                sindent = indent;
            } else {
                c.misc += 1;
                section = "";
            }
            continue;
        }
        let body = t.strip_prefix('-').map_or(t, |r| r.trim_start());
        let name = body
            .find([':', ' '])
            .map_or(body, |i| &body[..i])
            .trim_matches('"')
            .trim_matches('\'');
        match section {
            "layers" if indent == sindent + 2 && !name.is_empty() => {
                if LAYERS.contains(&name) {
                    c.layers += 1;
                } else {
                    c.misc += 1;
                }
            }
            "sources" if indent == sindent + 2 && !name.is_empty() => {
                c.sources += 1;
            }
            "args" | "download" | "tilemap" | "output" | "archive"
                if indent == sindent + 2 && !name.is_empty() =>
            {
                c.args += 1;
            }
            _ => c.misc += 1,
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn planetiler() {
        let cfg = b"sources:\n  osm:\n    type: osm\n    url: https://x\n  natural_earth:\n    type: shapefile\nlayers:\n  archipelago: {}\n  boundaries: {}\n  water: {}\nargs:\n  area: monaco\n  download: true\n  languages: en\noutput:\n  type: mbtiles\n  path: out.mbtiles\ntilemap:\n  base_url: x\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.layers, 3);
        assert_eq!(c.sources, 2);
        assert!(c.args >= 5);
    }

    #[test]
    fn not_planetiler() {
        assert!(parse(b"foo: 1\n").is_none());
    }
}

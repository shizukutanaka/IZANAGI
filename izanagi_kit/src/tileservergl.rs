//! TileServer GL `config.json` の検出・カウント。
//!
//! `options`/`paths`/`styles`/`data`/`serve_rendered`/`serve_all_styles`
//! `formatQuality`/`maxSize`/`watermark` 等 TileServer GL 固有キーを持つ JSON。
//!
//! ```
//! let cfg = br#"{"options":{"paths":{"root":"","fonts":"f","sprites":"s","styles":"st","mbtiles":"m"}},"styles":{"a":{"style":"a.json","tilejson":{"type":"x"}}},"serve_rendered":true}"#;
//! assert!(izanagi_kit::tileservergl::detect(cfg));
//! let c = izanagi_kit::tileservergl::parse(cfg).unwrap();
//! assert!(c.entries >= 8);
//! ```

/// `options`/`paths` 系キー。
const OPTIONS: &[&str] = &[
    "options",
    "paths",
    "root",
    "fonts",
    "sprites",
    "styles",
    "mbtiles",
    "data",
    "fonts_dir",
    "sprites_dir",
    "styles_dir",
    "mbtiles_dir",
    "pbfAlias",
    "publicUrl",
];

/// サービス挙動系キー。
const SERVE: &[&str] = &[
    "serve_rendered",
    "serve_all_styles",
    "serve_static_maps",
    "serve_fonts",
    "serve_pbf",
    "serve_rendered_test",
    "allow_rendering",
    "domains",
    "formatQuality",
    "maxSize",
    "maxScaleFactor",
    "maxSizeHiDPI",
    "watermark",
    "verbosity",
    "frontPage",
    "startupPromise",
    "cli",
];

/// スタイル・タイル JSON 内のキー。
const STYLE: &[&str] = &[
    "style",
    "tilejson",
    "overlay",
    "wms",
    "wmts",
    "type",
    "bounds",
    "center",
    "format",
    "maxzoom",
    "minzoom",
    "name",
    "version",
    "attribution",
    "description",
    "scheme",
    "tiles",
    "grids",
    "data_tiles",
    "vector_layers",
    "id",
    "fields",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// JSON キートークン総数。
    pub entries: usize,
    /// `options`/`paths`/`fonts`/`sprites`/`mbtiles` 等パス設定系キー数。
    pub options: usize,
    /// `serve_*`/`formatQuality`/`watermark`/`maxSize` 等サービス挙動系キー数。
    pub serve: usize,
    /// `style`/`tilejson`/`overlay`/`vector_layers`/`bounds`/`center` 等スタイル系キー数。
    pub style: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が TileServer GL config かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.options + c.serve + c.style >= 4)
}

/// キートークンを走査 (`"name"` の直後が `:` のもの)。
fn keys(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            let mut esc = false;
            while j < bytes.len() {
                if esc {
                    esc = false;
                } else if bytes[j] == 0x5C {
                    esc = true;
                } else if bytes[j] == b'"' {
                    break;
                }
                j += 1;
            }
            let mut k = j + 1;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b':' && j > start {
                out.push(&text[start..j]);
            }
            i = k;
        } else {
            i += 1;
        }
    }
    out
}

/// `b` を `config.json` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    if !text.trim_start().starts_with('{') {
        return None;
    }
    let keys = keys(text);
    if keys.is_empty() {
        return None;
    }
    let mut c = Counts {
        entries: keys.len(),
        options: 0,
        serve: 0,
        style: 0,
        misc: 0,
    };
    for k in keys {
        if OPTIONS.contains(&k) {
            c.options += 1;
        } else if SERVE.contains(&k) || k.starts_with("serve_") {
            c.serve += 1;
        } else if STYLE.contains(&k) {
            c.style += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options + c.serve + c.style >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn tileserver() {
        let cfg = br#"{"options":{"paths":{"root":"","fonts":"f","sprites":"s","styles":"st","mbtiles":"m"}},"domains":["a"],"formatQuality":{"jpeg":80},"maxSize":2048,"watermark":"w","serve_rendered":true,"serve_all_styles":false,"serve_static_maps":true,"serve_fonts":true,"serve_pbf":false,"styles":{"a":{"style":"a.json","tilejson":{"type":"overlay","bounds":[0,0,1,1],"center":[0,0,5],"maxzoom":10,"name":"n"}}},"data":{"z":{"mbtiles":"z.mbtiles"}},"other":1}"#;
        let c = parse(cfg).unwrap();
        assert_eq!(c.entries, 30);
        assert_eq!(c.options, 10);
        assert_eq!(c.serve, 9);
        assert_eq!(c.style, 7);
        assert_eq!(c.misc, 4);
    }

    #[test]
    fn not_tileserver() {
        assert!(parse(br#"{"a":1}"#).is_none());
    }
}

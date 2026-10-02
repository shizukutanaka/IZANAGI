//! Mapnik XML スタイルシートの検出・カウント。
//!
//! `<Map>` ルートの下に `<Layer>`/`<Style>`/`<Rule>`/`<*Symbolizer>`
//! `<Datasource>` を持つ Mapnik スタイル XML。
//!
//! ```
//! let xml = br#"<Map><Style name="s"><Rule><LineSymbolizer/></Rule></Style>
//!               <Layer name="l"><StyleName>s</StyleName><Datasource><Parameter name="file">x</Parameter></Datasource></Layer></Map>"#;
//! assert!(izanagi_kit::mapnikxml::detect(xml));
//! let c = izanagi_kit::mapnikxml::parse(xml).unwrap();
//! assert_eq!(c.symbolizers, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 開始タグ総数。
    pub entries: usize,
    /// `<Layer>` レイヤ数。
    pub layers: usize,
    /// `<Style>` スタイル数。
    pub styles: usize,
    /// `<Rule>` ルール数 (`<MinScaleDenominator>` 等を含む)。
    pub rules: usize,
    /// `<*Symbolizer>` シンボライザ数。
    pub symbolizers: usize,
    /// `<Datasource>`/`<Parameter>`/`<StyleName>` データソース系タグ数。
    pub datasources: usize,
    /// `<FontSet>`/`<Font>`/`<ShieldSymbolizer>`/`<MarkersSymbolizer>` 等フォント・マーカ系タグ数。
    pub fonts: usize,
    /// その他タグ数。
    pub misc: usize,
}

/// 開始タグ名を順に列挙。
fn tags(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if j < bytes.len() && matches!(bytes[j], b'!' | b'?' | b'/') {
            i += 1;
            continue;
        }
        let start = j;
        while j < bytes.len()
            && (bytes[j].is_ascii_alphanumeric() || matches!(bytes[j], b'_' | b'-' | b'.' | b':'))
        {
            j += 1;
        }
        if j > start {
            out.push(&text[start..j]);
        }
        i = j;
    }
    out
}

/// `b` が Mapnik XML かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.styles + c.layers >= 1)
}

/// `b` を Mapnik スタイル XML として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut it = tags(text);
    match it.first().copied() {
        Some("Map") => {}
        _ => return None,
    }
    it.remove(0);
    let mut c = Counts {
        entries: 1,
        layers: 0,
        styles: 0,
        rules: 0,
        symbolizers: 0,
        datasources: 0,
        fonts: 0,
        misc: 0,
    };
    for t in it {
        c.entries += 1;
        if t.ends_with("Symbolizer") {
            c.symbolizers += 1;
        } else {
            match t {
                "Layer" => c.layers += 1,
                "Style" => c.styles += 1,
                "Rule"
                | "Filter"
                | "ElseFilter"
                | "MinScaleDenominator"
                | "MaxScaleDenominator" => c.rules += 1,
                "Datasource" | "Parameter" | "StyleName" | "LayerOption" => {
                    c.datasources += 1;
                }
                "FontSet" | "Font" | "FontSymbolizer" | "GroupSymbolizer" => {
                    c.fonts += 1;
                }
                _ => c.misc += 1,
            }
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn mapnik() {
        let xml = br#"<Map srs="+init=epsg:3857">
<FontSet name="f"><Font face-name="d"/></FontSet>
<Style name="roads">
  <Rule><Filter>[highway]</Filter><LineSymbolizer stroke="x"/><MaxScaleDenominator>10</MaxScaleDenominator></Rule>
  <Rule><ElseFilter/><PolygonSymbolizer/><TextSymbolizer/></Rule>
</Style>
<Layer name="l"><StyleName>roads</StyleName><Datasource><Parameter name="file">d</Parameter><Parameter name="type">shape</Parameter></Datasource></Layer>
</Map>"#;
        let c = parse(xml).unwrap();
        assert_eq!(c.layers, 1);
        assert_eq!(c.styles, 1);
        assert_eq!(c.rules, 5);
        assert_eq!(c.symbolizers, 3);
        assert_eq!(c.datasources, 4);
        assert_eq!(c.fonts, 2);
    }

    #[test]
    fn not_mapnik() {
        assert!(parse(br#"<html><body/></html>"#).is_none());
    }
}

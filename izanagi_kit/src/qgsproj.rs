//! QGIS プロジェクト `.qgs` / `.qgz` (XML) の検出・カウント。
//!
//! `<qgis>` ルートに `<maplayer>`/`<projectlayers>`/`<layer-tree-group>`
//! `<mapcanvas>`/`<projectproperties>`/`<relations>` を持つ QGIS プロジェクト。
//!
//! ```
//! let xml = br#"<qgis><projectlayers><maplayer><id>1</id><datasource>d</datasource></maplayer></projectlayers>
//!               <layer-tree-group><layer-tree-layer/></layer-tree-group></qgis>"#;
//! assert!(izanagi_kit::qgsproj::detect(xml));
//! let c = izanagi_kit::qgsproj::parse(xml).unwrap();
//! assert_eq!(c.layers, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 開始タグ総数。
    pub entries: usize,
    /// `<maplayer>` レイヤ数。
    pub layers: usize,
    /// `<layer-tree-*`/`<legend*>` ツリー・凡例系タグ数。
    pub tree: usize,
    /// `<projectproperties>` 配下を含むプロパティ・設定系タグ数。
    pub properties: usize,
    /// `<relation>`/`<relations>`/`<fieldref>` リレーション系タグ数。
    pub relations: usize,
    /// `<mapcanvas>`/`<projectlayers>`/`<layerorder>` 等キャンバス・集合タグ数。
    pub containers: usize,
    /// `<id>`/`<layername>`/`<datasource>`/`<provider>`/`<geometry>` 等レイヤ属性タグ数。
    pub attributes: usize,
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

/// `b` が QGIS プロジェクトかどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.layers + c.containers + c.tree >= 1)
}

/// `b` を `.qgs` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut it = tags(text);
    match it.first().copied() {
        Some("qgis") => {}
        _ => return None,
    }
    it.remove(0);
    let mut c = Counts {
        entries: 1,
        layers: 0,
        tree: 0,
        properties: 0,
        relations: 0,
        containers: 0,
        attributes: 0,
        misc: 0,
    };
    for t in it {
        c.entries += 1;
        match t {
            "maplayer" => c.layers += 1,
            "layer-tree-group"
            | "layer-tree-layer"
            | "legend"
            | "legendlayer"
            | "layer-tree-custom-order" => c.tree += 1,
            "relation" | "relations" | "fieldref" => c.relations += 1,
            "mapcanvas" | "projectlayers" | "layerorder" | "visibility-presets" | "mapview"
            | "projectstyles" | "snapping-settings" => {
                c.containers += 1;
            }
            "id" | "layername" | "datasource" | "provider" | "geometry" | "srs"
            | "layerkeywords" | "flags" | "pipe" | "rasterrenderer" => {
                c.attributes += 1;
            }
            _ if t.starts_with("projectproperties")
                || t.starts_with("project-")
                || t == "properties" =>
            {
                c.properties += 1;
            }
            _ => c.misc += 1,
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn qgs() {
        let xml = br#"<qgis projectname="p" version="3">
<projectlayers>
 <maplayer type="vector">
  <id>l1</id><layername>n</layername><datasource>d</datasource><provider>ogr</provider>
  <srs/><geometry>g</geometry>
 </maplayer>
</projectlayers>
<layer-tree-group><layer-tree-layer/><legendlayer/></layer-tree-group>
<mapcanvas/>
<relations><relation/></relations>
<projectproperties><x/></projectproperties>
</qgis>"#;
        let c = parse(xml).unwrap();
        assert_eq!(c.layers, 1);
        assert_eq!(c.tree, 3);
        assert_eq!(c.relations, 2);
        assert_eq!(c.containers, 2);
        assert_eq!(c.attributes, 6);
        assert_eq!(c.properties, 1);
    }

    #[test]
    fn not_qgs() {
        assert!(parse(br#"<project><x/></project>"#).is_none());
    }
}

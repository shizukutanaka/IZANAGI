//! MapProxy `mapproxy.yaml` の検出・カウント。
//!
//! `services`/`layers`/`caches`/`sources`/`grids`/`globals` トップレベル
//! セクションと、その配下のサービス種別・キャッシュ・ソース定義を分類する。
//!
//! ```
//! let cfg = b"services:\n  demo: {}\nlayers:\n  - name: l\n    title: t\n    sources: [c]\ncaches:\n  c:\n    sources: [s]\nsources:\n  s:\n    type: wms\n";
//! assert!(izanagi_kit::mapproxyconf::detect(cfg));
//! let c = izanagi_kit::mapproxyconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 4);
//! ```

/// トップレベルセクション。
const SECTIONS: &[&str] = &[
    "services", "layers", "caches", "sources", "grids", "globals", "parts",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// キー・リスト項目の総数。
    pub entries: usize,
    /// `services`/`layers`/`caches`/`sources`/`grids`/`globals` トップセクション数。
    pub sections: usize,
    /// `layers:` 配下の `- name:` レイヤ定義数。
    pub layers: usize,
    /// `caches:`/`sources:` 配下の定義名数。
    pub caches: usize,
    /// `services:` 配下のサービス種別・オプションキー数。
    pub services: usize,
    /// `grids:`/`globals:` 配下の項目数。
    pub grids: usize,
    /// その他項目数。
    pub misc: usize,
}

/// `b` が mapproxy.yaml かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2)
}

/// `b` を mapproxy.yaml として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        sections: 0,
        layers: 0,
        caches: 0,
        services: 0,
        grids: 0,
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
            "layers" if t.starts_with('-') => c.layers += 1,
            "caches" | "sources" if !name.is_empty() && indent == sindent + 2 => {
                c.caches += 1;
            }
            "services" => c.services += 1,
            "grids" | "globals" if indent == sindent + 2 => c.grids += 1,
            _ => c.misc += 1,
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn mapproxy() {
        let cfg = b"services:\n  demo: {}\n  wms:\n    srs: ['EPSG:3857']\nlayers:\n  - name: layer1\n    title: t\n    sources: [cache1]\n  - name: layer2\n    title: t2\n    sources: [cache1]\ncaches:\n  cache1:\n    grids: [g1]\n    sources: [src1]\nsources:\n  src1:\n    type: wms\n    req:\n      url: http://x\ngrids:\n  g1:\n    srs: EPSG:3857\nglobals:\n  cache:\n    base_dir: /d\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.layers, 2);
        assert_eq!(c.caches, 2);
        assert_eq!(c.services, 3);
        assert_eq!(c.grids, 2);
    }

    #[test]
    fn not_mapproxy() {
        assert!(parse(b"foo: 1\n").is_none());
    }
}

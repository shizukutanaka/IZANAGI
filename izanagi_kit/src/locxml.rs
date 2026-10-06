//! `.loc` (TopoGraphix/LOC GPS ウェイポイントXML) 検出モジュール。
//!
//! LOC ファイル (EasyGPS/GSAK のウェイポイント交換) は XML で、
//! ルート `<loc version="..." src="...">` と `<waypoint>`/
//! `<name id="GCXXXX">`/`<coord lat="..." lon="...">`/`<type>`/
//! `<link>` 要素で構成される。
//!
//! ```
//! let b = br#"<loc version="1.0" src="EasyGPS">
//!   <waypoint>
//!     <name id="GC12345"><![CDATA[Cache Name]]></name>
//!     <coord lat="35.68123" lon="139.76712"/>
//!     <type>Geocache|Traditional Cache</type>
//!     <link text="Cache Details">http://example.com</link>
//!   </waypoint>
//! </loc>"#;
//! let c = izanagi_kit::locxml::parse(b);
//! assert!(izanagi_kit::locxml::detect(b));
//! assert_eq!(c.elements, 8);
//! ```

const ELEMENTS: &[&str] = &[
    "coord", "geocache", "link", "loc", "name", "type", "waypoint",
];

fn loc_elem(t: &str) -> bool {
    let inner = t.trim_start_matches('<').trim_start_matches('/');
    let name: String = inner
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    ELEMENTS.contains(&name.as_str()) && inner[name.len()..].starts_with([' ', '>', '/', '\t'])
}

/// `b` が LOC XML に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut root = 0usize;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if !tr.starts_with('<') {
            continue;
        }
        if tr.starts_with("<loc ") || tr.starts_with("<loc>") {
            root += 1;
            elems += 1;
            continue;
        }
        if loc_elem(tr) {
            elems += 1;
        }
    }
    (root >= 1 && elems >= 4) || (root >= 1 && elems >= 3 && t.contains("<waypoint"))
}

/// LOC XML の統計。
#[derive(Debug, Default, Clone)]
pub struct LocXml {
    /// 既知要素行数。
    pub elements: usize,
    /// `<loc` ルート行数。
    pub root: usize,
}

/// `b` を LOC XML として統計する。
pub fn parse(b: &[u8]) -> LocXml {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = LocXml::default();
    for l in t.lines() {
        let tr = l.trim();
        if !tr.starts_with('<') {
            continue;
        }
        if tr.starts_with("<loc ") || tr.starts_with("<loc>") {
            c.root += 1;
            c.elements += 1;
            continue;
        }
        if loc_elem(tr) {
            c.elements += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<loc version="1.0" src="EasyGPS">
<waypoint>
<name id="GC1"><![CDATA[x]]></name>
<coord lat="1" lon="2"/>
</waypoint>
</loc>"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.root, 1);
    }

    #[test]
    fn detects_with_link() {
        let b = br#"<loc version="1.0">
<waypoint>
<coord lat="1" lon="2"/>
<link>x</link>
</waypoint>
</loc>"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"<loc>\n"));
        assert!(!detect(b"<html><body>hi</body></html>"));
        assert!(!detect(
            b"<waypoint>\n<coord lat=\"1\" lon=\"2\"/>\n</waypoint>\n"
        ));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.elements, 0);
    }
}

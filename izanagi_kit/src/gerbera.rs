//! `gerbera` `config.xml` 検出モジュール。
//!
//! Gerbera(UPnP メディアサーバ)の config.xml は
//! `<config version="…">` ルートに `<server>`( `<ui enabled>`/
//! `<accounts>`/`<name>`/`<udn>`/`<home>`/`<webroot>`/`<storage>`/
//! `<mark-played-items>`/`<port>`/`<ip>`/`<interface>` )、
//! `<import>`( `<filesystem-charset>`/`<metadata-charset>`/
//! `<scripting>`/`<magic-file>`/`<online-content>`/`<layout>`/
//! `<hidden-files>`/`<resources>` )、`<transcoding>`/
//! `<extended-runtime-options>` で構成される。
//!
//! ```
//! let b = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
//!           <config version=\"2\" xmlns=\"http://mediatomb.cc/config/2\">\n\
//!           <server><ui enabled=\"yes\"/><name>Gerbera</name><udn/></server>\n\
//!           <import><hidden-files enabled=\"no\"/></import>\n\
//!           </config>\n";
//! let c = izanagi_kit::gerbera::parse(b);
//! assert!(izanagi_kit::gerbera::detect(b));
//! assert_eq!(c.elements, 2);
//! ```

const ELEMS: &[&str] = &[
    "<server",
    "<ui",
    "<accounts",
    "<account",
    "<name",
    "<udn",
    "<home",
    "<webroot",
    "<storage",
    "<port",
    "<ip",
    "<interface",
    "<server-status",
    "<cache",
    "<live",
    "<load-info",
    "<mark-played-items",
    "<import",
    "<filesystem-charset",
    "<metadata-charset",
    "<playlist-charset",
    "<scripting",
    "<virtual-layout",
    "<magic-file",
    "<online-content",
    "<hidden-files",
    "<resources",
    "<resource",
    "<fanart",
    "<subtitle",
    "<transcode",
    "<transcoding",
    "<mimetype-mappings",
    "<mimetype-contenttype",
    "<extension-mimetype",
    "<treat",
    "<autoscan",
    "<inotify",
    "<ext2mimetype",
    "<header",
    "<agent",
    "<buffer",
    "<chunk-size",
    "<extended-runtime-options",
    "<mark-played-content",
    "<pc directory",
    "<samsung",
    "<bookmarks",
    "<lastplayed",
    "<chat",
    "<youtube",
    "<sopcast",
    "<apple-trailers",
    "<imdb",
    "<tmdb",
    "<trailers",
    "<atavisio",
    "<config-file",
    "<update-check",
];

fn strip_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `b` が gerbera config.xml に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_comments(t);
    let n = ELEMS.iter().filter(|e| t.contains(**e)).count();
    t.contains("<config") && t.contains("<server") && n >= 3
}

/// gerbera config.xml の統計。
#[derive(Debug, Default, Clone)]
pub struct Gerbera {
    /// 既知要素出現行数。
    pub elements: usize,
    /// `<!-- -->` コメント数。
    pub comments: usize,
}

/// `b` を gerbera config.xml として統計する。
pub fn parse(b: &[u8]) -> Gerbera {
    let t = strip_comments(std::str::from_utf8(b).unwrap_or(""));
    let mut c = Gerbera {
        comments: t.matches("<!--").count(),
        ..Gerbera::default()
    };
    if !detect(b) {
        return c;
    }
    for l in t.lines() {
        if ELEMS.iter().any(|e| l.contains(e)) {
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
        let b = b"<config version=\"2\"><server><name>G</name><udn/></server></config>\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.elements, 1);
    }

    #[test]
    fn detects_nested() {
        let b = b"<config>\n<server><ui enabled=\"yes\"/></server>\n<import><hidden-files/></import>\n<transcoding/>\n</config>\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.elements, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"<config><server/></config>\n"));
        assert!(!detect(b"<root><a>1</a></root>\n"));
        assert!(!detect(b"key=value\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.elements, 0);
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}

//! JavaFX `.fxml` form file scanner.
//!
//! FXML files are XML documents describing a scene graph: they start
//! with `<?import …?>` processing instructions for every used class,
//! then a root element whose name is the root controller type (e.g.
//! `<VBox>`, `<AnchorPane>`, `<GridPane>`).
//!
//! ```
//! let d = b"<?import javafx.scene.layout.VBox?>\
//! <VBox xmlns:fx=\"http://javafx.com/fxml\">\
//! <children><Button text=\"OK\"/></children></VBox>";
//! let f = izanagi_kit::fxml::parse(d).unwrap();
//! assert_eq!(f.imports, 1);
//! assert_eq!(f.root, "VBox");
//! assert_eq!(f.elements, 3);
//! ```
//!
//! Reference: JavaFX FXML documentation (`<?import …?>` PI usage and
//! the element-name → class mapping convention).

/// Parsed `.fxml` statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Fxml {
    /// `<?import …?>` processing-instruction count.
    pub imports: usize,
    /// Root element name (`VBox`, `AnchorPane`, …).
    pub root: String,
    /// Total element count (every `<Name …>` opening tag).
    pub elements: usize,
    /// `true` when the `xmlns:fx` namespace is declared.
    pub fx_ns: bool,
}

/// Parse a `.fxml` file; `None` when no `<?import` PI exists.
pub fn parse(d: &[u8]) -> Option<Fxml> {
    let text = core::str::from_utf8(d).ok()?;
    let imports = text.matches("<?import").count();
    if imports == 0 {
        return None;
    }
    // First non-PI tag is the root element.
    let mut rest = text;
    let mut root = String::new();
    while let Some(i) = rest.find('<') {
        let after = &rest[i + 1..];
        if let Some(pi) = after.strip_prefix('?') {
            let end = pi.find("?>")?;
            rest = &pi[end + 2..];
            continue;
        }
        let name: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '_')
            .collect();
        if name.is_empty() {
            return None;
        }
        root = name;
        break;
    }
    if root.is_empty() {
        return None;
    }
    let elements = text.matches('<').count() - imports - text.matches("</").count();
    Some(Fxml {
        imports,
        root,
        elements,
        fx_ns: text.contains("xmlns:fx"),
    })
}

/// `true` if the buffer looks like a `.fxml` file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<?import javafx.scene.layout.VBox?><VBox xmlns:fx=\"http://javafx.com/fxml\"><children><Button text=\"OK\"/></children></VBox>";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.imports, 1);
        assert_eq!(f.root, "VBox");
        assert_eq!(f.elements, 3);
        assert!(f.fx_ns);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html/>").is_none());
        assert!(parse(&[0xff]).is_none());
        assert!(parse(b"<?xml version=\"1\x2e0\"?><x/>").is_none()); // no <?import
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"<?import"));
    }
}

//! Qt Designer `.ui` form file scanner.
//!
//! Qt Designer saves forms as XML with root `<ui version="4.0">`,
//! containing `<class>WindowClass</class>`, `<widget class="QWidget"
//! name="…">` trees, `<layout>` elements, `<signal>`/`<slot>`
//! connections, and `<resources>`/`${rcc}` blocks.
//!
//! ```
//! let d = b"<ui version=\"4\x2e0\"><class>MainW</class>\
//! <widget class=\"QMainWindow\" name=\"w\">\
//! <property name=\"geometry\"><rect><x>0</x></rect></property></widget></ui>";
//! let q = izanagi_kit::qtui::parse(d).unwrap();
//! assert_eq!(q.version.as_deref(), Some("4\x2e0"));
//! assert_eq!(q.class.as_deref(), Some("MainW"));
//! assert_eq!(q.widgets, 1);
//! ```
//!
//! Reference: Qt Designer `.ui` file format (the `<ui>`/`<class>`/
//! `<widget>`/`<layout>` schema documented in Qt's uic manual).

/// Parsed `.ui` statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct QtUi {
    /// `version=` attribute of the `<ui>` root.
    pub version: Option<String>,
    /// `<class>` contents — the form's generated class name.
    pub class: Option<String>,
    /// `<widget>` element count.
    pub widgets: usize,
    /// `<layout>` element count.
    pub layouts: usize,
    /// `<property>` element count.
    pub properties: usize,
    /// `<connection>` element count.
    pub connections: usize,
}

/// Parse a `.ui` form; `None` if the `<ui` root is absent.
pub fn parse(d: &[u8]) -> Option<QtUi> {
    let text = core::str::from_utf8(d).ok()?;
    if !text.contains("<ui ") && !text.contains("<ui>") {
        return None;
    }
    let ui_start = text.find("<ui")?;
    let ui_tag_end = text[ui_start..].find('>')? + ui_start;
    let ui_attrs = &text[ui_start..ui_tag_end];
    let version = ui_attrs
        .split("version=\"")
        .nth(1)
        .and_then(|r| r.split('"').next())
        .map(|s| s.to_string());
    let class = text
        .split("<class>")
        .nth(1)
        .and_then(|r| r.split("</class>").next())
        .map(|s| s.trim().to_string());
    Some(QtUi {
        version,
        class,
        widgets: text.matches("<widget ").count() + text.matches("<widget>").count(),
        layouts: text.matches("<layout ").count() + text.matches("<layout>").count(),
        properties: text.matches("<property ").count() + text.matches("<property>").count(),
        connections: text.matches("<connection").count() - text.matches("<connections").count(),
    })
}

/// `true` if the buffer looks like a Qt Designer `.ui` file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<?xml version=\"1\x2e0\"?><ui version=\"4\x2e0\"><class>MainW</class><widget class=\"QMainWindow\" name=\"w\"><layout class=\"QVBoxLayout\"><property name=\"x\"><number>1</number></property></layout></widget><connections><connection/></connections></ui>";

    #[test]
    fn parses() {
        let q = parse(DOC).unwrap();
        assert_eq!(q.version.as_deref(), Some("4\x2e0"));
        assert_eq!(q.class.as_deref(), Some("MainW"));
        assert_eq!(q.widgets, 1);
        assert_eq!(q.layouts, 1);
        assert_eq!(q.properties, 1);
        assert_eq!(q.connections, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html/>").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"<ui"));
    }
}

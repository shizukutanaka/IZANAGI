//! Apple Interface Builder `.xib` document scanner.
//!
//! Modern `.xib` files are XML with root `<document>` carrying
//! `type="com.apple.InterfaceBuilder3.CocoaTouch.XIB"` (iOS) or
//! `com.apple.InterfaceBuilder3.Cocoa.XIB` (macOS), plus `<objects>`
//! and `<connections>` sections.
//!
//! ```
//! let d = b"<?xml version=\"1\x2e0\"?><document type=\"com.apple.InterfaceBuilder3.CocoaTouch.XIB\" version=\"3\x2e0\"><objects><view id=\"v\"/></objects></document>";
//! let x = izanagi_kit::xib::parse(d).unwrap();
//! assert_eq!(x.platform, "iOS");
//! assert_eq!(x.objects, 1);
//! ```
//!
//! Reference: Apple Interface Builder XIB documentation (the
//! `com.apple.InterfaceBuilder3.*` document-type strings and the
//! `<document>`/`<objects>`/`<connections>` element set).

/// Parsed `.xib` statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Xib {
    /// `"iOS"` for `CocoaTouch.XIB`, `"macOS"` for `Cocoa.XIB`,
    /// `"unknown"` for other `InterfaceBuilder3` types.
    pub platform: String,
    /// `version=` attribute of the `<document>` root, if any.
    pub version: Option<String>,
    /// Element count inside `<objects>`/`<dependencies>` (object,
    /// view, viewController, …).
    pub objects: usize,
    /// `<connections>` child count.
    pub connections: usize,
    /// `<outlet>`/`<action>` binding count.
    pub bindings: usize,
}

/// Parse a `.xib` file; `None` when no InterfaceBuilder marker exists.
pub fn parse(d: &[u8]) -> Option<Xib> {
    let text = core::str::from_utf8(d).ok()?;
    if !text.contains("InterfaceBuilder") {
        return None;
    }
    let platform = if text.contains("CocoaTouch") {
        "iOS"
    } else if text.contains("Cocoa") {
        "macOS"
    } else {
        "unknown"
    }
    .to_string();
    let version = text
        .split("<document ")
        .nth(1)
        .and_then(|r| r.split("version=\"").nth(1))
        .and_then(|r| r.split('"').next())
        .map(|s| s.to_string());
    Some(Xib {
        platform,
        version,
        objects: text.matches("<object ").count()
            + text.matches("<view ").count()
            + text.matches("<viewController").count()
            + text.matches("<window").count(),
        connections: text.matches("<connections>").count(),
        bindings: text.matches("<outlet").count() + text.matches("<action").count(),
    })
}

/// `true` if the buffer looks like a `.xib` document.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<?xml version=\"1\x2e0\"?><document type=\"com.apple.InterfaceBuilder3.CocoaTouch.XIB\" version=\"3\x2e0\"><objects><view id=\"v\"/><viewController id=\"c\"><connections><outlet property=\"x\"/></connections></viewController></objects></document>";

    #[test]
    fn parses() {
        let x = parse(DOC).unwrap();
        assert_eq!(x.platform, "iOS");
        assert_eq!(x.version.as_deref(), Some("3\x2e0"));
        assert_eq!(x.objects, 2);
        assert_eq!(x.connections, 1);
        assert_eq!(x.bindings, 1);
    }

    #[test]
    fn macos() {
        let x = parse(b"<document type=\"com.apple.InterfaceBuilder3.Cocoa.XIB\"/>").unwrap();
        assert_eq!(x.platform, "macOS");
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
        assert!(!detect(b"<xib"));
    }
}

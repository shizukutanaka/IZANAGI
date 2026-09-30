//! GtkBuilder `.glade` / `.ui` interface file scanner.
//!
//! GtkBuilder XML files have root `<interface>` (GtkBuilder) or
//! `<glade-interface>` (libglade), containing `<requires lib="gtk+"
//! version="…"/>` lines and `<object class="GtkWindow" …>` widgets.
//!
//! ```
//! let d = b"<interface><requires lib=\"gtk+\" version=\"4\x2e0\"/>\
//! <object class=\"GtkWindow\" id=\"w\"><property name=\"title\">T</property></object></interface>";
//! let g = izanagi_kit::glade::parse(d).unwrap();
//! assert_eq!(g.objects, 1);
//! assert!(g.gtk_version.is_some());
//! ```
//!
//! Reference: GTK Builder XML documentation (`<interface>` +
//! `<object>`/`<property>` schema) and libglade's `<glade-interface>`
//! legacy form.

/// Parsed GtkBuilder file statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Glade {
    /// `true` for legacy `<glade-interface>` roots, `false` for
    /// `<interface>` (GtkBuilder).
    pub legacy: bool,
    /// `version=` value from the first `<requires>` element, if any.
    pub gtk_version: Option<String>,
    /// `<object class="…">` widget count.
    pub objects: usize,
    /// `<signal>` handler-binding count.
    pub signals: usize,
    /// `<property …>` count.
    pub properties: usize,
}

/// Parse a GtkBuilder `.glade` file; `None` if no interface root exists.
pub fn parse(d: &[u8]) -> Option<Glade> {
    let text = core::str::from_utf8(d).ok()?;
    let legacy = text.contains("<glade-interface");
    if !legacy && !text.contains("<interface>") && !text.contains("<interface ") {
        return None;
    }
    let gtk_version = text
        .split("<requires")
        .nth(1)
        .and_then(|r| r.split("version=\"").nth(1))
        .and_then(|r| r.split('"').next())
        .map(|s| s.to_string());
    Some(Glade {
        legacy,
        gtk_version,
        objects: text.matches("<object ").count() + text.matches("<object>").count(),
        signals: text.matches("<signal ").count() + text.matches("<signal>").count(),
        properties: text.matches("<property ").count() + text.matches("<property>").count(),
    })
}

/// `true` if the buffer looks like a GtkBuilder file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<?xml version=\"1\x2e0\"?><interface><requires lib=\"gtk+\" version=\"4\x2e0\"/><object class=\"GtkWindow\" id=\"w\"><property name=\"title\">T</property><signal name=\"destroy\" handler=\"on_quit\"/></object></interface>";

    #[test]
    fn parses() {
        let g = parse(DOC).unwrap();
        assert!(!g.legacy);
        assert_eq!(g.gtk_version.as_deref(), Some("4\x2e0"));
        assert_eq!(g.objects, 1);
        assert_eq!(g.signals, 1);
        assert_eq!(g.properties, 1);
    }

    #[test]
    fn legacy_root() {
        let g = parse(b"<glade-interface></glade-interface>").unwrap();
        assert!(g.legacy);
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
        assert!(!detect(b"<interface"));
    }
}

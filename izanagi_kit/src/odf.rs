//! ODF (OpenDocument) package inspection via `crate::zip`.
//!
//! ODF is a ZIP container whose first entry must be `mimetype`
//! stored uncompressed with content like
//! `application/vnd.oasis.opendocument.text`, plus
//! `META-INF/manifest.xml`.
//!
//! ```
//! use izanagi_kit::odf;
//! let mut w = izanagi_kit::zip::ZipWriter::new();
//! w.add_stored("mimetype", b"application/vnd.oasis.opendocument.text");
//! w.add_stored("META-INF/manifest.xml", b"<manifest/>");
//! w.add("content.xml", b"{}");
//! let z = w.finish();
//! let o = odf::inspect(&z).unwrap();
//! assert_eq!(o.kind, odf::Kind::Text);
//! assert!(o.has_manifest);
//! ```

use std::string::String;

/// ODF document family from the mimetype.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `*.odt` — text.
    Text,
    /// `*.ods` — spreadsheet.
    Spreadsheet,
    /// `*.odp` — presentation.
    Presentation,
    /// `*.odg` — drawing.
    Drawing,
    /// `*.odf` — formula.
    Formula,
    /// `*.odc` — chart.
    Chart,
    /// `*.odb` — database.
    Database,
    /// `*.odi` — image.
    Image,
    /// `*.odm` — text master.
    TextMaster,
    /// Flat ODF XML or other `application/vnd.oasis`/sun mimetype.
    Other,
}

/// Parsed ODF package info.
#[derive(Clone, Debug, PartialEq)]
pub struct Odf {
    /// Raw `mimetype` member contents.
    pub mimetype: String,
    /// Document family.
    pub kind: Kind,
    /// True when `META-INF/manifest.xml` is present.
    pub has_manifest: bool,
}

fn kind_of(mt: &str) -> Kind {
    match mt {
        "application/vnd.oasis.opendocument.text" => Kind::Text,
        "application/vnd.oasis.opendocument.spreadsheet" => Kind::Spreadsheet,
        "application/vnd.oasis.opendocument.presentation" => Kind::Presentation,
        "application/vnd.oasis.opendocument.graphics" => Kind::Drawing,
        "application/vnd.oasis.opendocument.formula" => Kind::Formula,
        "application/vnd.oasis.opendocument.chart" => Kind::Chart,
        "application/vnd.oasis.opendocument.database" => Kind::Database,
        "application/vnd.oasis.opendocument.image" => Kind::Image,
        "application/vnd.oasis.opendocument.text-master" => Kind::TextMaster,
        _ => Kind::Other,
    }
}

/// Inspects a ZIP byte blob as ODF: first entry must be
/// `mimetype` stored uncompressed (method 0), its content must
/// start with `application/vnd.oasis.` or `application/vnd.sun.xml.`.
pub fn inspect(d: &[u8]) -> Option<Odf> {
    let entries = crate::zip::list(d)?;
    let first = entries.first()?;
    if first.name != "mimetype" || first.method != 0 {
        return None;
    }
    let body = crate::zip::extract(d, "mimetype")?;
    let mimetype = String::from_utf8_lossy(&body).trim().to_string();
    if !(mimetype.starts_with("application/vnd.oasis.")
        || mimetype.starts_with("application/vnd.sun.xml."))
    {
        return None;
    }
    let has_manifest = entries.iter().any(|e| e.name == "META-INF/manifest.xml");
    Some(Odf {
        kind: kind_of(&mimetype),
        mimetype,
        has_manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn build(mt: &str, manifest: bool) -> Vec<u8> {
        let mut w = crate::zip::ZipWriter::new();
        w.add_stored("mimetype", mt.as_bytes());
        if manifest {
            w.add_stored("META-INF/manifest.xml", b"<manifest/>");
        }
        w.finish()
    }

    #[test]
    fn detects_families() {
        let d = build("application/vnd.oasis.opendocument.spreadsheet", true);
        let o = inspect(&d).unwrap();
        assert_eq!(o.kind, Kind::Spreadsheet);
        assert!(o.has_manifest);
        let d = build("application/vnd.sun.xml.writer", false);
        assert_eq!(inspect(&d).unwrap().kind, Kind::Other);
    }

    #[test]
    fn rejects() {
        assert!(inspect(b"not a zip").is_none());
        assert!(inspect(&build("text/plain", false)).is_none());
    }
}

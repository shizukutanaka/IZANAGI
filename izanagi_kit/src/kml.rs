//! KML (Keyhole Markup Language): `<kml>` documents containing
//! `<Placemark>` features with `<name>` and `<coordinates>`
//! (`lon,lat[,alt]` whitespace-separated). Coordinates stay textual —
//! converting to degrees is the caller's job.
//!
//! ```
//! use izanagi_kit::kml::parse;
//!
//! let src = "<kml><Document><Placemark><name>Tokyo</name>\
//!            <Point><coordinates>139.7,35.6,0</coordinates></Point></Placemark>\
//!            </Document></kml>";
//! let k = parse(src).unwrap();
//! assert_eq!(k.placemarks.len(), 1);
//! assert_eq!(k.placemarks[0].name.as_deref(), Some("Tokyo"));
//! assert_eq!(k.placemarks[0].coordinates.as_deref(), Some("139.7,35.6,0"));
//! ```

use std::string::String;
use std::vec::Vec;

/// One `<Placemark>` feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placemark {
    /// `<name>` text.
    pub name: Option<String>,
    /// `<description>` text.
    pub description: Option<String>,
    /// `<coordinates>` text (`lon,lat[,alt]` tuples, space-separated).
    pub coordinates: Option<String>,
}

/// Parsed KML document.
#[derive(Debug, Clone)]
pub struct Kml {
    /// `<Document><name>` if present.
    pub name: Option<String>,
    /// All `<Placemark>` features in document order.
    pub placemarks: Vec<Placemark>,
}

/// `<name ..>text</name>` inside `src[from..]`.
fn tag_text(src: &str, name: &str, from: usize) -> Option<String> {
    let open = String::from("<");
    let i = src[from..].find((open.clone() + name).as_str())? + from;
    let gt = src[i..].find('>')? + i;
    if src[..gt].ends_with('/') {
        return Some(String::new());
    }
    let close = String::from("</") + name + ">";
    let j = src[gt + 1..].find(close.as_str())? + gt + 1;
    Some(src[gt + 1..j].trim().to_string())
}

/// All `<name ..>inner</name>` inner ranges.
fn blocks(src: &str, name: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let open = String::from("<") + name;
    let close = String::from("</") + name + ">";
    let mut at = 0usize;
    while let Some(i) = src[at..].find(open.as_str()).map(|p| p + at) {
        let Some(gt) = src[i..].find('>').map(|p| p + i) else {
            break;
        };
        let Some(j) = src[gt + 1..].find(close.as_str()).map(|p| p + gt + 1) else {
            at = gt + 1;
            continue;
        };
        out.push((gt + 1, j));
        at = j + close.len();
    }
    out
}

/// Parse a `<kml>` document into its placemarks.
pub fn parse(src: &str) -> Option<Kml> {
    if !src.contains("<kml") {
        return None;
    }
    let mut placemarks = Vec::new();
    for (a, b) in blocks(src, "Placemark") {
        let inner = &src[a..b];
        placemarks.push(Placemark {
            name: tag_text(inner, "name", 0).filter(|s| !s.is_empty()),
            description: tag_text(inner, "description", 0).filter(|s| !s.is_empty()),
            coordinates: tag_text(inner, "coordinates", 0).filter(|s| !s.is_empty()),
        });
    }
    Some(Kml {
        name: tag_text(src, "name", 0).filter(|s| !s.is_empty()),
        placemarks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "<kml><Document><name>Doc</name>\
        <Placemark><name>A</name><description>d</description>\
        <Point><coordinates>1.5,2.5</coordinates></Point></Placemark>\
        <Placemark><LineString><coordinates>0,0 3,3</coordinates></LineString></Placemark>\
        </Document></kml>";

    #[test]
    fn fields() {
        let k = parse(DOC).unwrap();
        assert_eq!(k.name.as_deref(), Some("Doc"));
        assert_eq!(k.placemarks.len(), 2);
        assert_eq!(k.placemarks[0].name.as_deref(), Some("A"));
        assert_eq!(k.placemarks[0].description.as_deref(), Some("d"));
        assert_eq!(k.placemarks[0].coordinates.as_deref(), Some("1.5,2.5"));
        assert!(k.placemarks[1].name.is_none());
        assert_eq!(k.placemarks[1].coordinates.as_deref(), Some("0,0 3,3"));
    }

    #[test]
    fn rejects() {
        assert!(parse("<xml/>").is_none());
        assert_eq!(parse("<kml/>").unwrap().placemarks.len(), 0);
    }
}

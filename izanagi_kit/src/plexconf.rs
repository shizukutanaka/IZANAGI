//! Plex Media Server `Preferences.xml` census.
//!
//! Plex Preferences.xml keeps every setting as attributes on a single
//! `<Preferences .../>` element: `MachineIdentifier`,
//! `ProcessedMachineIdentifier`, `OldestPreviousVersion`,
//! `AnnounceToken`, `AcceptedEULA`, `FSEventLibraryUpdatesEnabled`,
//! `Dvr*`, `PublishServerOnPlexOnlineKey`, `General` prefs,
//! `allowedNetworks`, `disable*`, `secureConnections`,
//! `customCertificate*`, `lanNetworksBandwidth*`,
//! `autoEmptyTrash`, `enableHTTPS`, `OnDeckWindow`.
//!
//! ```rust
//! let c = izanagi_kit::plexconf::Plexconf::parse(b"<Preferences MachineIdentifier=\"x\" AcceptedEULA=\"1\"/>").unwrap();
//! assert_eq!(c.attributes, 2);
//! ```

use crate::textutil::strip_xml_comments;
/// Plex `Preferences.xml` census.
#[derive(Debug, Clone)]
pub struct Plexconf {
    /// `name="value"` attribute pairs on `<Preferences>`.
    pub attributes: usize,
    /// Attributes whose value is `0`/`1`/`true`/`false`.
    pub flags: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a Plex Preferences.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_xml_comments(t);
    t.contains("<Preferences")
        && (t.contains("MachineIdentifier")
            || t.contains("OldestPreviousVersion")
            || t.contains("ProcessedMachineIdentifier")
            || t.contains("FSEventLibraryUpdatesEnabled")
            || t.contains("AnnounceToken")
            || t.contains("AcceptedEULA"))
}

impl Plexconf {
    /// Parse a Plex Preferences.xml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = strip_xml_comments(std::str::from_utf8(b).ok()?);
        let mut c = Self {
            attributes: 0,
            flags: 0,
            comments: 0,
        };
        c.comments = t.matches("<!--").count();
        let b = t.as_bytes();
        let mut i = 0usize;
        while i < b.len() {
            match b[i] {
                b'"' => {
                    i += 1;
                    while i < b.len() && b[i] != b'"' {
                        i += 1;
                    }
                    i += 1;
                }
                b'=' => {
                    i += 1;
                    while i < b.len() && b[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    if i < b.len() && b[i] == b'"' {
                        i += 1;
                        let start = i;
                        while i < b.len() && b[i] != b'"' {
                            i += 1;
                        }
                        let v = &t[start..i];
                        i += 1;
                        c.attributes += 1;
                        if v == "0" || v == "1" || v == "true" || v == "false" {
                            c.flags += 1;
                        }
                    }
                }
                _ => i += 1,
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_preferences() {
        let b = concat!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
            "<Preferences OldestPreviousVersion=\"v132\" MachineIdentifier=\"mid\"\n",
            "  ProcessedMachineIdentifier=\"pmid\" AnnounceToken=\"tok\"\n",
            "  AcceptedEULA=\"1\" FSEventLibraryUpdatesEnabled=\"1\"\n",
            "  secureConnections=\"2\" disableRemoteSecurity=\"0\"\n",
            "  autoEmptyTrash=\"1\" customCertificatePath=\"/cert.pem\" />\n",
        );
        let c = Plexconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.attributes, 12);
        assert_eq!(c.flags, 4);
    }

    #[test]
    fn rejects_other() {
        assert!(Plexconf::parse(b"<Preferences foo=\"1\"/>").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_xml_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_xml_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}

//! Jellyfin `system.xml`/`network.xml`/library `options.xml` census.
//!
//! Jellyfin config is XML under `<ServerConfiguration>`:
//! `<LogFileRetentionDays>`/`<IsStartupWizardCompleted>`/
//! `<CachePath>`/`<EnableMetrics>`/`<UICulture>`/
//! `<PublicPort>`/`<HttpPort>`/`<MinResumePct>`/`<MaxResumePct>`/
//! `<LibraryOptions>` + `<VirtualFolder>`/`<ImageOptions>`/
//! `<TypeOptions>`/`<RemoteClientBitrateLimit>`/`<KnownProxies>`/
//! `<IgnoreVirtualInterfaces>`/`xmlns:xsi`/`xmlns:xsd` attributes.
//!
//! ```rust
//! let c = izanagi_kit::jellyfin::Jellyfin::parse(b"<ServerConfiguration>\n<EnableMetrics>false</EnableMetrics>\n<PublicPort>8096</PublicPort>\n</ServerConfiguration>").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

/// Jellyfin config XML census.
#[derive(Debug, Clone)]
pub struct Jellyfin {
    /// Elements opening a nested block (`<Tag>` without inline close).
    pub sections: usize,
    /// `<Key>value</Key>`/self-closing element entries.
    pub entries: usize,
    /// `true`/`false` element values.
    pub booleans: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "IsStartupWizardCompleted",
    "CachePath",
    "EnableMetrics",
    "UICulture",
    "PublicPort",
    "HttpPort",
    "MinResumePct",
    "MaxResumePct",
    "LibraryOptions",
    "VirtualFolder",
    "ImageOptions",
    "TypeOptions",
    "RemoteClientBitrateLimit",
    "KnownProxies",
    "IgnoreVirtualInterfaces",
    "Subtitle",
    "EncodeMethod",
    "TranscodingTempPath",
    "ServerName",
    "EnableHttps",
];

/// Whether the buffer looks like a Jellyfin config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<ServerConfiguration") && KEYS.iter().filter(|k| t.contains(**k)).count() >= 2
}

impl Jellyfin {
    /// Parse a Jellyfin config file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            booleans: 0,
            comments: 0,
        };
        c.comments = t.matches("<!--").count();
        for l in t.lines() {
            let s = l.trim();
            if let Some(rest) = s.strip_prefix('<') {
                if rest.starts_with('!') || rest.starts_with('?') || rest.starts_with('/') {
                    continue;
                }
                if let Some(gt) = rest.find('>') {
                    let head = &rest[..gt];
                    let name = head.split(' ').next().unwrap_or("");
                    if name.is_empty()
                        || name == "ServerConfiguration"
                        || name == "configuration"
                        || !name.bytes().all(|x| x.is_ascii_alphanumeric() || x == b'_')
                    {
                        continue;
                    }
                    let val = &rest[gt + 1..];
                    if rest.ends_with("/>") {
                        c.entries += 1;
                    } else if let Some(end) = val.find("</") {
                        c.entries += 1;
                        let v = &val[..end];
                        if v == "true" || v == "false" || v == "True" || v == "False" {
                            c.booleans += 1;
                        }
                    } else {
                        c.sections += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_system_xml() {
        let b = concat!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
            "<ServerConfiguration xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\n",
            "  <IsStartupWizardCompleted>true</IsStartupWizardCompleted>\n",
            "  <CachePath />\n",
            "  <EnableMetrics>false</EnableMetrics>\n",
            "  <UICulture>en-US</UICulture>\n",
            "  <PublicPort>8096</PublicPort>\n",
            "  <LibraryOptions>\n",
            "    <VirtualFolder name=\"Movies\" />\n",
            "    <ImageOptions />\n",
            "  </LibraryOptions>\n",
            "  <!-- jellyfin -->\n",
            "</ServerConfiguration>\n",
        );
        let c = Jellyfin::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.entries, 7);
        assert_eq!(c.booleans, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Jellyfin::parse(b"<Config><InstanceName>Sonarr</InstanceName></Config>").is_none());
    }
}

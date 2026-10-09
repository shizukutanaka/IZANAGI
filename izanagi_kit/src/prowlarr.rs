//! Prowlarr `config.xml` census.
//!
//! Prowlarr config.xml is flat XML under `<Config>`:
//! `<BindAddress>`/`<Port>`(9696)/`<SslPort>`/`<UrlBase>`/`<ApiKey>`/
//! `<AuthenticationMethod>`/`<EnableSsl>`/`<InstanceName>Prowlarr`/
//! `<LaunchBrowser>`/`<UpdateMechanism>`/`<Branch>`/
//! `<AnalyticsEnabled>`/`<LogLevel>`/`<HistoryCleanupDays>`/
//! `<Theme>` entries.
//!
//! ```rust
//! let c = izanagi_kit::prowlarr::Prowlarr::parse(b"<Config>\n<InstanceName>Prowlarr</InstanceName>\n<Port>9696</Port>\n</Config>").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

use crate::textutil::strip_xml_comments;
/// Prowlarr `config.xml` census.
#[derive(Debug, Clone)]
pub struct Prowlarr {
    /// `<Key>value</Key>` element entries.
    pub entries: usize,
    /// `True`/`False` element values.
    pub booleans: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a Prowlarr config.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_xml_comments(t);
    t.contains("<Config>")
        && (t.contains("<InstanceName>Prowlarr")
            || t.contains("<Port>9696")
            || t.contains("<Prowlarr"))
}

impl Prowlarr {
    /// Parse a Prowlarr config.xml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = strip_xml_comments(std::str::from_utf8(b).ok()?);
        let mut c = Self {
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
                    let name = &rest[..gt];
                    if name.is_empty()
                        || name == "Config"
                        || !name.bytes().all(|x| x.is_ascii_alphanumeric() || x == b'_')
                    {
                        continue;
                    }
                    let val = &rest[gt + 1..];
                    if let Some(end) = val.find("</") {
                        let v = &val[..end];
                        c.entries += 1;
                        if v == "True" || v == "False" || v == "true" || v == "false" {
                            c.booleans += 1;
                        }
                    } else if rest.ends_with("/>") {
                        c.entries += 1;
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
    fn parses_prowlarr() {
        let b = concat!(
            "<Config>\n",
            "  <LogLevel>info</LogLevel>\n",
            "  <BindAddress>*</BindAddress>\n",
            "  <Port>9696</Port>\n",
            "  <EnableSsl>False</EnableSsl>\n",
            "  <LaunchBrowser>True</LaunchBrowser>\n",
            "  <ApiKey>dddddddddddddddddddddddddddddddd</ApiKey>\n",
            "  <AuthenticationMethod>None</AuthenticationMethod>\n",
            "  <InstanceName>Prowlarr</InstanceName>\n",
            "  <Branch>master</Branch>\n",
            "  <HistoryCleanupDays>365</HistoryCleanupDays>\n",
            "</Config>\n",
        );
        let c = Prowlarr::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 10);
        assert_eq!(c.booleans, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Prowlarr::parse(b"<Config><InstanceName>Radarr</InstanceName></Config>").is_none());
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

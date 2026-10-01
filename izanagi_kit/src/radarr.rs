//! Radarr `config.xml` census.
//!
//! Radarr config.xml is flat XML under `<Config>`:
//! `<BindAddress>`/`<Port>`(7878)/`<SslPort>`/`<UrlBase>`/`<ApiKey>`/
//! `<AuthenticationMethod>`/`<EnableSsl>`/`<InstanceName>Radarr`/
//! `<LaunchBrowser>`/`<UpdateMechanism>`/`<Branch>`/
//! `<AnalyticsEnabled>`/`<LogLevel>`/`<Theme>`/`<Backup*>`/
//! `<ProxyType>`/`<RecycleBin>` entries.
//!
//! ```rust
//! let c = izanagi_kit::radarr::Radarr::parse(b"<Config>\n<InstanceName>Radarr</InstanceName>\n<Port>7878</Port>\n</Config>").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

/// Radarr `config.xml` census.
#[derive(Debug, Clone)]
pub struct Radarr {
    /// `<Key>value</Key>` element entries.
    pub entries: usize,
    /// `True`/`False` element values.
    pub booleans: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a Radarr config.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<Config>")
        && (t.contains("<InstanceName>Radarr") || t.contains("<Port>7878") || t.contains("<Radarr"))
}

impl Radarr {
    /// Parse a Radarr config.xml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
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
    fn parses_radarr() {
        let b = concat!(
            "<Config>\n",
            "  <LogLevel>info</LogLevel>\n",
            "  <BindAddress>*</BindAddress>\n",
            "  <Port>7878</Port>\n",
            "  <EnableSsl>False</EnableSsl>\n",
            "  <LaunchBrowser>True</LaunchBrowser>\n",
            "  <ApiKey>bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb</ApiKey>\n",
            "  <AuthenticationMethod>Forms</AuthenticationMethod>\n",
            "  <InstanceName>Radarr</InstanceName>\n",
            "  <Branch>master</Branch>\n",
            "  <AnalyticsEnabled>False</AnalyticsEnabled>\n",
            "  <RecycleBin>/media/.recycle</RecycleBin>\n",
            "  <Theme>dark</Theme>\n",
            "</Config>\n",
        );
        let c = Radarr::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 12);
        assert_eq!(c.booleans, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Radarr::parse(b"<Config><InstanceName>Sonarr</InstanceName></Config>").is_none());
    }
}

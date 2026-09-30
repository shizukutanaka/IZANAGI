//! Sonarr `config.xml` census.
//!
//! Sonarr config.xml is flat XML under `<Config>`:
//! `<BindAddress>`/`<Port>`(8989)/`<SslPort>`/`<UrlBase>`/`<ApiKey>`/
//! `<AuthenticationMethod>`/`<AuthenticationRequired>`/
//! `<EnableSsl>`/`<SslCertPath>`/`<InstanceName>Sonarr`/
//! `<LaunchBrowser>`/`<UpdateMechanism>`/`<Branch>`/
//! `<AnalyticsEnabled>`/`<LogLevel>`/`<SyslogUrl>`/`<Theme>`/
//! `<BackupFolder>`/`<BackupInterval>`/`<BackupRetention>`/
//! `<ProxyType>`/`<ProxyHostname>`/`<CertFingerprint>` entries.
//!
//! ```rust
//! let c = izanagi_kit::sonarr::Sonarr::parse(b"<Config>\n<InstanceName>Sonarr</InstanceName>\n<Port>8989</Port>\n</Config>").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

/// Sonarr `config.xml` census.
#[derive(Debug, Clone)]
pub struct Sonarr {
    /// `<Key>value</Key>` element entries.
    pub entries: usize,
    /// `True`/`False` element values.
    pub booleans: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

/// Whether the buffer looks like a Sonarr config.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<Config>")
        && (t.contains("<InstanceName>Sonarr")
            || t.contains("<Port>8989")
            || (t.contains("<Sonarr") && t.contains("<ApiKey")))
}

impl Sonarr {
    /// Parse a Sonarr config.xml into census counts.
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
                    } else if name.ends_with('/') || rest.ends_with("/>") {
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
    fn parses_sonarr() {
        let b = concat!(
            "<Config>\n",
            "  <LogLevel>info</LogLevel>\n",
            "  <BindAddress>*</BindAddress>\n",
            "  <Port>8989</Port>\n",
            "  <SslPort>9898</SslPort>\n",
            "  <EnableSsl>False</EnableSsl>\n",
            "  <LaunchBrowser>True</LaunchBrowser>\n",
            "  <ApiKey>aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa</ApiKey>\n",
            "  <AuthenticationMethod>None</AuthenticationMethod>\n",
            "  <InstanceName>Sonarr</InstanceName>\n",
            "  <UpdateMechanism>BuiltIn</UpdateMechanism>\n",
            "  <Branch>main</Branch>\n",
            "  <AnalyticsEnabled>False</AnalyticsEnabled>\n",
            "  <UrlBase></UrlBase>\n",
            "  <!-- keep -->\n",
            "</Config>\n",
        );
        let c = Sonarr::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 13);
        assert_eq!(c.booleans, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Sonarr::parse(b"<Config><InstanceName>Radarr</InstanceName></Config>").is_none());
    }
}

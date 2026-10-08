//! Lidarr `config.xml` census.
//!
//! Lidarr config.xml is flat XML under `<Config>`:
//! `<BindAddress>`/`<Port>`(8686)/`<SslPort>`/`<UrlBase>`/`<ApiKey>`/
//! `<AuthenticationMethod>`/`<EnableSsl>`/`<InstanceName>Lidarr`/
//! `<LaunchBrowser>`/`<UpdateMechanism>`/`<Branch>`/
//! `<AnalyticsEnabled>`/`<LogLevel>`/`<Theme>` entries.
//!
//! ```rust
//! let c = izanagi_kit::lidarr::Lidarr::parse(b"<Config>\n<InstanceName>Lidarr</InstanceName>\n<Port>8686</Port>\n</Config>").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

/// Lidarr `config.xml` census.
#[derive(Debug, Clone)]
pub struct Lidarr {
    /// `<Key>value</Key>` element entries.
    pub entries: usize,
    /// `True`/`False` element values.
    pub booleans: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

fn strip_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether the buffer looks like a Lidarr config.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_comments(t);
    t.contains("<Config>")
        && (t.contains("<InstanceName>Lidarr") || t.contains("<Port>8686") || t.contains("<Lidarr"))
}

impl Lidarr {
    /// Parse a Lidarr config.xml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = strip_comments(std::str::from_utf8(b).ok()?);
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
    fn parses_lidarr() {
        let b = concat!(
            "<Config>\n",
            "  <LogLevel>debug</LogLevel>\n",
            "  <BindAddress>*</BindAddress>\n",
            "  <Port>8686</Port>\n",
            "  <EnableSsl>False</EnableSsl>\n",
            "  <LaunchBrowser>True</LaunchBrowser>\n",
            "  <ApiKey>cccccccccccccccccccccccccccccccc</ApiKey>\n",
            "  <AuthenticationMethod>External</AuthenticationMethod>\n",
            "  <InstanceName>Lidarr</InstanceName>\n",
            "  <Branch>nightly</Branch>\n",
            "  <UpdateMechanism>Docker</UpdateMechanism>\n",
            "</Config>\n",
        );
        let c = Lidarr::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 10);
        assert_eq!(c.booleans, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Lidarr::parse(b"<Config><InstanceName>Sonarr</InstanceName></Config>").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}

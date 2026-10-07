//! Readarr `config.xml` 検出モジュール。
//!
//! Readarr(*arr 系)の config.xml は `<Config>` 直下の
//! `<Port>8787`/`<SslPort>`/`<UrlBase>`/`<ApiKey>`/
//! `<AuthenticationMethod>`/`<EnableSsl>`/`<InstanceName>Readarr`/
//! `<LaunchBrowser>`/`<Branch>`/`<LogLevel>`/`<SslCertPath>`/
//! `<UpdateMechanism>`/`<AnalyticsEnabled>` 等の要素で構成される。
//!
//! ```
//! let b = b"<Config>\n<InstanceName>Readarr</InstanceName>\n<Port>8787</Port>\n<ApiKey>x</ApiKey>\n</Config>\n";
//! let c = izanagi_kit::readarr::parse(b);
//! assert!(izanagi_kit::readarr::detect(b));
//! assert_eq!(c.entries, 3);
//! ```

/// `b` が Readarr config.xml に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<Config>")
        && (t.contains("<InstanceName>Readarr")
            || t.contains("<Port>8787")
            || (t.contains("<Readarr") && t.contains("<ApiKey")))
}

/// Readarr config.xml の統計。
#[derive(Debug, Default, Clone)]
pub struct Readarr {
    /// `<Key>value</Key>` 要素数。
    pub entries: usize,
    /// 真偽値要素数。
    pub booleans: usize,
    /// `<!-- -->` コメント数。
    pub comments: usize,
}

/// `b` を Readarr config.xml として統計する。
pub fn parse(b: &[u8]) -> Readarr {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Readarr {
        comments: t.matches("<!--").count(),
        ..Readarr::default()
    };
    if !detect(b) {
        return c;
    }
    for l in t.lines() {
        let s = l.trim();
        let Some(rest) = s.strip_prefix('<') else {
            continue;
        };
        if rest.starts_with('!') || rest.starts_with('?') || rest.starts_with('/') {
            continue;
        }
        let Some(gt) = rest.find('>') else {
            continue;
        };
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
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"<Config>\n<InstanceName>Readarr</InstanceName>\n<Port>8787</Port>\n</Config>\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.entries, 2);
    }

    #[test]
    fn detects_port() {
        let b = b"<Config>\n<Port>8787</Port>\n<ApiKey>x</ApiKey>\n</Config>\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(
            b"<Config>\n<InstanceName>Sonarr</InstanceName>\n<Port>8989</Port>\n</Config>\n"
        ));
        assert!(!detect(b"<root><a>1</a></root>\n"));
        assert!(!detect(b"key=value\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.entries, 0);
    }
}

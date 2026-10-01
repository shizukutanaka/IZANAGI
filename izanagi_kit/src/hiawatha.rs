//! Census of a Hiawatha `hiawatha.conf`.
//!
//! Hiawatha config: `Variable = value` settings, `Binding { … }`/
//! `VirtualHost { … }`/`Directory { … }`/`FastCGIserver { … }`/
//! `UrlToolkit { … }`/`TLSCertFile` blocks with `Hostname`/`Port`/
//! `Interface`/`DocumentRoot`/`StartFile`/`AccessLogfile`/`ErrorLogfile`/
//! `TimeForCGI`/`Extension`/`ReverseProxy`/`RequireTLS`/`PreventXSS`/
//! `PreventSQLi`/`PreventCSRF`/`ToolkitID`/`Match`/`RequestURI`/
//! `Gzip`/`UserDirectory` directives, `#` comments.
//!
//! ```rust
//! let c = izanagi_kit::hiawatha::Hiawatha::parse(
//!     b"VirtualHost {\n    Hostname = example.com\n    DocumentRoot = /srv\n}\n",
//! ).unwrap();
//! assert_eq!(c.blocks, 1);
//! ```
#![forbid(unsafe_code)]

/// Hiawatha config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hiawatha {
    /// `Name {`/`Name=…` block headers (VirtualHost/Binding/Directory/etc.).
    pub blocks: usize,
    /// `key = value` settings.
    pub settings: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Block keywords.
const BLOCKS: &[&str] = &[
    "VirtualHost",
    "Binding",
    "Directory",
    "FastCGIserver",
    "UrlToolkit",
    "CGIhandler",
    "Thread",
];

/// True if `b` looks like Hiawatha config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("VirtualHost") && t.contains('='))
        || t.contains("UrlToolkit")
        || t.contains("FastCGIserver")
        || (t.contains("Hostname") && t.contains("DocumentRoot"))
}

impl Hiawatha {
    /// Parse an `hiawatha.conf` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            blocks: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') || l.starts_with(';') {
                c.comments += 1;
            } else if BLOCKS.iter().any(|k| l.starts_with(k)) && l.contains('{') {
                c.blocks += 1;
            } else if l.contains('=') {
                c.settings += 1;
            }
        }
        if c.settings == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# hiawatha\n",
            "ServerId = hiawatha\n",
            "ConnectionsTotal = 1000\n",
            "Binding {\n",
            "    Port = 443\n",
            "    Interface = 0.0.0.0\n",
            "    MaxRequestSize = 2048\n",
            "}\n",
            "VirtualHost {\n",
            "    Hostname = example.com\n",
            "    DocumentRoot = /srv/www\n",
            "    StartFile = index.html\n",
            "    AccessLogfile = /var/log/hiawatha/access.log\n",
            "    RequireTLS = yes\n",
            "}\n",
            "UrlToolkit {\n",
            "    ToolkitID = cms\n",
            "    Match ^/old(.*) Redirect /new$1\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Hiawatha::parse(b.as_bytes()).unwrap();
        assert_eq!(c.blocks, 3);
        assert!(c.settings >= 10);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"server { listen 80; }\n"));
        assert!(Hiawatha::parse(b"# none\n").is_none());
    }
}

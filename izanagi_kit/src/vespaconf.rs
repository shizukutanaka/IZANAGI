//! Vespa `services.xml` parser.
//!
//! Detects Vespa application `services.xml` by `<container`/`<content`/
//! `<document-api`/`<search`/`<storage`/`<jdisc`/`<slobrok`/
//! `<configserver`/`<cluster` tags under a `<services>` root, and counts
//! tag structure.
//!
//! ```
//! let b = b"<services version=\"1.0\">\n  <container id=\"query\" version=\"1.0\">\n    <search />\n    <document-api />\n  </container>\n  <content id=\"music\" version=\"1.0\">\n    <documents>\n      <document type=\"music\" mode=\"index\" />\n    </documents>\n  </content>\n</services>\n";
//! assert!(izanagi_kit::vespaconf::detect(b));
//! let c = izanagi_kit::vespaconf::Vespa::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed services.xml summary.
#[derive(Debug, Clone)]
pub struct Vespa {
    /// Recognized tag occurrences.
    pub keys: usize,
    /// `<container`/`<search`/`<document-api` query-side tags.
    pub container_keys: usize,
    /// `<content`/`<storage`/`<distribution`/`<persistence` storage-side tags.
    pub content_keys: usize,
    /// `<document`/`<documents` schema tags.
    pub document_keys: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

/// Container-side tags.
const CONTAINER_KEYS: &[&str] = &[
    "<container",
    "<search",
    "<document-api",
    "<handler",
    "<component",
    "<processing",
    "<document-processing",
    "<accesslog",
    "<server",
    "<http-server",
    "<threadpool",
    "<nodes",
    "<node",
    "<jdis\u{63}",
    "<http",
    "<chain",
    "<searcher",
];

/// Content-side tags.
const CONTENT_KEYS: &[&str] = &[
    "<content",
    "<storage",
    "<distribution",
    "<persistence",
    "<bucket",
    "<group",
    "<nodes",
    "<node",
    "<tuning",
    "<dispatch",
    "<cluster",
    "<redundancy",
    "<coverage",
    "<min-",
    "<merges",
    "<distributor",
    "<filedistribution",
];

/// Schema/config tags.
const OTHER_KEYS: &[&str] = &[
    "<services",
    "<service",
    "<documents",
    "<document",
    "<configserver",
    "<slobrok",
    "<slobroks",
    "<admin",
    "<adminserver",
    "<logd",
    "<metrics",
    "<application",
    "<config",
    "<deployment",
    "<include",
    "<resources",
    "<resource-limits",
    "<environment",
    "<runtime",
    "<roots",
    "<retention",
    "<reindexing",
    "<selector",
    "<query",
    "<summary",
    "<instances",
    "<profile",
];

/// Document schema tags.
const DOCUMENT_KEYS: &[&str] = &["<documents", "<document"];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Vespa services.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = CONTAINER_KEYS
        .iter()
        .chain(CONTENT_KEYS.iter())
        .chain(OTHER_KEYS.iter())
        .filter(|k| key_present(t, k))
        .count();
    hits >= 2 && t.contains("<services")
}

impl Vespa {
    /// Count categories. Returns `None` when the input does not look like
    /// a services.xml.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            container_keys: 0,
            content_keys: 0,
            document_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            if l.contains("<!--") {
                c.comments += 1;
            }
        }
        for k in CONTAINER_KEYS {
            c.container_keys += t.matches(k).count();
        }
        for k in CONTENT_KEYS {
            c.content_keys += t.matches(k).count();
        }
        for k in DOCUMENT_KEYS {
            c.document_keys += t.matches(k).count();
        }
        let mut other = 0;
        for k in OTHER_KEYS {
            other += t.matches(k).count();
        }
        c.keys = c.container_keys + c.content_keys + c.document_keys + other;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"<services version=\"1.0\">\n  <container id=\"query\" version=\"1.0\">\n    <search />\n    <document-api />\n    <nodes count=\"1\">\n      <node hostalias=\"node1\" />\n    </nodes>\n  </container>\n  <content id=\"music\" version=\"1.0\">\n    <redundancy>1</redundancy>\n    <documents>\n      <document type=\"music\" mode=\"index\" />\n    </documents>\n    <nodes count=\"1\">\n      <node hostalias=\"node1\" />\n    </nodes>\n  </content>\n</services>\n";
        assert!(detect(b));
        let c = Vespa::parse(b).unwrap();
        assert!(c.container_keys >= 4);
        assert!(c.content_keys >= 5);
        assert_eq!(c.document_keys, 4);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_xml() {
        assert!(!detect(b"<foo><bar/></foo>\n"));
        assert!(Vespa::parse(b"").is_none());
    }
}

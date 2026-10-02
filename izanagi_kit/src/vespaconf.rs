//! Vespa `services.xml` の検出と構造カウント。
//!
//! `<services>` ルート + `<container>`/`<content>`/`<jdisc>`/`<admin>` クラスタ +
//! `<search>`/`<document-api>`/`<documents>`/`<nodes>`/`<node>` を XML タグ走査で分類。
//!
//! ```
//! let c = izanagi_kit::vespaconf::parse(
//!     b"<services version=\"1.0\"><container id=\"default\" version=\"1.0\"><search/></container></services>").unwrap();
//! assert_eq!(c.elements, 3);
//! assert!(izanagi_kit::vespaconf::detect(b"<services><content id=\"x\" version=\"1.0\"/></services>"));
//! ```

/// Vespa トップレベルクラスタ要素。
const CLUSTERS: &[&str] = &[
    "admin",
    "clients",
    "container",
    "content",
    "jdisc",
    "slobrok",
];
/// クラスタ配下の既知要素。
const FEATURES: &[&str] = &[
    "accesslog",
    "admin",
    "clients",
    "cluster",
    "clustercontroller",
    "config",
    "configserver",
    "container",
    "content",
    "coverage",
    "distributor",
    "document",
    "document-api",
    "document-processing",
    "documents",
    "documentdb",
    "engine",
    "feed",
    "filedistribution",
    "http",
    "index",
    "logserver",
    "metrics",
    "node",
    "nodes",
    "persistence",
    "processing",
    "proton",
    "query",
    "redundancy",
    "resource-limits",
    "rot",
    "search",
    "searchchain",
    "server",
    "slobrok",
    "slobroks",
    "storage",
    "streaming",
    "tuning",
    "transactionlog",
    "visitors",
];

/// `<name` のタグ名を列挙(`<?`/`<!`/`</` を除く)。
fn tags(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'<' && bytes[i + 1] != b'?' && bytes[i + 1] != b'!' && bytes[i + 1] != b'/'
        {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || matches!(bytes[j], b'_' | b'-' | b'.'))
            {
                j += 1;
            }
            if j > start {
                out.push(&text[start..j]);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Vespa 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 全要素数。
    pub elements: usize,
    /// クラスタ要素(container/content/jdisc/admin)。
    pub clusters: usize,
    /// クラスタ配下の機構要素(search/documents/nodes…)。
    pub features: usize,
    /// 未知要素。
    pub misc: usize,
}

/// b が services.xml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let ts = tags(text);
    match ts.first() {
        Some(r) if *r == "services" => {}
        _ => return false,
    }
    ts[1..].iter().filter(|t| CLUSTERS.contains(t)).count() >= 1
}

/// services.xml の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let ts = tags(text);
    if ts.is_empty() {
        return None;
    }
    let mut c = Counts {
        elements: 0,
        clusters: 0,
        features: 0,
        misc: 0,
    };
    for t in ts {
        c.elements += 1;
        if t == "services" {
            continue;
        }
        if CLUSTERS.contains(&t) {
            c.clusters += 1;
        } else if FEATURES.contains(&t) {
            c.features += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<services version=\"1.0\" xmlns:deploy=\"vespa\" xmlns:preprocess=\"properties\">\n  <container id=\"default\" version=\"1.0\">\n    <search/>\n    <document-api/>\n    <nodes>\n      <node hostalias=\"node1\"/>\n      <node hostalias=\"node2\"/>\n    </nodes>\n  </container>\n  <content id=\"music\" version=\"1.0\">\n    <redundancy>2</redundancy>\n    <documents>\n      <document type=\"music\" mode=\"index\"/>\n    </documents>\n    <nodes>\n      <node distribution-key=\"0\" hostalias=\"node1\"/>\n    </nodes>\n  </content>\n  <admin version=\"2.0\">\n    <configservers>\n      <configserver hostalias=\"node1\"/>\n    </configservers>\n  </admin>\n</services>\n";

    #[test]
    fn vespaconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.clusters, 3);
        assert!(c.features >= 8);
        assert_eq!(c.elements, c.clusters + c.features + c.misc + 1);
    }

    #[test]
    fn not_vespa() {
        assert!(!detect(b"<html><body>x</body></html>"));
        assert!(!detect(b"hello\n"));
    }
}

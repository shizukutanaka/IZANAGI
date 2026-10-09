//! ActiveMQ ブローカー設定(`activemq.xml`)の検出と構造カウント。
//!
//! `<beans>`/`<broker>` ルートと `<transportConnectors>`/
//! `<networkConnectors>`/`<destinationsPolicy>`/`<persistenceAdapter>`/
//! `<plugins>`/`<systemUsage>` 等のコンテナ、`<transportConnector>`/
//! `<policyEntry>`/`<authorizationEntry>`/`<kahaDB>` 等のエントリ、
//! `uri=`/`brokerName=`/`userName=`/`password=`/`queue=`/`topic=` 属性を識別する。
//!
//! ```
//! let c = izanagi_kit::activemq::parse(
//!     b"<broker brokerName=\"b1\">\n  <transportConnectors>\n    <transportConnector uri=\"tcp://0.0.0.0:61616\"/>\n  </transportConnectors>\n</broker>\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.entries, 1);
//! assert!(izanagi_kit::activemq::detect(b"<broker>\n  <transportConnectors>\n    <transportConnector uri=\"tcp://x:61616\"/>\n  </transportConnectors>\n</broker>\n"));
//! ```

use crate::textutil::strip_xml_comments;
/// 既知コンテナ要素。
const CONTAINERS: &[&str] = &[
    "beans",
    "broker",
    "destinationInterceptors",
    "destinationsPolicy",
    "jmsBridgeConnectors",
    "managementContext",
    "networkConnectors",
    "persistenceAdapter",
    "plugins",
    "policyEntries",
    "policyMap",
    "regionBroker",
    "schedulePolicy",
    "sslContext",
    "systemUsage",
    "taskRunnerFactory",
    "transportConnectors",
    "virtualDestinations",
];

/// 既知エントリ要素。
const ENTRIES: &[&str] = &[
    "authenticationPlugin",
    "authorizationEntry",
    "authorizationMap",
    "authorizationPlugin",
    "bean",
    "compositeDiscoveryAgent",
    "discoveryAgent",
    "dynamicallyIncludedDestinations",
    "excludedDestinations",
    "filteredDestination",
    "jdbcPersistenceAdapter",
    "kahaDB",
    "levelDB",
    "memoryUsage",
    "mKahaDB",
    "multicastDiscoveryAgent",
    "networkConnector",
    "policyEntry",
    "propertyPlaceholderConfigurer",
    "queue",
    "replicatedLevelDB",
    "simpleAuthenticationPlugin",
    "staticDiscoveryAgent",
    "staticallyIncludedDestinations",
    "storeUsage",
    "tempAuthorizationEntry",
    "tempUsage",
    concat!("topi", "\u{63}"),
    "transportConnector",
    "virtualDestinationInterceptor",
    concat!("virtualTopi", "\u{63}"),
    "vmTransportConnector",
];

/// 既知属性名。
const ATTRS: &[&str] = &[
    "brokerName=",
    "class=",
    "clientID=",
    "destName=",
    "groups=",
    "id=",
    "memoryLimit=",
    "name=",
    "password=",
    "persistent=",
    "physicalName=",
    "producerFlowControl=",
    "queue=",
    "storeUsage=",
    "tempUsage=",
    concat!("topi", "c="),
    "uri=",
    "useJmx=",
    "userName=",
    "users=",
];

/// activemq 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知コンテナ要素行。
    pub sections: usize,
    /// 既知エントリ要素行。
    pub entries: usize,
    /// 既知属性出現数。
    pub attributes: usize,
    /// `<!-- -->` コメント行。
    pub comments: usize,
    /// 分類不能行(未知タグ・閉じタグ・その他)。
    pub misc: usize,
}

/// 行内の開始/自己完結タグ名を列挙(閉じタグ `</name>` は除外)。
fn tags_in_line<'a>(t: &'a str, out: &mut Vec<&'a str>) {
    let b = t.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'<' && b[i + 1].is_ascii_alphabetic() {
            let start = i + 1;
            let mut j = start;
            while j < b.len()
                && (b[j].is_ascii_alphanumeric() || b[j] == b'.' || b[j] == b'-' || b[j] == b'_')
            {
                j += 1;
            }
            if j > start {
                out.push(&t[start..j]);
            }
            i = j;
        } else {
            i += 1;
        }
    }
}

/// 行内の既知属性出現数。
fn attrs_in(t: &str) -> usize {
    ATTRS.iter().filter(|a| t.contains(**a)).count()
}

/// b が activemq.xml かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = strip_xml_comments(core::str::from_utf8(b).unwrap_or(""));
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("<!--") || t.starts_with("<?") {
            continue;
        }
        let mut tags = Vec::new();
        tags_in_line(t, &mut tags);
        hits += tags
            .iter()
            .filter(|tag| CONTAINERS.contains(tag) || ENTRIES.contains(tag))
            .count();
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = strip_xml_comments(core::str::from_utf8(b).ok()?);
    let mut c = Counts {
        sections: 0,
        entries: 0,
        attributes: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("<!--") {
            c.comments += 1;
            continue;
        }
        // 閉じタグのみの行は構造行。
        if t.starts_with("</") {
            continue;
        }
        let mut tags = Vec::new();
        tags_in_line(t, &mut tags);
        c.attributes += attrs_in(t);
        if tags.is_empty() {
            if t.chars().any(|ch| ch.is_alphanumeric()) && !t.starts_with("<?xml") {
                c.misc += 1;
            }
            continue;
        }
        for tag in tags {
            if CONTAINERS.contains(&tag) {
                c.sections += 1;
            } else if ENTRIES.contains(&tag) {
                c.entries += 1;
            } else {
                c.misc += 1;
            }
        }
    }
    (c.sections + c.entries >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<beans>\n  <broker brokerName=\"local\" persistent=\"true\" useJmx=\"false\">\n    <transportConnectors>\n      <transportConnector name=\"openwire\" uri=\"tcp://0.0.0.0:61616\"/>\n      <transportConnector name=\"stomp\" uri=\"stomp://0.0.0.0:61613\"/>\n    </transportConnectors>\n    <destinationsPolicy>\n      <policyMap>\n        <policyEntries>\n          <policyEntry queue=\">\" producerFlowControl=\"true\" memoryLimit=\"10mb\"/>\n        </policyEntries>\n      </policyMap>\n    </destinationsPolicy>\n    <persistenceAdapter>\n      <kahaDB directory=\"data/kahadb\"/>\n    </persistenceAdapter>\n  </broker>\n</beans>\n";

    #[test]
    fn activemq() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 7);
        assert_eq!(c.entries, 4);
        assert!(c.attributes >= 6);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_activemq() {
        assert!(!detect(b"<html><body>hi</body></html>\n"));
        assert!(!detect(b"<broker/>\n"));
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

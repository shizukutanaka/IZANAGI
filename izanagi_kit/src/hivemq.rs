//! HiveMQ ブローカー設定(`config.xml`)の検出と構造カウント。
//!
//! `<hivemq>` ルートと `<listeners>`(`<tcp-listener>`/`<websocket-listener>`/
//! `<tls-tcp-listener>`/`<udp-listener>`)、`<cluster>`/`<mqtt>`/`<security>`/
//! `<restrictions>`/`<overload-protection>`/`<logging>`/`<usage-statistics>` 等の
//! ブロック要素を識別する。
//!
//! ```
//! let c = izanagi_kit::hivemq::parse(
//!     b"<hivemq>\n  <listeners>\n    <tcp-listener>\n      <port>1883</port>\n    </tcp-listener>\n  </listeners>\n</hivemq>\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.entries, 2);
//! assert!(izanagi_kit::hivemq::detect(b"<hivemq>\n  <listeners>\n    <tcp-listener/>\n  </listeners>\n</hivemq>\n"));
//! ```

use crate::textutil::strip_xml_comments;
/// ブロック/コンテナ要素。
const CONTAINERS: &[&str] = &[
    "cluster",
    "discovery",
    "extension",
    "hivemq",
    "listeners",
    "logging",
    "mqtt",
    "mqtt-configuration-defaults",
    "overload-protection",
    "queued-message-strategy",
    "replication",
    "restrictions",
    "security",
    "extension-security",
    "server-information",
    "session-expiry",
    "message-expiry",
    "usage-statistics",
    "anonymous-usage-statistics",
    "control-listener",
    "transport",
    "tls",
    "truststore",
    "keystore",
    "trace-recording",
];

/// リスナー/リーフエントリ要素。
const ENTRIES: &[&str] = &[
    "allow-empty-client-id",
    "bind-address",
    "broadcast",
    "client-certificates",
    "extension",
    "keep-alive",
    "max-client-identifier-length",
    "max-connections",
    "max-topic-length",
    "multicast",
    "name",
    "no-connect-idle-timeout",
    "port",
    "proxy-protocol",
    "qos-0-priority",
    "rate-limit",
    "retained-messages",
    "retry-interval",
    "shared-subscriptions",
    concat!("stati", "\u{63}"),
    "subscription-identifiers",
    "tcp",
    "tcp-listener",
    "tls-tcp-listener",
    "tls-websocket-listener",
    "topic-alias",
    "udp",
    "udp-listener",
    "web",
    "websocket-listener",
    "wildcard-subscriptions",
];

/// hivemq 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知コンテナ要素行。
    pub sections: usize,
    /// 既知エントリ/リーフ要素行。
    pub entries: usize,
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

/// b が config.xml(HiveMQ)かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = strip_xml_comments(core::str::from_utf8(b).unwrap_or(""));
    let mut hits = 0;
    let mut root = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("<!--") || t.starts_with("<?") {
            continue;
        }
        let mut tags = Vec::new();
        tags_in_line(t, &mut tags);
        for tag in tags {
            if tag == "hivemq" {
                root = true;
            }
            if CONTAINERS.contains(&tag) || ENTRIES.contains(&tag) {
                hits += 1;
            }
        }
    }
    root && hits >= 3
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = strip_xml_comments(core::str::from_utf8(b).ok()?);
    let mut c = Counts {
        sections: 0,
        entries: 0,
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

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<hivemq>\n  <listeners>\n    <tcp-listener>\n      <port>1883</port>\n      <bind-address>0.0.0.0</bind-address>\n    </tcp-listener>\n    <tls-tcp-listener>\n      <port>8883</port>\n      <tls>\n        <keystore>\n          <path>conf/hivemq.jks</path>\n        </keystore>\n      </tls>\n    </tls-tcp-listener>\n    <websocket-listener>\n      <port>8000</port>\n    </websocket-listener>\n  </listeners>\n  <mqtt>\n    <session-expiry>\n      <max-interval>120</max-interval>\n    </session-expiry>\n  </mqtt>\n  <restrictions>\n    <max-connections>100000</max-connections>\n  </restrictions>\n</hivemq>\n";

    #[test]
    fn hivemq() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 7);
        assert_eq!(c.entries, 8);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_hivemq() {
        assert!(!detect(b"<html><body>hi</body></html>\n"));
        assert!(!detect(b"<hivemq/>\n"));
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

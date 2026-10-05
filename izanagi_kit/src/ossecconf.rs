//! OSSEC/Wazuh `ossec.conf` の検出と構造カウント。
//!
//! `<ossec_config>` ルート + `global`/`syscheck`/`rootcheck`/`alerts`/
//! `rules`/`decoders`/`localfile`/`remote`/`active-response`/`client`/`wodle`/
//! `indexer`/`vulnerability-detection`/`sca`/`logging`/`command`/`agentless`/
//! `monitor`/`cluster`/`integration`/`email_alerts`/`reports` ブロックを識別する。
//!
//! ```
//! let c = izanagi_kit::ossecconf::parse(
//!     b"<ossec_config>\n  <global>\n    <email_notification>yes</email_notification>\n  </global>\n  <syscheck>\n    <disabled>no</disabled>\n  </syscheck>\n</ossec_config>\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::ossecconf::detect(
//!     b"<ossec_config><syscheck><disabled>no</disabled></syscheck></ossec_config>\n"));
//! ```

/// 設定ブロック(コンテナ)タグ。
const BLOCKS: &[&str] = &[
    "active-response",
    "agentless",
    "alerts",
    "auth",
    "client",
    "cluster",
    "command",
    "database_output",
    "decoders",
    "email_alerts",
    "global",
    "indexer",
    "integration",
    "localfile",
    "logging",
    "monitor",
    "remote",
    "reports",
    "rootcheck",
    "rules",
    "sca",
    "syslog_output",
    "syscheck",
    "vulnerability-detection",
    "wodle",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<ossec_config>` 行数。
    pub sections: usize,
    /// ブロック開始タグ数。
    pub entries: usize,
    /// その他の設定要素(`<key>` 形式)行数。
    pub options: usize,
    /// コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn tags_in_line(t: &str, f: &mut dyn FnMut(&str)) {
    let b = t.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'<' && i + 1 < b.len() && b[i + 1].is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-' || b[j] == b'_') {
                j += 1;
            }
            f(&t[i + 1..j]);
            i = j;
        } else {
            i += 1;
        }
    }
}

/// `ossec.conf` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut root = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("<!--") || t.starts_with("<?") || t.starts_with("</") {
            continue;
        }
        tags_in_line(t, &mut |tag| {
            if tag == "ossec_config" {
                root = true;
            }
            if BLOCKS.contains(&tag) {
                hits += 1;
            }
        });
        if root && hits >= 2 {
            return true;
        }
    }
    root && hits >= 1
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("<!--") || t.starts_with("<?") {
            c.comments += 1;
            continue;
        }
        if t.starts_with("</") {
            continue;
        }
        let mut counted = false;
        tags_in_line(t, &mut |tag| {
            if tag == "ossec_config" {
                c.sections += 1;
                counted = true;
            } else if BLOCKS.contains(&tag) {
                c.entries += 1;
                counted = true;
            } else {
                c.options += 1;
                counted = true;
            }
        });
        if !counted && t.chars().any(|ch| ch.is_ascii_alphanumeric()) {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<!-- ossec -->\n<ossec_config>\n  <global>\n    <email_notification>yes</email_notification>\n    <email_to>admin@example.com</email_to>\n    <smtp_server>localhost</smtp_server>\n  </global>\n  <rules>\n    <include>rules_config.xml</include>\n  </rules>\n  <syscheck>\n    <disabled>no</disabled>\n    <frequency>7200</frequency>\n    <directories>/etc,/usr/bin</directories>\n  </syscheck>\n  <rootcheck>\n    <disabled>no</disabled>\n  </rootcheck>\n  <localfile>\n    <log_format>syslog</log_format>\n    <location>/var/log/messages</location>\n  </localfile>\n</ossec_config>\n";

    #[test]
    fn ossecconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.entries, 5);
        assert_eq!(c.options, 10);
        assert_eq!(c.comments, 2);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_ossecconf() {
        assert!(!detect(b"<root><a/></root>\n"));
        assert!(parse(b"text\n").is_none());
    }
}

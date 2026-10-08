//! Sysmon XML 設定(`sysmonconfig*.xml`)の検出と構造カウント。
//!
//! `<Sysmon schemaversion>` ルート、`<EventFiltering>`、`<RuleGroup>`
//! (groupRelation/onmatch/name)、イベント種別タグ(ProcessCreate/
//! NetworkConnect/RegistryEvent/DnsQuery/FileDelete/...) を識別する。
//! SwiftOnSecurity 形式にも対応。
//!
//! ```
//! let c = izanagi_kit::sysmonconf::parse(
//!     b"<Sysmon schemaversion=\"4.22\">\n  <EventFiltering>\n    <RuleGroup>\n      <ProcessCreate onmatch=\"exclude\"/>\n      <NetworkConnect onmatch=\"include\"/>\n    </RuleGroup>\n  </EventFiltering>\n</Sysmon>\n").unwrap();
//! assert_eq!(c.entries, 2);
//! assert!(izanagi_kit::sysmonconf::detect(
//!     b"<Sysmon schemaversion=\"4.22\"><EventFiltering><ProcessCreate onmatch=\"exclude\"/></EventFiltering></Sysmon>\n"));
//! ```

use crate::textutil::strip_xml_comments;
/// イベント/コンテナ要素。
const TAGS: &[&str] = &[
    "ArchiveDirectory",
    "CaptureClipboard",
    "CheckRevocation",
    "ClipboardChange",
    "CreateRemoteThread",
    "DnsQuery",
    "DriverLoad",
    "EventFiltering",
    "FileBlockCompressed",
    "FileBlockExecutable",
    "FileBlockShredding",
    "FileCreate",
    "FileCreateStreamHash",
    "FileCreateTime",
    "FileDelete",
    "FileDeleteDetected",
    "ImageLoad",
    "NetworkConnect",
    "PipeEvent",
    "ProcessAccess",
    "ProcessCreate",
    "ProcessTampering",
    "ProcessTerminate",
    "RawAccessRead",
    "RegistryEvent",
    "RuleGroup",
    "WmiEvent",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<Sysmon>`/`<EventFiltering>`/`<RuleGroup>` 行数。
    pub sections: usize,
    /// イベント種別タグ出現数。
    pub entries: usize,
    /// 条件要素(`<CommandLine onmatch="...">` 等)行数。
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

/// Sysmon XML らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_xml_comments(text);
    let mut root = false;
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("<!--") || t.starts_with("<?") || t.starts_with("</") {
            continue;
        }
        tags_in_line(t, &mut |tag| {
            if tag == "Sysmon" {
                root = true;
            }
            if TAGS.contains(&tag) {
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
        tags_in_line(t, &mut |tag| match tag {
            "Sysmon" | "EventFiltering" | "RuleGroup" => {
                c.sections += 1;
                counted = true;
            }
            x if TAGS.contains(&x) => {
                c.entries += 1;
                counted = true;
            }
            _ => {
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

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\"?>\n<!-- Sysmon -->\n<Sysmon schemaversion=\"4.82\">\n  <EventFiltering>\n    <RuleGroup name=\"default\" groupRelation=\"or\">\n      <ProcessCreate onmatch=\"exclude\">\n        <CommandLine condition=\"is\">C:\\Windows\\system32\\svchost.exe</CommandLine>\n        <Image condition=\"image\">init.exe</Image>\n      </ProcessCreate>\n      <NetworkConnect onmatch=\"include\">\n        <DestinationPort condition=\"is\">443</DestinationPort>\n      </NetworkConnect>\n      <RegistryEvent onmatch=\"exclude\"/>\n      <DnsQuery onmatch=\"exclude\"/>\n      <FileCreateTime onmatch=\"exclude\"/>\n    </RuleGroup>\n  </EventFiltering>\n</Sysmon>\n";

    #[test]
    fn sysmonconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 5);
        assert_eq!(c.options, 3);
        assert_eq!(c.comments, 2);
    }

    #[test]
    fn not_sysmonconf() {
        assert!(!detect(b"<root><a/></root>\n"));
        assert!(parse(b"text\n").is_none());
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

//! NuGet 設定ファイル(`nuget.config`)の検出と構造カウント。
//!
//! `<configuration>` ルート内の `<packageSources>`/`<disabledPackageSources>`/
//! `<packageRestore>`/`<activePackageSource>`/`<packageSourceCredentials>`/
//! `<apikeys>`/`<trustedSigners>`/`<fallbackPackageFolders>`/`<config>`/
//! `<auditSources>` セクションと `<add key=".." value=".."/>`/`<clear/>`
//! エントリを識別する。
//!
//! ```
//! let c = izanagi_kit::nugetconfig::parse(
//!     b"<configuration>\n  <packageSources>\n    <add key=\"nuget\" value=\"https://api.nuget.org/v3/index.json\" />\n  </packageSources>\n</configuration>\n").unwrap();
//! assert_eq!(c.elements, 2);
//! assert_eq!(c.entries, 1);
//! assert!(izanagi_kit::nugetconfig::detect(b"<packageSources>\n  <add key=\"a\" value=\"b\" />\n</packageSources>\n"));
//! ```

use crate::textutil::strip_xml_comments;
/// 既知セクション/コンテナ要素名。
const CONTAINERS: &[&str] = &[
    "activePackageSource",
    "apikeys",
    "auditSources",
    "certificate",
    "config",
    "configuration",
    "disabledPackageSources",
    "fallbackPackageFolders",
    "owners",
    "packageRestore",
    "packageSourceCredentials",
    "packageSources",
    "pluginSources",
    "repository",
    "solution",
    "trustedSigners",
];

/// エントリ要素名(`key`/`value` 属性を持つもの + `<clear>`)。
const ENTRIES: &[&str] = &[
    "add",
    "author",
    "clear",
    "environmentVariables",
    "fileCert",
    "remove",
    "storeCert",
];

/// nuget.config 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知セクション/コンテナ要素の開始タグ。
    pub elements: usize,
    /// `<add>`/`<clear>`/`<remove>` 等のエントリ要素。
    pub entries: usize,
    /// `key=`/`value=` 属性出現数。
    pub attributes: usize,
    /// `<!-- -->` コメント行。
    pub comments: usize,
    /// 未知タグ/分類不能行。
    pub misc: usize,
}

/// 行内の開始/自己完結タグ名を列挙(`<name`・`<name/>`; 閉じタグ `</name>` は除外)。
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

/// 行内の `key=`/`value=`/`protocolVersion=`/`password=` 等属性出現数。
fn attrs_in(t: &str) -> usize {
    let mut n = 0;
    for attr in [
        "key=",
        "value=",
        "protocolVersion=",
        "password=",
        "clearTextPassword=",
    ] {
        let mut s = t;
        while let Some(p) = s.find(attr) {
            n += 1;
            s = &s[p + attr.len()..];
        }
    }
    n
}

/// b が nuget.config かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = strip_xml_comments(core::str::from_utf8(b).unwrap_or(""));
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        let mut tags = Vec::new();
        tags_in_line(t, &mut tags);
        hits += tags
            .iter()
            .filter(|k| CONTAINERS.contains(k) || ENTRIES.contains(k))
            .count();
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = strip_xml_comments(core::str::from_utf8(b).ok()?);
    let mut c = Counts {
        elements: 0,
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
                c.elements += 1;
            } else if ENTRIES.contains(&tag) {
                c.entries += 1;
            } else if tag != "?xml" {
                c.misc += 1;
            }
        }
    }
    (c.elements >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<configuration>\n  <packageSources>\n    <add key=\"nuget.org\" value=\"https://api.nuget.org/v3/index.json\" protocolVersion=\"3\" />\n    <add key=\"local\" value=\"./feed\" />\n  </packageSources>\n  <disabledPackageSources>\n    <add key=\"local\" value=\"true\" />\n  </disabledPackageSources>\n  <packageSourceCredentials>\n    <local>\n      <add key=\"Username\" value=\"me\" />\n      <add key=\"ClearTextPassword\" value=\"x\" />\n    </local>\n  </packageSourceCredentials>\n  <config>\n    <add key=\"signatureValidationMode\" value=\"accept\" />\n  </config>\n</configuration>\n";

    #[test]
    fn nugetconfig() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.elements, 5);
        assert_eq!(c.entries, 6);
        assert!(c.attributes >= 8);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_nuget() {
        assert!(!detect(b"<project><name>x</name></project>\n"));
        assert!(!detect(b"key=value\n"));
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

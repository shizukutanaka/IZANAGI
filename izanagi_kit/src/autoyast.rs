//! SUSE AutoYaST `autoinst.xml` 自動インストール定義の認識と計数。
//!
//! AutoYaST は XML で、ルート `<profile xmlns="http://www.suse.com/1.0/yast2ns">`
//! の下に `<general>`/`<networking>`/`<partitioning>`/`<software>`/
//! `<scripts>`/`<users>`/`<groups>`/`<firewall>`/`<bootloader>`/
//! `<add-on>`/`<kdump>`/`<services>`/`<host>`/`<printer>`/`<report>`/
//! `<security>`/`<suse_register>`/`<ldap>`/`<nis>`/`<iscsi-client>` 等の
//! 設定セクションを持つ。`config:type="list"` の繰り返し要素(`<drive>`/
//! `<partition>`/`<user>`/`<package>`/`<script>`)と
//! `config:type="boolean"`/`config:type="integer"`/`config:type="string"`
//! の型注釈、`<![CDATA[…]]>` スクリプト埋め込みが特徴。
//!
//! ```
//! let b = br#"<?xml version="1.0"?>
//! <!DOCTYPE profile>
//! <profile xmlns="http://www.suse.com/1.0/yast2ns" xmlns:config="http://www.suse.com/1.0/configns">
//!   <general>
//!     <mode><confirm config:type="boolean">false</confirm></mode>
//!   </general>
//!   <networking>
//!     <interfaces config:type="list">
//!       <interface><name>eth0</name></interface>
//!     </interfaces>
//!   </networking>
//!   <partitioning config:type="list">
//!     <drive><device>/dev/sda</device></drive>
//!   </partitioning>
//!   <software>
//!     <packages config:type="list"><package>vim</package><package>git</package></packages>
//!   </software>
//!   <scripts>
//!     <post-scripts config:type="list">
//!       <script><filename>setup.sh</filename><source><![CDATA[#!/bin/sh]]></source></script>
//!     </post-scripts>
//!   </scripts>
//! </profile>
//! "#;
//! assert!(izanagi_kit::autoyast::detect(b));
//! let c = izanagi_kit::autoyast::parse(b).unwrap();
//! assert_eq!(c.top_sections, 5); // general/networking/partitioning/software/scripts
//! assert_eq!(c.list_elements, 4); // interfaces/partitioning/packages/post-scripts
//! assert_eq!(c.booleans, 1);
//! assert!(c.cdata >= 1);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<profile>` 直下の設定セクション(`<general>`/`<networking>`/
    /// `<partitioning>`/`<software>`/`<scripts>`/`<users>`/`<firewall>`/
    /// `<bootloader>`/`<add-on>` 等)の個数。
    pub top_sections: usize,
    /// `config:type="list"` 属性を持つ要素の個数。
    pub list_elements: usize,
    /// `config:type="boolean"` の個数。
    pub booleans: usize,
    /// `config:type="integer"`/`config:type="string"`/`config:type="symbol"`
    /// の個数。
    pub typed_elements: usize,
    /// 開始タグ総数(`<name …>`、`</name>`・`<?…?>`・`<!…>` を除く)。
    pub elements: usize,
    /// `<![CDATA[` ブロックの個数。
    pub cdata: usize,
    /// XML コメント `<!--` の個数。
    pub comments: usize,
}

/// AutoYaST らしさを返す。`<profile` + `yast2ns` 名前空間。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<profile") && t.contains("yast2ns")
}

fn tag_name(s: &str) -> &str {
    s.trim_start_matches(['<', '/'])
        .split([' ', '>', '\t', '/'])
        .next()
        .unwrap_or("")
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let mut c = Counts {
        top_sections: 0,
        list_elements: 0,
        booleans: 0,
        typed_elements: 0,
        elements: 0,
        cdata: 0,
        comments: 0,
    };
    let mut depth: i32 = 0;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.contains("<![CDATA[") {
            c.cdata += 1;
        }
        if s.contains("<!--") {
            c.comments += 1;
        }
        // 行内の全タグを走査(複数タグ/行対応)。
        let mut rest = s;
        while let Some(p) = rest.find('<') {
            let tail = &rest[p..];
            let Some(end) = tail.find('>') else {
                break;
            };
            let tag = &tail[..=end];
            rest = &tail[end + 1..];
            if tag.starts_with("<!") || tag.starts_with("<?") {
                continue;
            }
            if tag.starts_with("</") {
                depth -= 1;
                continue;
            }
            let name = tag_name(tag);
            if name.is_empty() {
                continue;
            }
            c.elements += 1;
            depth += 1;
            if tag.ends_with("/>") {
                depth -= 1;
            }
            if depth == 2 && name != "profile" {
                c.top_sections += 1;
            }
            if tag.contains("config:type=\"list\"") || tag.contains("type=\"list\"") {
                c.list_elements += 1;
            }
            if tag.contains("type=\"boolean\"") {
                c.booleans += 1;
            } else if tag.contains("type=\"integer\"")
                || tag.contains("type=\"string\"")
                || tag.contains("type=\"symbol\"")
            {
                c.typed_elements += 1;
            }
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(
            br#"<profile xmlns="http://www.suse.com/1.0/yast2ns"><general/></profile>"#
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"<profile><a/></profile>"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = br#"<profile xmlns="http://www.suse.com/1.0/yast2ns"><software><packages config:type="list"><package>x</package></packages></software><scripts><s config:type="boolean">true</s></scripts></profile>"#;
        let c = parse(b).unwrap();
        assert_eq!(c.top_sections, 2);
        assert_eq!(c.list_elements, 1);
        assert_eq!(c.booleans, 1);
        assert!(c.elements >= 5);
    }

    #[test]
    fn copy_eq() {
        let c = parse(br#"<profile xmlns="http://www.suse.com/1.0/yast2ns"></profile>"#).unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}

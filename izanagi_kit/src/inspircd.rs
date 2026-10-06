//! `inspircd.conf` 検出モジュール。
//!
//! InspIRCd の設定は XML ライクな `<tag attr=...>` ディレクティブの
//! 連続で、単一ルート要素を持たない。`<define>`/`<bind>`/`<class>`/
//! `<connect>`/`<oper>`/`<server>`/`<module>` 等が主要タグ。
//!
//! ```
//! let b = br#"<define name="host" value="irc.example.net">
//! <server name="irc.example.net" description="example">
//! <bind address="0.0.0.0" port="6667" type="clients">
//! <class name="local" recvq="8192">
//! <connect name="local" allow="*">
//! <oper name="admin" password="secret" type="NetAdmin">
//! "#;
//! let c = izanagi_kit::inspircd::parse(b);
//! assert!(izanagi_kit::inspircd::detect(b));
//! assert_eq!(c.directives, 6);
//! ```

const TAGS: &[&str] = &[
    "admin",
    "bind",
    "channel",
    "class",
    "configformat",
    "connect",
    "define",
    "die",
    "disable",
    "files",
    "link",
    "log",
    "module",
    "oper",
    "options",
    "performance",
    "power",
    "server",
    "security",
    "uline",
    "users",
    "whowas",
    "autoconnect",
    "cidr",
    "exempt",
    "insnival",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with("<!--")
}

fn tag_hit(t: &str) -> bool {
    if !t.starts_with('<') {
        return false;
    }
    TAGS.iter().any(|tag| {
        t.starts_with(&format!("<{tag} "))
            || t.starts_with(&format!("<{tag}>"))
            || t.starts_with(&format!("<{tag}/"))
    })
}

/// `b` が inspircd.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut hits = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if tag_hit(tr) {
            hits += 1;
        }
    }
    hits >= 3
}

/// inspircd.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct InspircdConf {
    /// 既知タグ行数。
    pub directives: usize,
    /// `<define>` 行数。
    pub defines: usize,
    /// `<oper>` 行数。
    pub opers: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を inspircd.conf として統計する。
pub fn parse(b: &[u8]) -> InspircdConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = InspircdConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tag_hit(tr) {
            c.directives += 1;
            if tr.starts_with("<define") {
                c.defines += 1;
            } else if tr.starts_with("<oper") {
                c.opers += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<define name="host" value="x">
<bind address="*" port="6667">
<class name="local" recvq="8192">
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 3);
        assert_eq!(c.defines, 1);
    }

    #[test]
    fn detects_full() {
        let b = br#"<server name="irc.example.net">
<connect name="local" allow="*">
<oper name="admin" password="x" type="NetAdmin">
<module name="m_topic.so">
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.opers, 1);
        assert_eq!(c.directives, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"<define name=\"a\" value=\"b\">\n"));
        assert!(!detect(b"<root><a/></root>\n"));
        assert!(!detect(b"<html><body>x</body></html>\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}

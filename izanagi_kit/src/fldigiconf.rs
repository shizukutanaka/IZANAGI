//! `fldigi_def.xml` 検出モジュール。
//!
//! fldigi (デジタルモード通信ソフト) の設定は大文字タグの
//! `<TAG>value</TAG>` 行で構成される。`MODEM`/`PWR`/`AFQUENCY`/
//! `CALL`/`MYLAT`/`TEXT_to_RX`/`XMLRPCLIST` 等が特徴。
//!
//! ```
//! let b = br#"<?xml version="1.0" encoding="UTF-8"?>
//! <FLDIGI_DEFS>
//! <MODEM>8</MODEM>
//! <AFQUENCY>1500</AFQUENCY>
//! <PWR>50</PWR>
//! <CALL>N0CALL</CALL>
//! </FLDIGI_DEFS>
//! "#;
//! let c = izanagi_kit::fldigiconf::parse(b);
//! assert!(izanagi_kit::fldigiconf::detect(b));
//! assert_eq!(c.tag_lines, 5);
//! ```

fn is_tag_line(t: &str) -> bool {
    // <TAGNAME> または <TAGNAME>value — 大文字/数字/_のみのタグ名。
    if !t.starts_with('<') || t.starts_with("<?") || t.starts_with("</") {
        return false;
    }
    let inner = &t[1..];
    let name: String = inner
        .chars()
        .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
        .collect();
    !name.is_empty() && inner.starts_with(&name) && inner[name.len()..].starts_with('>')
}

const REQUIRED_HINTS: &[&str] = &[
    "<MODEM>",
    "<AFQUENCY>",
    "<PWR>",
    "<CALL>",
    "<FLDIGI_DEFS>",
    "<MYLAT>",
    "<MYLON>",
    "<XMLRPCLIST>",
    "<TEXT_to_RX>",
    "<CWDIRECTORY>",
];

/// `b` が fldigi_def.xml に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut tags = 0usize;
    let mut hints = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if is_tag_line(tr) {
            tags += 1;
            if REQUIRED_HINTS
                .iter()
                .any(|h| tr.starts_with(h) || tr.starts_with(&h[..h.len() - 1]))
            {
                hints += 1;
            }
        }
    }
    (hints >= 1 && tags >= 4) || tags >= 8
}

/// fldigi_def.xml の統計。
#[derive(Debug, Default, Clone)]
pub struct FldigiConf {
    /// 大文字タグ行数。
    pub tag_lines: usize,
    /// fldigi 特有タグ行数。
    pub fldigi_tags: usize,
}

/// `b` を fldigi_def.xml として統計する。
pub fn parse(b: &[u8]) -> FldigiConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = FldigiConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if is_tag_line(tr) {
            c.tag_lines += 1;
            if REQUIRED_HINTS
                .iter()
                .any(|h| tr.starts_with(h) || tr.starts_with(&h[..h.len() - 1]))
            {
                c.fldigi_tags += 1;
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
        let b = br#"<FLDIGI_DEFS>
<MODEM>8</MODEM>
<AFQUENCY>1500</AFQUENCY>
<PWR>50</PWR>
<CALL>N0CALL</CALL>
</FLDIGI_DEFS>"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.tag_lines, 5);
    }

    #[test]
    fn detects_many_tags() {
        let b = br#"<TEXT_to_RX>abc</TEXT_to_RX>
<MYLAT>35.6</MYLAT>
<MYLON>139.6</MYLON>
<XMLRPCLIST>srv</XMLRPCLIST>
<BEACON>1</BEACON>"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"<MODEM>1</MODEM>\n<X>1</X>\n<Y>2</Y>"));
        assert!(!detect(b"<html><body>hi</body></html>"));
        assert!(!detect(b"MODEM=8\nPWR=50\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.tag_lines, 0);
    }
}

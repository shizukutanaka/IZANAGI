//! `modsecurity.conf` / CRS設定 検出モジュール。
//!
//! ModSecurity WAF の設定は `SecRule`/`SecAction`/`SecRuleEngine`/
//! `SecRequestBodyAccess`/`SecResponseBodyAccess`/`SecAuditLog`/
//! `SecDebugLog`/`SecMarker`/`SecDefaultAction`/`SecStatusEngine`/
//! `SecUnicodeMapFile` 等の `Sec*` ディレクティブで構成される。
//!
//! ```
//! let b = br#"SecRuleEngine On
//! SecRequestBodyAccess On
//! SecAuditEngine RelevantOnly
//! SecAuditLog /var/log/modsec_audit.log
//! SecRule REQUEST_HEADERS:User-Agent "@streq evil" "id:1,deny,status:403"
//! "#;
//! let c = izanagi_kit::modsecurity::parse(b);
//! assert!(izanagi_kit::modsecurity::detect(b));
//! assert_eq!(c.directives, 5);
//! ```

fn sec_head(t: &str) -> bool {
    // `Sec` で始まり大文字+小文字が続くディレクティブ名。
    if !t.starts_with("Sec") || t.len() < 4 {
        return false;
    }
    let name_end = t.find(|c: char| c.is_whitespace()).unwrap_or(t.len());
    let name = &t[..name_end];
    name.len() > 3
        && name
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c.is_ascii_digit())
        && name[3..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
}

/// `b` が modsecurity.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut dirs = 0usize;
    let mut rules = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if sec_head(tr) {
            dirs += 1;
            if tr.starts_with("SecRule") || tr.starts_with("SecAction") {
                rules += 1;
            }
        }
    }
    (rules >= 1 && dirs >= 2) || dirs >= 4
}

/// modsecurity.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct ModsecurityConf {
    /// `Sec*` ディレクティブ行数。
    pub directives: usize,
    /// `SecRule`/`SecAction` 行数。
    pub rules: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を modsecurity.conf として統計する。
pub fn parse(b: &[u8]) -> ModsecurityConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = ModsecurityConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if sec_head(tr) {
            c.directives += 1;
            if tr.starts_with("SecRule") || tr.starts_with("SecAction") {
                c.rules += 1;
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
        let b = br#"SecRuleEngine On
SecRequestBodyAccess On
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.directives, 2);
    }

    #[test]
    fn detects_with_rule() {
        let b = br#"SecRuleEngine On
SecRule REQUEST_HEADERS:UA "@streq x" "id:1,deny"
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"SecRuleEngine On\n"));
        assert!(!detect(
            b"security = on\nsection = x\nseparate = y\nsecond = z\n"
        ));
        assert!(!detect(b"# SecRuleEngine On\n# SecRule a b\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.directives, 0);
    }
}

//! Quassel `quasselcore.conf`/quasselclient 設定検出モジュール。
//!
//! Quassel の設定は INI 形式で `[Core]`/`[General]` セクションと
//! `Storage`、`Ssl`、`AdminUser`、`Listen` 等のキーが使われる。
//!
//! ```
//! let b = br#"[General]
//! Version=1
//! [Core]
//! Storage=PostgreSQL
//! AdminUser=quassel
//! Listen=0.0.0.0:4242
//! Ssl=true
//! "#;
//! let c = izanagi_kit::quassel::parse(b);
//! assert!(izanagi_kit::quassel::detect(b));
//! assert_eq!(c.sections, 2);
//! ```

const SECTIONS: &[&str] = &[
    "Accounts",
    "Aliases",
    "Auth",
    "Core",
    "Database",
    "General",
    "Identities",
    "Networks",
    "Proxy",
    "Ssl",
    "Storage",
];

const KEYS: &[&str] = &[
    "AdminUser",
    "AuthMethod",
    "CompressionLevel",
    "DataDir",
    "DigestIterations",
    "Host",
    "Listen",
    "Password",
    "Port",
    "Ssl",
    "SslCertificate",
    "SslKey",
    "Storage",
    "StrictIdentEnabled",
    "UseSsl",
    "Version",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#') || t.starts_with(';')
}

fn section_name(t: &str) -> Option<&str> {
    if !(t.starts_with('[') && t.ends_with(']')) {
        return None;
    }
    Some(&t[1..t.len() - 1])
}

fn key_present(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が quasselcore.conf 等に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if let Some(n) = section_name(tr) {
            if SECTIONS.contains(&n) {
                secs += 1;
            }
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 4
}

/// Quassel 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct QuasselConf {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Quassel 設定として統計する。
pub fn parse(b: &[u8]) -> QuasselConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = QuasselConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if let Some(n) = section_name(tr) {
            if SECTIONS.contains(&n) {
                c.sections += 1;
            }
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"[Core]
Storage=PostgreSQL
Listen=0.0.0.0:4242
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
        assert_eq!(c.keys, 2);
    }

    #[test]
    fn detects_general() {
        let b = br#"[General]
Version=1
[Storage]
Host=localhost
Port=5432
[Core]
AdminUser=quassel
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 3);
        assert_eq!(c.keys, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[foo]\nbar=1\n"));
        assert!(!detect(b"key=value\nother=2\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}

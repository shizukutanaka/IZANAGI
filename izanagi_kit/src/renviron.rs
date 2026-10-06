//! `.Renviron` 検出モジュール。
//!
//! R 起動時の環境変数ファイルは `NAME = value`（`#` コメント）の
//! フラットな形式で、`R_LIBS*`/`R_PROFILE_USER`/`R_ENVIRON_USER`/
//! `R_LIBS_USER`/`TZ`/`LANG`/`R_BATCHSAVE` 等が使われる。
//!
//! ```
//! let b = br#"R_LIBS_USER = ~/R/library
//! R_PROFILE_USER = ~/.Rprofile
//! R_ENVIRON_USER = ~/.Renviron
//! LANG = en_US.UTF-8
//! TZ = UTC
//! "#;
//! let c = izanagi_kit::renviron::parse(b);
//! assert!(izanagi_kit::renviron::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "LANG",
    "LC_ALL",
    "LC_COLLATE",
    "LC_CTYPE",
    "LC_MESSAGES",
    "LC_MONETARY",
    "LC_NUMERIC",
    "LC_TIME",
    "MAKEFLAGS",
    "PAGER",
    "R_BATCHSAVE",
    "R_BROWSER",
    "R_CMD",
    "R_LIBS",
    "R_LIBS_SITE",
    "R_LIBS_USER",
    "R_ENVIRON_USER",
    "R_PROFILE_USER",
    "R_HISTORY",
    "R_HOME",
    "R_PAPERSIZE",
    "R_PDFVIEWER",
    "R_PRINTCMD",
    "R_RD4PDF",
    "R_SHARE_DIR",
    "R_TEXI2DVICMD",
    "R_TESTS",
    "R_USER",
    "TZ",
    "TZDIR",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn key_present(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が .Renviron に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut r_keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            keys += 1;
            if tr.starts_with("R_") {
                r_keys += 1;
            }
        }
    }
    keys >= 2 && r_keys >= 1
}

/// .Renviron の統計。
#[derive(Debug, Default, Clone)]
pub struct REnviron {
    /// 既知変数行数。
    pub keys: usize,
    /// 全 `NAME = value` 行数。
    pub assignments: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .Renviron として統計する。
pub fn parse(b: &[u8]) -> REnviron {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = REnviron::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr.contains('=') {
            c.assignments += 1;
            if KEYS.iter().any(|k| key_present(tr, k)) {
                c.keys += 1;
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
        let b = br#"R_LIBS_USER = ~/R/library
LANG = en_US.UTF-8
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 2);
        assert_eq!(c.assignments, 2);
    }

    #[test]
    fn detects_user_vars() {
        let b = br#"R_PROFILE_USER = ~/.Rprofile
R_ENVIRON_USER = ~/.Renviron
TZ = UTC
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"R_LIBS_USER = x\n"));
        assert!(!detect(b"FOO = 1\nBAR = 2\n"));
        assert!(!detect(b"LANG = en_US.UTF-8\nTZ = UTC\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}

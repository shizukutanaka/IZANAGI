//! OfflineIMAP `.offlineimaprc` の検出・カウント。
//!
//! INI 風で `[general]`/`[Account name]`/`[Repository name]`/`[mbnames]`/
//! `[ui.X]` セクションを持ち、Repository 内に `type = Maildir`/`IMAP`/`Gmail`、
//! Account 内に `localrepository`/`remoterepository` を置く。
//!
//! ```
//! let cfg = b"[general]\naccounts = Work\npythonfile = ~/.offlineimap.py\n\n\
//!             [Account Work]\nlocalrepository = Work-Local\nremoterepository = Work-Remote\n\n\
//!             [Repository Work-Local]\ntype = Maildir\nlocalfolders = ~/Mail\n\n\
//!             [Repository Work-Remote]\ntype = IMAP\nremotehost = imap.example.com\n\
//!             remoteuser = me\nssl = yes\ncert_fingerprint = aa11bb22\n";
//! assert!(izanagi_kit::offlineimap::detect(cfg));
//! let c = izanagi_kit::offlineimap::parse(cfg).unwrap();
//! assert_eq!(c.account_sections, 1);
//! assert_eq!(c.repo_sections, 2);
//! ```

/// 既知のセクション接頭辞。
const SECTION_HEADS: &[&str] = &["general", "Account", "Repository", "mbnames", "ui"];

/// 既知キー。
const KNOWN_KEYS: &[&str] = &[
    "accounts",
    "pythonfile",
    "metadata",
    "ui",
    "fsync",
    "maxage",
    "maxsize",
    "maxsyncaccounts",
    "socktimeout",
    "ssl",
    "sslcacertfile",
    "ssl_client_cert",
    "ssl_client_key",
    "ssl_version",
    "start_tls",
    "tls_level",
    "cert_fingerprint",
    "remotehost",
    "remoteport",
    "remoteuser",
    "remotepass",
    "remotepasseval",
    "remotepassfile",
    "remote_identity",
    "folderfilter",
    "folderincludes",
    "nametrans",
    "folderfilter_dynamic",
    "localrepository",
    "remoterepository",
    "status_backend",
    "type",
    "localfolders",
    "readonly",
    "restoreatime",
    "sep",
    "sep_foldernames",
    "startdate",
    "synclabels",
    "labelsheader",
    "labelsfilter",
    "utime_from_header",
    "utime_from_headers",
    "filename_use_mail_timestamp",
    "holdconnectionopen",
    "keepalive",
    "createfolders",
    "sync_deletes",
    "quick",
    "usecompression",
    "ipv6",
    "proxy",
    "preauthtunnel",
    "transport",
    "auth_mechanisms",
    "oauth2_client_id",
    "oauth2_client_secret",
    "oauth2_request_url",
    "oauth2_token_url",
    "oauth2_access_token",
    "oauth2_refresh_token",
    "realdelete",
    "includes",
    "presynchook",
    "postsynchook",
    "maildir-windows-compatible",
    "reference",
    "gmail_mailboxes",
    "singlethreadperfolder",
];

/// セクション名が既知接頭辞に一致するか。
fn section_head(inner: &str) -> Option<&'static str> {
    SECTION_HEADS
        .iter()
        .find(|h| {
            inner == **h
                || inner.starts_with(&format!("{h} "))
                || inner.starts_with(&format!("{h}."))
        })
        .copied()
}

fn key_of(s: &str) -> Option<(&str, &str)> {
    let i = s.find('=')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
    {
        return None;
    }
    Some((k, s[i + 1..].trim()))
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知セクション数。
    pub sections: usize,
    /// `[Account …]` セクション数。
    pub account_sections: usize,
    /// `[Repository …]` セクション数。
    pub repo_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キーの行数。
    pub known_entries: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

/// `b` が `.offlineimaprc` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.account_sections >= 1 || c.repo_sections >= 1) && c.known_entries >= 2
}

/// `b` を `.offlineimaprc` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        account_sections: 0,
        repo_sections: 0,
        entries: 0,
        known_entries: 0,
        comments: 0,
    };
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') {
            let inner = s
                .strip_prefix('[')
                .unwrap_or(s)
                .split(']')
                .next()
                .unwrap_or("")
                .trim();
            match section_head(inner) {
                Some("Account") => {
                    c.sections += 1;
                    c.account_sections += 1;
                }
                Some("Repository") => {
                    c.sections += 1;
                    c.repo_sections += 1;
                }
                Some(_) => c.sections += 1,
                None => {}
            }
            continue;
        }
        if let Some((k, _v)) = key_of(s) {
            c.entries += 1;
            if KNOWN_KEYS.contains(&k) {
                c.known_entries += 1;
            }
        }
    }
    if c.sections == 0 && c.known_entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_offlineimaprc() {
        let cfg = b"[general]\naccounts = Gmail\nui = basic\n\n\
                    [Account Gmail]\nlocalrepository = Gmail-Local\n\
                    remoterepository = Gmail-Remote\npostsynchook = notmuch new\n\n\
                    [Repository Gmail-Local]\ntype = Maildir\nlocalfolders = ~/Mail/Gmail\n\n\
                    [Repository Gmail-Remote]\ntype = Gmail\nremoteuser = me@gmail.com\n\
                    sslcacertfile = /etc/ssl/cert.pem\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.account_sections, 1);
        assert_eq!(c.repo_sections, 2);
    }

    #[test]
    fn rejects_plain_ini() {
        assert!(!detect(
            b"[app]\nname = x\nversion = 1\n[db]\nhost = localhost\n"
        ));
    }
}

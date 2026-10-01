//! msmtp `.msmtprc`/`msmtprc` の検出・カウント。
//!
//! `defaults`/`account name`/`account default : name` のブロック宣言と
//! `host`/`port`/`from`/`user`/`password`/`auth`/`tls` 等のキー行。
//!
//! ```
//! let cfg = b"defaults\nauth on\ntls on\ntls_trust_file /etc/ssl/certs/ca-certificates.crt\n\
//!             logfile ~/.msmtp.log\n\n\
//!             account work\nhost smtp.example.com\nport 587\nfrom me@example.com\n\
//!             user me\npassword secret\n\naccount default : work\n";
//! assert!(izanagi_kit::msmtprc::detect(cfg));
//! let c = izanagi_kit::msmtprc::parse(cfg).unwrap();
//! assert_eq!(c.accounts, 2);
//! assert_eq!(c.tls_entries, 2);
//! assert_eq!(c.password_entries, 2);
//! ```

/// `account name` / `account default : name` の宣言行か。
fn account_kind(s: &str) -> Option<&'static str> {
    let rest = s.strip_prefix("account")?;
    let rest = rest.trim_start();
    if rest.starts_with("default") {
        Some("default")
    } else if rest.is_empty() {
        None
    } else {
        Some("named")
    }
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `account name` 宣言行数 (`account default :` 含む)。
    pub accounts: usize,
    /// `defaults` ブロック行数。
    pub default_blocks: usize,
    /// `key value` 設定行数。
    pub entries: usize,
    /// `tls*`/`tls_*` キー行数。
    pub tls_entries: usize,
    /// `password`/`passwordeval`/`ntlmdomain`/`auth` 等の認証系行数。
    pub password_entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

/// `b` が `.msmtprc` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.accounts + c.default_blocks >= 1 && c.entries >= 3
}

/// `b` を `.msmtprc` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        accounts: 0,
        default_blocks: 0,
        entries: 0,
        tls_entries: 0,
        password_entries: 0,
        comments: 0,
    };
    let mut in_block = false;
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let head = s.split([' ', '\t']).next().unwrap_or("");
        if head == "defaults" {
            c.default_blocks += 1;
            in_block = true;
            continue;
        }
        if head == "account" {
            if account_kind(s).is_some() {
                c.accounts += 1;
                in_block = true;
            }
            continue;
        }
        if !in_block {
            continue;
        }
        if !head.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            continue;
        }
        c.entries += 1;
        if head.starts_with("tls") {
            c.tls_entries += 1;
        }
        if matches!(head, "password" | "passwordeval" | "ntlmdomain" | "auth") {
            c.password_entries += 1;
        }
    }
    if c.entries == 0 && c.accounts == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_msmtprc() {
        let cfg =
            b"defaults\nauth on\ntls on\ntls_trust_file /etc/ssl/certs/ca-certificates.crt\n\n\
                    account gmail\nhost smtp.gmail.com\nport 587\nfrom me@gmail.com\n\
                    user me@gmail.com\npasswordeval \"gpg -d ~/.pass.gpg\"\n\n\
                    account default : gmail\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.accounts, 2);
        assert_eq!(c.default_blocks, 1);
        assert_eq!(c.entries, 8);
        assert_eq!(c.tls_entries, 2);
        assert_eq!(c.password_entries, 2);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[smtp]\nhost = smtp.x\nport = 25\nuser = u\n"));
    }
}

//! `/etc/login.defs` 設定の認識と計数。
//!
//! login.defs は `KEY VALUE` 空白区切り行が主体。キーは `MAIL_DIR`/`PASS_*`/
//! `UID_*`/`GID_*`/`SYS_*`/`SUB_*`/`ENCRYPT_METHOD`/`MD5_CRYPT_ENAB`/
//! `SHA_CRYPT_*`/`UMASK`/`USERGROUPS_ENAB`/`CREATE_HOME`/`SULOG_FILE`/
//! `SU_NAME`/`TTYMODE`/`TTYPERM`/`TTYTYPE_FILE`/`MOTD_FILE`/`HUSHLOGIN_FILE`/
//! `LOGIN_*`/`LASTLOG_ENAB`/`USERDEL_CMD`/`FAKE_SHELL`/`ENCRYPT_METHOD` 等で、
//! 値は絶対パス・`yes`/`no`・数値のいずれか。
//!
//! ```
//! let b = b"MAIL_DIR        /var/mail\nPASS_MAX_DAYS   99999\nPASS_MIN_DAYS   0\nPASS_WARN_AGE   7\nUID_MIN          1000\nUID_MAX         60000\nSYS_UID_MIN       100\nSYS_UID_MAX       999\nGID_MIN          1000\nENCRYPT_METHOD SHA512\nSHA_CRYPT_MIN_ROUNDS 5000\nUMASK           022\nUSERGROUPS_ENAB yes\nCREATE_HOME     yes\nLOGIN_RETRIES   5\n";
//! assert!(izanagi_kit::logindefs::detect(b));
//! let c = izanagi_kit::logindefs::parse(b).unwrap();
//! assert_eq!(c.settings, 15);
//! assert_eq!(c.families, 11); // MAIL/PASS/UID/SYS_UID/GID/ENCRYPT/SHA_CRYPT/UMASK/USERGROUPS/CREATE/LOGIN
//! assert_eq!(c.yesno, 2);
//! assert_eq!(c.paths, 1);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `KEY VALUE` 設定行の個数。
    pub settings: usize,
    /// 先頭トークン(`MAIL`/`PASS`/`UID`/`SYS_UID`/`GID`/`ENCRYPT`/`SHA`/
    /// `UMASK`/`USERGROUPS`/`CREATE`/`LOGIN` 等、`_` 前のファミリ名)の種類数。
    pub families: usize,
    /// `yes`/`no` 値の個数。
    pub yesno: usize,
    /// 絶対パス値の個数。
    pub paths: usize,
    /// 数値値の個数。
    pub numbers: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// login.defs らしさを返す。ALL-CAPS + 値の行が複数あり、典型キーを含む。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    let mut typical = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let mut it = s.split_whitespace();
        let key = it.next().unwrap_or("");
        let val = it.next();
        if key.len() > 1
            && key
                .chars()
                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
            && val.is_some()
        {
            hits += 1;
            for k in [
                "MAIL",
                "PASS",
                "UID",
                "GID",
                "SYS_",
                "ENCRYPT",
                "SHA_CRYPT",
                "MD5",
                "UMASK",
                "USERGROUPS",
                "CREATE_HOME",
                "LOGIN_",
                "LASTLOG",
                "SULOG",
                "SU_",
                "TTY",
                "MOTD",
                "HUSHLOGIN",
                "SUB_",
                "CHFN",
                "DEFAULT_HOME",
                "ERASECHAR",
                "KILLCHAR",
                "NONEXISTENT",
                "USERDEL",
                "FAKE_SHELL",
                "GETTY",
                "MAX_MEMBERS",
                "MINTMDAY",
                "OBSOLETE",
                "PERM_GROUP",
            ] {
                if key.starts_with(k) {
                    typical += 1;
                    break;
                }
            }
        }
    }
    hits >= 4 && typical >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        settings: 0,
        families: 0,
        yesno: 0,
        paths: 0,
        numbers: 0,
        comments: 0,
    };
    let mut fams: std::vec::Vec<&str> = std::vec::Vec::new();
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let mut it = s.split_whitespace();
        let Some(key) = it.next() else {
            continue;
        };
        let Some(val) = it.next() else {
            continue;
        };
        if !key
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
        {
            continue;
        }
        c.settings += 1;
        let fam = key.split('_').next().unwrap_or(key);
        // `SYS_UID`/`SYS_GID`/`SUB_*`/`SHA_CRYPT` は2語目までファミリに含める。
        let fam = match (fam, key) {
            ("SYS", k) | ("SUB", k) if k.contains('_') => {
                &key[..key.rfind('_').unwrap_or(key.len())]
            }
            ("SHA", k) if k.starts_with("SHA_CRYPT") => "SHA_CRYPT",
            _ => fam,
        };
        if !fams.contains(&fam) {
            fams.push(fam);
            c.families += 1;
        }
        if val == "yes" || val == "no" {
            c.yesno += 1;
        } else if val.starts_with('/') {
            c.paths += 1;
        } else if val.chars().all(|x| x.is_ascii_digit()) {
            c.numbers += 1;
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
            b"MAIL_DIR /var/mail\nPASS_MAX_DAYS 9\nUID_MIN 1000\nENCRYPT_METHOD SHA512\n"
        ));
        assert!(detect(
            b"UMASK 022\nUSERGROUPS_ENAB yes\nCREATE_HOME yes\nLOGIN_RETRIES 5\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo = bar\nbaz = qux\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"MAIL_DIR /var/mail\nPASS_MAX_DAYS 9\nSYS_UID_MIN 100\nSYS_UID_MAX 999\nENCRYPT_METHOD SHA512\nCREATE_HOME yes\n";
        let c = parse(b).unwrap();
        assert_eq!(c.settings, 6);
        assert_eq!(c.families, 5); // MAIL/PASS/SYS_UID/ENCRYPT/CREATE
        assert_eq!(c.yesno, 1);
        assert_eq!(c.paths, 1);
        assert_eq!(c.numbers, 3);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"UMASK 022\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

//! PAM スタック設定(`/etc/pam.conf`、`/etc/pam.d/*`)の認識と計数。
//!
//! PAM 設定行は `type control module-path [args…]` の4欄構造:
//! `auth required pam_unix.so`、`session optional pam_limits.so`。
//! type は `account`/`auth`/`password`/`session` のいずれか(先頭 `-` で
//! モジュール欠如時の無視、`!` で拒否を表す)。control は
//! `required`/`requisite`/`sufficient`/`optional`/`include`/`substack`
//! または `[value=action …]` の複雑式(`[success=ok new_authtok_reqd=ok]`)。
//! モジュールは `pam_*.so`・絶対パス・`pam_exec` 等で、`@include` 行は
//! 他ファイルの包含を表す。`/etc/pam.conf` は service 名を先頭に持つ
//! 5欄形式もある。
//!
//! ```
//! let b = b"# pam.d sample\nauth       required     pam_unix.so\nauth       optional     pam_gnome_keyring.so\nauth       [success=1 default=ignore] pam_succeed_if.so uid > 99 quiet\naccount    required     pam_unix.so\naccount    required     pam_nologin.so\npassword   required     pam_unix.so sha512 shadow\nsession    required     pam_limits.so\nsession    optional     pam_systemd.so\nsession    required     pam_unix.so\n@include common-auth\n";
//! assert!(izanagi_kit::pamstack::detect(b));
//! let c = izanagi_kit::pamstack::parse(b).unwrap();
//! assert_eq!(c.lines, 10);
//! assert_eq!(c.types, 4); // auth, account, password, session
//! assert_eq!(c.simple_controls, 8); // required/optional
//! assert_eq!(c.complex_controls, 1); // [success=1 default=ignore]
//! assert_eq!(c.modules, 9);
//! assert!(c.includes >= 1);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 有効設定行の個数(非コメント・非空・`@include` 含む)。
    pub lines: usize,
    /// `account`/`auth`/`password`/`session` タイプの種類数(0–4)。
    pub types: usize,
    /// `required`/`requisite`/`sufficient`/`optional`/`binding` 単純制御の個数。
    pub simple_controls: usize,
    /// `[value=action …]` 複雑制御の個数。
    pub complex_controls: usize,
    /// `include`/`substack`/`@include` 包含の個数。
    pub includes: usize,
    /// `pam_*` モジュール参照(`.so` 終端)の個数。
    pub modules: usize,
    /// 行頭 `-`/`!` フラグ付きエントリの個数。
    pub flagged: usize,
    /// モジュール引数トークンの総数(空白区切り、制御・モジュール以降)。
    pub args: usize,
    /// `#` コメント行の個数。
    pub comments: usize,
}

const TYPES: &[&str] = &["account", "auth", "password", "session"];

const CONTROLS: &[&str] = &["required", "requisite", "sufficient", "optional", "binding"];

/// pam.conf/pam.d らしさを返す。type+control+pam モジュールの行が複数。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let mut it = s.split_whitespace();
        let mut ty = it.next().unwrap_or("");
        if ty.starts_with('-') || ty.starts_with('!') {
            ty = &ty[1..];
        }
        if !TYPES.contains(&ty) {
            continue;
        }
        let ctl = it.next().unwrap_or("");
        if CONTROLS.contains(&ctl) || ctl.starts_with('[') {
            hits += 1;
        }
    }
    hits >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        types: 0,
        simple_controls: 0,
        complex_controls: 0,
        includes: 0,
        modules: 0,
        flagged: 0,
        args: 0,
        comments: 0,
    };
    let mut seen_types: std::vec::Vec<&str> = std::vec::Vec::new();
    for l in t.lines() {
        let s = l.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if let Some(rest) = s.strip_prefix("@include") {
            c.lines += 1;
            c.includes += 1;
            if !rest.trim().is_empty() {
                c.args += 1;
            }
            continue;
        }
        let mut it = s.split_whitespace();
        let Some(head) = it.next() else {
            continue;
        };
        let mut ty = head;
        let mut skip = 0usize; // pam.conf 5欄形式で service を落とす回数
        if head.starts_with('-') || head.starts_with('!') {
            c.flagged += 1;
            ty = &head[1..];
        }
        if !TYPES.contains(&ty) {
            // pam.conf: `service type control module args` — 2トークン目が type。
            let mut it2 = s.split_whitespace();
            let _ = it2.next();
            let Some(t2) = it2.next() else {
                continue;
            };
            let mut t2m = t2;
            if t2.starts_with('-') || t2.starts_with('!') {
                c.flagged += 1;
                t2m = &t2[1..];
            }
            if !TYPES.contains(&t2m) {
                continue;
            }
            ty = t2m;
            skip = 1;
        }
        c.lines += 1;
        if !seen_types.contains(&ty) {
            seen_types.push(ty);
            c.types += 1;
        }
        let mut toks = s.split_whitespace().skip(skip + 1);
        let Some(ctl) = toks.next() else {
            continue;
        };
        if CONTROLS.contains(&ctl) {
            c.simple_controls += 1;
        } else if ctl == "include" || ctl == "substack" {
            c.includes += 1;
        } else if ctl.starts_with('[') {
            c.complex_controls += 1;
        } else {
            continue;
        }
        // `[a=b c=d]` 複雑式は `]` まで読み飛ばす。
        if ctl.starts_with('[') && !ctl.ends_with(']') {
            for t in toks.by_ref() {
                if t.ends_with(']') {
                    break;
                }
            }
        }
        // 残り: module-path + args。
        let rest: std::vec::Vec<&str> = toks.collect();
        if let Some(m) = rest.first() {
            if m.ends_with(".so") || m.contains("pam_") {
                c.modules += 1;
            }
            c.args += rest.len() - 1;
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
            b"auth required pam_unix.so\nsession optional pam_limits.so\n"
        ));
        assert!(detect(
            b"auth [success=1 default=ignore] pam_x.so\naccount required pam_y.so\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo bar baz\nqux quux\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"auth required pam_unix.so\naccount required pam_nologin.so\npassword [success=ok] pam_unix.so sha512\nsession include system-auth\n@include common-auth\n";
        let c = parse(b).unwrap();
        assert_eq!(c.lines, 5);
        assert_eq!(c.types, 4);
        assert_eq!(c.simple_controls, 2);
        assert_eq!(c.complex_controls, 1);
        assert_eq!(c.includes, 2);
        assert_eq!(c.modules, 3);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"auth required pam_unix.so\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }
}

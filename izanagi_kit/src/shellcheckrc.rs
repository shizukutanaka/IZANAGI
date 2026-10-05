//! ShellCheck `.shellcheckrc`/`shellcheckrc` 設定ファイルの検出と構造カウント。
//!
//! `key=value` ディレクティブ(`disable`/`enable`/`source`/`shell`/`severity`/
//! `external-sources`/`allow-nul`/`check-sourced`/`extended-analysis`/
//! `include`/`exclude`/`optional`/`ignore` 等)と `SCNNNN` コードを識別する。
//!
//! ```
//! let c = izanagi_kit::shellcheckrc::parse(
//!     b"disable=SC1090,SC2034\nexternal-sources=true\nshell=bash\n").unwrap();
//! assert_eq!(c.directives, 3);
//! assert_eq!(c.sc_codes, 2);
//! assert!(izanagi_kit::shellcheckrc::detect(b"disable=SC2086\ncheck-sourced\n"));
//! ```

/// 既知ディレクティブキー。
const KEYS: &[&str] = &[
    "allow-nul",
    "check-sourced",
    "disable",
    "enable",
    "exclude",
    "extended-analysis",
    "external-sources",
    "ignore",
    "include",
    "optional",
    "rcfile",
    "requires",
    "severity",
    "shell",
    "source",
];

/// `.shellcheckrc` 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ディレクティブ行(既知キー)。
    pub directives: usize,
    /// `SCNNNN` コード出現数。
    pub sc_codes: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 未知ディレクティブ/分類不能行。
    pub misc: usize,
}

/// 行がディレクティブか(キー部分を返す; `key=value` または裸フラグ)。
fn directive_key(t: &str) -> Option<&str> {
    let end = t.find('=').unwrap_or(t.len());
    let k = t[..end].trim();
    if k.is_empty()
        || !k.chars().all(|c| c.is_ascii_lowercase() || c == '-')
        || k.starts_with('-')
        || k.ends_with('-')
    {
        None
    } else {
        Some(k)
    }
}

/// 行内の `SCNNNN` コード出現数。
fn sc_codes_in(t: &str) -> usize {
    let b = t.as_bytes();
    let mut n = 0;
    let mut i = 0;
    while i + 6 <= b.len() {
        if b[i] == b'S' && b[i + 1] == b'C' && b[i + 2..i + 6].iter().all(|c| c.is_ascii_digit()) {
            n += 1;
            i += 6;
        } else {
            i += 1;
        }
    }
    n
}

/// b が .shellcheckrc かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with('#') && directive_key(t).is_some_and(|k| KEYS.contains(&k))
        })
        .count()
        >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        directives: 0,
        sc_codes: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        // インラインコメントを剥がしてコード走査(行全体で数える)。
        let body = t.split('#').next().map_or(t, |s| s.trim());
        c.sc_codes += sc_codes_in(body);
        if let Some(k) = directive_key(body) {
            if KEYS.contains(&k) {
                c.directives += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.directives >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# shellcheck\nexternal-sources=true\ndisable=SC1090,SC2034 # noisy\nenable=all\nseverity=style\nshell=bash\ncheck-sourced\nsource=/dev/null\n";

    #[test]
    fn shellcheckrc() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.directives, 7);
        assert_eq!(c.sc_codes, 2);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_shellcheckrc() {
        assert!(!detect(b"FOO=bar\nBAZ=qux\n"));
        assert!(!detect(b"[section]\nkey = value\n"));
    }
}

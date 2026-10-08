//! hadolint 設定ファイル(`.hadolint.yaml`/`hadolint.yaml`)の検出と構造カウント。
//!
//! `ignored`/`failure-threshold`/`format`/`strict-labels`/`label-schema`/
//! `disable-ignore-pragma`/`trustedRegistries`/`override` 既知トップキーと
//! `DLNNNN`/`SCNNNN` ルールコードを識別する。
//!
//! ```
//! let c = izanagi_kit::hadolintconf::parse(
//!     b"ignored:\n  - DL3008\n  - SC1010\nfailure-threshold: warning\nstrict-labels: true\n").unwrap();
//! assert_eq!(c.options, 3);
//! assert_eq!(c.codes, 2);
//! assert!(izanagi_kit::hadolintconf::detect(b"ignored:\n  - DL3008\nformat: tty\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &[
    "disable-ignore-pragma",
    "failure-threshold",
    "format",
    "ignored",
    "label-schema",
    "no-color",
    "override",
    "severity-threshold",
    "strict-labels",
    "trustedRegistries",
    "verbose",
];

/// hadolint 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知トップレベルキー + `override:`/サブキー行。
    pub options: usize,
    /// `DLNNNN`/`SCNNNN` ルールコード出現数。
    pub codes: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行(リスト要素・未知キー等)。
    pub misc: usize,
}

/// `key:` 先頭のキー名。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        None
    } else {
        Some(k)
    }
}

/// 行内の `DLNNNN`/`SCNNNN` コード出現数。
fn codes_in(t: &str) -> usize {
    let b = t.as_bytes();
    let mut n = 0;
    let mut i = 0;
    while i + 6 <= b.len() {
        if ((b[i] == b'D' && b[i + 1] == b'L') || (b[i] == b'S' && b[i + 1] == b'C'))
            && b[i + 2..i + 6].iter().all(|c| c.is_ascii_digit())
        {
            n += 1;
            i += 6;
        } else {
            i += 1;
        }
    }
    n
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が hadolint 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && yaml_key(t).is_some_and(|k| TOP_KEYS.contains(&k)) {
            hits += 1;
        } else {
            hits += codes_in(t);
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        options: 0,
        codes: 0,
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
        let indent = line.len() - line.trim_start().len();
        let body = t.strip_prefix('-').map_or(t, |s| s.trim());
        c.codes += codes_in(body);
        if let Some(k) = yaml_key(body) {
            if (indent == 0 && TOP_KEYS.contains(&k)) || indent > 0 {
                // トップ既知キー・ネストされたキー(label-schema 値・override 内
                // severity 等)をオプションとして数える。
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        // 純粋なリスト要素(コード/値)は構造行。
        if !body.is_empty()
            && codes_in(body) == 0
            && !body.chars().all(|ch| {
                ch.is_ascii_alphanumeric() || ch == '.' || ch == '/' || ch == '-' || ch == '_'
            })
        {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# hadolint\nignored:\n  - DL3008\n  - SC1010\nfailure-threshold: warning\nformat: tty\nstrict-labels: true\ndisable-ignore-pragma: hadolint ignore\nlabel-schema:\n  author: text\n  version: semver\ntrustedRegistries:\n  - docker.io\noverride:\n  error:\n    - DL3020\n";

    #[test]
    fn hadolintconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 11);
        assert_eq!(c.codes, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_hadolint() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"ignored:\n  - foo\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

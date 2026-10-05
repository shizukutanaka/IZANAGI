//! Staticcheck 設定ファイル(`staticcheck.conf`)の検出と構造カウント。
//!
//! `key = value` 形式の既知キー(`checks`/`initialisms`/
//! `dot_import_whitelist`/`http_status_code_whitelist`/`inherit`)と
//! `["all", "-ST1000", "SA*"]` 風のチェック指定値を識別する。
//!
//! ```
//! let c = izanagi_kit::staticcheckconf::parse(
//!     b"checks = [\"all\", \"-ST1000\", \"-SA1019\"]\ninitialisms = [\"API\", \"HTTP\"]\n").unwrap();
//! assert_eq!(c.options, 2);
//! assert_eq!(c.checks, 3);
//! assert!(izanagi_kit::staticcheckconf::detect(b"checks = [\"all\"]\ndot_import_whitelist = [\"x\"]\n"));
//! ```

/// 既知キー。
const KEYS: &[&str] = &[
    "checks",
    "deprecated_status_code_whitelist",
    "dot_import_whitelist",
    "http_status_code_whitelist",
    "inherit",
    "initialisms",
];

/// staticcheck.conf 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知 `key = value` 代入。
    pub options: usize,
    /// チェック指定エントリ(`"all"`/`"-XXXXNNNN"`/`"XX*"` 値)。
    pub checks: usize,
    /// `#`/`//` コメント行。
    pub comments: usize,
    /// 分類不能行(配列継続・未知キー等)。
    pub misc: usize,
}

/// `key = value` のキー部分。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find('=')?;
    let k = t[..p].trim();
    if k.is_empty() || !k.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        None
    } else {
        Some(k)
    }
}

/// 値がチェック指定か(`all`/`-XXXX…`/`XX*`/2-3 大文字+数字)。
fn is_check_value(v: &str) -> bool {
    let v = v.trim().trim_matches('"').trim_matches('\'');
    if v == "all" || v == "inherit" {
        return true;
    }
    let v = v.strip_prefix('-').unwrap_or(v);
    let v = v.strip_suffix('*').unwrap_or(v);
    let b = v.as_bytes();
    (2..=7).contains(&b.len())
        && b.iter().take(2).all(|c| c.is_ascii_uppercase())
        && b[2..]
            .iter()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
        && b[2..].iter().any(|c| c.is_ascii_digit())
}

/// 行内の引用符値に含まれるチェック指定数。
fn check_values_in(t: &str) -> usize {
    let bytes = t.as_bytes();
    let mut n = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let q = bytes[i];
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != q {
                j += 1;
            }
            if j > start && is_check_value(&t[start..j]) {
                n += 1;
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    n
}

/// b が staticcheck.conf かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| kv_key(l.trim()).is_some_and(|k| KEYS.contains(&k)))
        .count()
        >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        checks: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        c.checks += check_values_in(t);
        if let Some(k) = kv_key(t) {
            if KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        // 配列継続行(`"-ST1000",` や `]`)は構造行。
        if t != "]" && t != "[" && check_values_in(t) == 0 {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# staticcheck\nchecks = [\"all\", \"-ST1000\", \"-ST1003\", \"-SA4006\"]\ninitialisms = [\"ACL\", \"API\", \"ASCII\", \"CPU\", \"HTTP\", \"HTTPS\"]\ndot_import_whitelist = [\"github.com/mmcloughlin/avo/build\", \"github.com/mmcloughlin/avo/operand\"]\nhttp_status_code_whitelist = [\"200\", \"400\", \"404\", \"500\"]\n";

    #[test]
    fn staticcheckconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 4);
        assert_eq!(c.checks, 4);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn not_staticcheck() {
        assert!(!detect(b"name = \"x\"\nversion = 1\n"));
        assert!(!detect(b"checks = [\"all\"]\n"));
    }
}

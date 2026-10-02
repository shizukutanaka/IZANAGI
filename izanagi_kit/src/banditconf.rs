//! Bandit `.bandit` / `bandit.yaml` 設定の検出と構造カウント。
//!
//! `[bandit]` INI セクション、`tests`/`skips`/`targets`/`exclude_dirs`/
//! `severity`/`confidence`/`profile` キー、`B101`-`B704` テストコードを分類する。
//!
//! ```
//! let c = izanagi_kit::banditconf::parse(
//!     b"[bandit]\ntargets: tests\nskips: B101,B602\nseverity: medium\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::banditconf::detect(b"tests: B101,B102\nskips: B603\n"));
//! ```

/// 既知オプションキー。
const KEYS: &[&str] = &[
    "aggregate",
    "confidence",
    "configfile",
    "context_lines",
    "exclude",
    "exclude_dirs",
    "exit_zero",
    "format",
    "level",
    "log-level",
    "number",
    "output",
    "profile",
    "progress",
    "quiet",
    "recursive",
    "severity",
    "skips",
    "targets",
    "test_ids",
    "tests",
    "verbose",
];
/// テストコード接頭辞(B + 数字3桁)。
fn is_test_code(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 4 && b[0] == b'B' && b[1..].iter().all(|c| c.is_ascii_digit())
}

/// Bandit 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[bandit]` セクション。
    pub sections: usize,
    /// 既知オプション行。
    pub options: usize,
    /// B テストコード参照(値内カンマ区切り)。
    pub test_codes: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行が既知キー行かどうか。
fn known_line(t: &str) -> Option<&str> {
    for sep in ['=', ':'] {
        if let Some(pos) = t.find(sep) {
            let k = t[..pos].trim();
            if KEYS.contains(&k) || k == "bandit" {
                return Some(t[pos + 1..].trim());
            }
        }
    }
    None
}

/// b が bandit 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut keys = 0;
    let mut codes = 0;
    for line in text.lines() {
        let t = line.trim();
        if let Some(v) = known_line(t) {
            keys += 1;
            codes += v.split(',').filter(|s| is_test_code(s.trim())).count();
        } else if t == "[bandit]" {
            keys += 1;
        }
    }
    keys >= 1 && (codes >= 1 || keys >= 2)
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        test_codes: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            if &t[1..t.len() - 1] == "bandit" {
                c.sections += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        if let Some(v) = known_line(t) {
            c.options += 1;
            c.test_codes += v.split(',').filter(|s| is_test_code(s.trim())).count();
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# bandit\n[bandit]\ntargets: tests\ntests: B101,B102,B108\nskips: B603,B404\nexclude_dirs: tests/fixtures\nseverity: medium\nconfidence: high\nrecursive: true\nother = x\n";

    #[test]
    fn banditconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.options, 7);
        assert_eq!(c.test_codes, 5);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_bandit() {
        assert!(!detect(b"[flake8]\nmax-line-length = 100\n"));
        assert!(!detect(b"hello\n"));
    }
}

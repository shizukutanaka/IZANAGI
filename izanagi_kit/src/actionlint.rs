//! actionlint 設定ファイル(`.github/actionlint.yaml`)の検出と構造カウント。
//!
//! `self-hosted-runner`/`config-variables`/`paths` トップキーとその下の
//! `labels`/`ignore-errors`/パスglob マップを識別する。
//!
//! ```
//! let c = izanagi_kit::actionlint::parse(
//!     b"self-hosted-runner:\n  labels:\n    - linux-arm64\nconfig-variables:\n  - ENV\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.options, 1);
//! assert!(izanagi_kit::actionlint::detect(b"paths:\n  '.github/workflows/ci.yaml':\n    ignore-errors:\n      - '.*'\n"));
//! ```

/// 既知トップレベルキー。
const TOP_KEYS: &[&str] = &["config-variables", "paths", "self-hosted-runner"];

/// ネストされた既知サブキー。
const SUB_KEYS: &[&str] = &["ignore-errors", "labels"];

/// actionlint 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルキー行。
    pub sections: usize,
    /// ネストされたキー行(`labels:`/`ignore-errors:`/glob パスマップ)。
    pub options: usize,
    /// `- value` リスト要素。
    pub items: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// `key:` 先頭のキー名(引用符は剥がす)。
fn yaml_key(t: &str) -> Option<&str> {
    let t = t.trim().trim_matches('"').trim_matches('\'');
    let p = t.find(':')?;
    let k = t[..p].trim();
    if k.is_empty() {
        return None;
    }
    Some(k)
}

/// b が actionlint 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if let Some(k) = yaml_key(t) {
            if (indent == 0 && TOP_KEYS.contains(&k)) || (indent > 0 && SUB_KEYS.contains(&k)) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        items: 0,
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
        if t.starts_with('-') {
            c.items += 1;
            continue;
        }
        if let Some(k) = yaml_key(t) {
            if indent == 0 {
                if TOP_KEYS.contains(&k) {
                    c.sections += 1;
                } else {
                    c.misc += 1;
                }
            } else if SUB_KEYS.contains(&k) || t.contains("': {") || t.contains(": {") {
                c.options += 1;
            } else {
                // `paths:` 内の glob キー等は options として数える。
                c.options += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# actionlint\nself-hosted-runner:\n  labels:\n    - linux-arm64\n    - gpu\nconfig-variables:\n  - ENV\n  - REGION\npaths:\n  '.github/workflows/deploy.yaml':\n    ignore-errors:\n      - 'on.push.branches'\n";

    #[test]
    fn actionlint() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.options, 3);
        assert_eq!(c.items, 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_actionlint() {
        assert!(!detect(b"key: value\nother: thing\n"));
        assert!(!detect(b"labels:\n  - x\n"));
    }
}

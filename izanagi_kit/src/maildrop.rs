//! maildrop フィルタスクリプト(maildropfilter)の検出と構造カウント。
//!
//! `if (...)`/`elsif`/`else`・`to`/`cc`/`xfilter` アクション・`foreach`/`while`
//! ループ・`VAR=value` 代入を行頭キーワードで分類する。
//!
//! ```
//! let c = izanagi_kit::maildrop::parse(
//!     b"if (/^From:.*boss/)\n{\n  to \"mail/work\"\n}\n").unwrap();
//! assert_eq!(c.conditions, 1);
//! assert_eq!(c.actions, 1);
//! assert!(izanagi_kit::maildrop::detect(b"to \"$DEFAULT\"\nxfilter \"reformail\"\n"));
//! ```

/// 制御キーワード。
const CONTROLS: &[&str] = &["if", "elsif", "else"];
/// ループキーワード。
const LOOPS: &[&str] = &["foreach", "while"];
/// アクション・組込みコマンド。
const ACTIONS: &[&str] = &[
    "to",
    "cc",
    "bcc",
    "dotlock",
    "echo",
    "exception",
    "exit",
    "flock",
    "getline",
    "import",
    "include",
    "log",
    "logfile",
    "mkdir",
    "rmdir",
    "ungetline",
    "xfilter",
];

/// maildrop 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `if`/`elsif`/`else`。
    pub conditions: usize,
    /// `foreach`/`while`。
    pub loops: usize,
    /// `to`/`cc`/`xfilter`/`exit`/`exception` 等。
    pub actions: usize,
    /// `VAR=value` 代入。
    pub assignments: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// 行が `VAR=value` 代入かどうか。
fn is_assign(t: &str) -> bool {
    let Some(pos) = t.find('=') else {
        return false;
    };
    if t[..pos].ends_with('~')
        || t[..pos].ends_with('!')
        || t[..pos].ends_with('<')
        || t[..pos].ends_with('>')
    {
        return false;
    }
    let key = &t[..pos];
    !key.is_empty()
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !key.contains(' ')
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が maildrop スクリプトかどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut sig = 0;
    for line in text.lines() {
        let t = line.trim();
        let h = t
            .split(|ch: char| ch.is_whitespace() || ch == '(')
            .next()
            .unwrap_or("");
        if ACTIONS.contains(&h)
            || LOOPS.contains(&h)
            || ((h == "if" || h == "elsif") && t.contains('/'))
        {
            sig += 1;
        }
    }
    sig >= 2
}

/// maildrop スクリプトの構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        conditions: 0,
        loops: 0,
        actions: 0,
        assignments: 0,
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
        let h = t
            .split(|ch: char| ch.is_whitespace() || ch == '(')
            .next()
            .unwrap_or("");
        if CONTROLS.contains(&h) {
            c.conditions += 1;
        } else if LOOPS.contains(&h) {
            c.loops += 1;
        } else if ACTIONS.contains(&h) {
            c.actions += 1;
        } else if is_assign(t) {
            c.assignments += 1;
        } else if t == "{" || t == "}" || t.ends_with('{') || t.starts_with('}') {
            // ブロック行は条件側で計上済みか括弧のみ。
        } else {
            c.misc += 1;
        }
    }
    (c.conditions + c.actions + c.assignments >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# maildrop\nVERBOSE=5\nlogfile \"maildrop.log\"\n\nif (/^From:.*boss@work/)\n{\n  to \"mail/work\"\n}\nelsif (/^Subject:.*meeting/)\n{\n  cc \"me@home\"\n}\nelse\n{\n  to \"$DEFAULT\"\n}\n\nforeach /addr /= To\n{\n  log \"found addr\"\n}\nxfilter \"reformail -A'X-Tag: yes'\"\n";

    #[test]
    fn maildrop() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.conditions, 3);
        assert_eq!(c.loops, 1);
        assert_eq!(c.actions, 6);
        assert_eq!(c.assignments, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_maildrop() {
        assert!(!detect(b"key = value\n[section]\n"));
        assert!(!detect(b"hello world\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

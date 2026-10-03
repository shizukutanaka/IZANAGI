//! procmail レシピ(.procmailrc)の検出と構造カウント。
//!
//! `:0` レシピ開始行・`*` 条件行・アクション行・`VAR=value` 代入を分類する。
//!
//! ```
//! let c = izanagi_kit::procmailrc::parse(
//!     b"MAILDIR=$HOME/Mail\n:0\n* ^From: boss@work\nwork/\n").unwrap();
//! assert_eq!(c.recipes, 1);
//! assert!(izanagi_kit::procmailrc::detect(b":0:\n* ^Subject: x\n{ }\n"));
//! ```

/// 既知 procmail 変数名。
const VARS: &[&str] = &[
    "COMSAT",
    "DEFAULT",
    "DELIVERED",
    "DROPPRIVS",
    "EXITCODE",
    "HOST",
    "INCLUDERC",
    "LASTFOLDER",
    "LINEBUF",
    "LOCKEXT",
    "LOCKFILE",
    "LOCKSLEEP",
    "LOCKTIMEOUT",
    "LOG",
    "LOGABSTRACT",
    "LOGFILE",
    "MAILDIR",
    "MATCH",
    "MSGPREFIX",
    "NORESRETC",
    "ORGMAIL",
    "PATH",
    "SENDMAIL",
    "SENDMAILFLAGS",
    "SHELL",
    "SHELLFLAGS",
    "SHELLMETAS",
    "SHIFT",
    "SUSPEND",
    "TRAP",
    "UMASK",
    "VERBOSE",
];

/// procmailrc 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `:0` レシピ開始行。
    pub recipes: usize,
    /// `*` 条件行。
    pub conditions: usize,
    /// アクション行(フォルダ・`|cmd`・`!addr`)。
    pub actions: usize,
    /// `VAR=value` 代入。
    pub assignments: usize,
    /// `{`/`}` ブロック行。
    pub blocks: usize,
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
    let key = &t[..pos];
    !key.is_empty()
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !key.contains(' ')
}

/// b が .procmailrc かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let mut sig = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with(":0") {
            sig += 2;
        } else if t.starts_with('*') && (t.contains('^') || t.contains('?')) {
            sig += 1;
        } else if is_assign(t) {
            let key = &t[..t.find('=').unwrap_or(0)];
            if VARS.contains(&key) {
                sig += 1;
            }
        }
    }
    sig >= 2
}

/// .procmailrc の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        recipes: 0,
        conditions: 0,
        actions: 0,
        assignments: 0,
        blocks: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_recipe = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with(":0") {
            c.recipes += 1;
            in_recipe = true;
            continue;
        }
        if t.starts_with('*') && in_recipe {
            c.conditions += 1;
            continue;
        }
        if t.starts_with('{') || t.starts_with('}') {
            c.blocks += 1;
            continue;
        }
        if is_assign(t) && !in_recipe {
            c.assignments += 1;
            continue;
        }
        if in_recipe {
            c.actions += 1;
            continue;
        }
        if is_assign(t) {
            c.assignments += 1;
            continue;
        }
        c.misc += 1;
    }
    (c.recipes + c.assignments >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"PATH=/bin:/usr/bin\nMAILDIR=$HOME/Mail\nDEFAULT=$MAILDIR/mbox\nLOGFILE=$MAILDIR/log\n\n# filter\n:0:\n* ^From: boss@work.example\n* ^Subject:.*urgent\nwork/\n\n:0\n* ^From:.*@lists.example\n| deliver lists\n\n:0\n{ :0\n  * ^Subject: backup\n  backup/\n}\n";

    #[test]
    fn procmailrc() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.recipes, 3);
        assert_eq!(c.conditions, 4);
        assert_eq!(c.actions, 3);
        assert_eq!(c.assignments, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_procmail() {
        assert!(!detect(b"key = value\n[section]\nkey2 = v2\n"));
        assert!(!detect(b"hello world\n"));
    }
}

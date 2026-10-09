//! AIDE `aide.conf` の検出と構造カウント。
//!
//! `@@define`/`@@ifdef`/`@@ifhost`/`@@endif`/`@@groupadd` ディレクティブ、
//! `database_in`/`database_out`/`database_new`/`verbose`/`report_url` キー、
//! `/path RULE`・`!/path`・`!/pattern` 選択ルール(パス式 + ルール文字)を識別する。
//!
//! ```
//! let c = izanagi_kit::aideconf::parse(
//!     b"@@define DBDIR /var/lib/aide\ndatabase_in = file:@@{DBDIR}/aide.db\nverbose = 5\n/etc p+i+n+g\n!/var/log\n").unwrap();
//! assert!(c.entries >= 2);
//! assert!(izanagi_kit::aideconf::detect(
//!     b"database_in = file:/x/aide.db\n/etc p+i+n+g\n"));
//! ```

use crate::textutil::strip_bom;
/// `@@` ディレクティブ(一致は先頭走査)。
const DIRECTIVES: &[&str] = &[
    "define", "endif", "groupadd", "ifhost", "ifnhost", "ifdef", "ifndef",
];

/// トップレベル既知キー。
const KEYS: &[&str] = &[
    "acl_no_symlinks",
    "attrs",
    "compression_level",
    "database_add_metadata",
    "database_attrs",
    "database_in",
    "database_new",
    "database_out",
    "gzip_dbout",
    "ignore_list",
    "log_level",
    "num_workers",
    "report_attrs",
    "report_append",
    "report_base16",
    "report_detailed_init",
    "report_force_attrs",
    "report_grouped",
    "report_ignore_added_attrs",
    "report_ignore_changed_attrs",
    "report_ignore_e2fsattrs",
    "report_ignore_removed_attrs",
    "report_level",
    "report_limit",
    "report_prefix",
    "report_summarize_changes",
    "report_syslog",
    "report_url",
    "root_prefix",
    "summarize_changes",
    "verbose",
    "warn_dead_symlinks",
    "xacl_set",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `@@` ディレクティブ行数。
    pub sections: usize,
    /// `/path RULE`・`!/path` 選択ルール行数。
    pub entries: usize,
    /// `key = value` 既知キー行数。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn is_directive(t: &str) -> bool {
    if !t.starts_with("@@") {
        return false;
    }
    let w = t[2..]
        .split(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("");
    DIRECTIVES.contains(&w)
}

fn rule_only(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, '+' | '-'))
        && s.bytes().any(|b| b.is_ascii_alphabetic())
}

fn is_path_rule(t: &str) -> bool {
    // `/path RULE` または `!/path` / `!/pattern[...]` 形。
    let (path, rest) = if let Some(r) = t.strip_prefix('!') {
        (r, None::<&str>)
    } else if t.starts_with('/') {
        let mut it = t.splitn(2, ' ');
        (it.next().unwrap_or(""), it.next())
    } else {
        return false;
    };
    if !path.starts_with('/') || path.contains('=') {
        return false;
    }
    match rest {
        None => true, // `!/path` 単体
        Some(r) => rule_only(r.trim().split(' ').next().unwrap_or("")),
    }
}

/// `aide.conf` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut keys = 0usize;
    let mut rules = 0usize;
    let mut dirs = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if is_directive(t) {
            dirs += 1;
        }
        if is_path_rule(t) {
            rules += 1;
        }
        if let Some(eq) = t.find('=') {
            let k = t[..eq].trim();
            if KEYS.contains(&k) || k.starts_with("@@") {
                keys += 1;
            }
        }
        if (keys + rules + dirs) >= 3 && rules >= 1 {
            return true;
        }
    }
    rules >= 1 && (keys + dirs) >= 1
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        entries: 0,
        options: 0,
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
        if is_directive(t) {
            c.sections += 1;
        } else if is_path_rule(t) {
            c.entries += 1;
        } else if let Some(eq) = t.find('=') {
            let k = t[..eq].trim();
            if KEYS.contains(&k) || k.starts_with("@@") {
                c.options += 1;
            } else {
                c.misc += 1;
            }
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# aide.conf\n@@define DBDIR /var/lib/aide\n@@define LOGDIR /var/log\ndatabase_in = file:@@{DBDIR}/aide.db\ndatabase_out = file:@@{DBDIR}/aide.db.new\nverbose = 5\nreport_url = file:@@{LOGDIR}/aide.log\n\n/etc p+i+n+g+md5\n/bin p+i+n+g+s\n/root i+n+g\n!/etc/mtab\n!/proc\n!/sys\n";

    #[test]
    fn aideconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.entries, 6);
        assert_eq!(c.options, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_aideconf() {
        assert!(!detect(b"key = value\n"));
        assert!(parse(b"text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

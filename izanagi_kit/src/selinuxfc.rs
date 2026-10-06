//! SELinux `.fc` (file contexts) 検出モジュール。
//!
//! .fc ファイルは各行が `<path_regex> [<type>] <context>` 形式で、
//! 型フラグは `--`(通常ファイル)/`-d`(dir)/`-l`(symlink)/`-s`(sock)/
//! `-c`(char)/`-b`(block)/`-p`(pipe)/`-e`(all) で、コンテキストは
//! `system_u:object_r:foo_t:s0` または `gen_context(system_u:
//! object_r:foo_t,s0)`/`<<none>>`。
//!
//! ```
//! let b = b"/usr/bin/myapp -- gen_context(system_u:object_r:myapp_exec_t,s0)\n\
//!           /usr/sbin/myappd -- gen_context(system_u:object_r:myapp_exec_t,s0)\n\
//!           /var/lib/myapp(/.*)? gen_context(system_u:object_r:myapp_var_t,s0)\n\
//!           /run/myapp.pid -- gen_context(system_u:object_r:myapp_var_run_t,s0)\n";
//! let c = izanagi_kit::selinuxfc::parse(b);
//! assert!(izanagi_kit::selinuxfc::detect(b));
//! assert_eq!(c.entries, 4);
//! ```

const FLAGS: &[char] = &['-', 'd', 'l', 's', 'c', 'b', 'p', 'e'];

fn is_flag(t: &str) -> bool {
    t == "--" || (t.len() == 2 && t.starts_with('-') && FLAGS.contains(&(t.as_bytes()[1] as char)))
}

fn looks_ctx(t: &str) -> bool {
    t.contains("gen_context(") || t.contains(":object_r:") || t == "<<none>>"
}

fn is_entry(l: &str) -> bool {
    let mut it = l.split_whitespace();
    let Some(path) = it.next() else {
        return false;
    };
    if !path.starts_with('/') {
        return false;
    }
    let mut saw_ctx = false;
    let mut rest_ok = true;
    for tok in it.by_ref() {
        if is_flag(tok) || looks_ctx(tok) {
            if looks_ctx(tok) {
                saw_ctx = true;
            }
        } else {
            rest_ok = false;
            break;
        }
    }
    saw_ctx && rest_ok
}

/// `b` が .fc ファイルに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut entries = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_entry(tr) {
            entries += 1;
        }
    }
    entries >= 2
}

/// .fc ファイルの統計。
#[derive(Debug, Default, Clone)]
pub struct SelinuxFc {
    /// エントリ行数。
    pub entries: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を .fc ファイルとして統計する。
pub fn parse(b: &[u8]) -> SelinuxFc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = SelinuxFc::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_entry(tr) {
            c.entries += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"/a/b -- system_u:object_r:x_t:s0\n/c/d gen_context(u:object_r:y_t,s0)\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.entries, 2);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"/a/b -- system_u:object_r:x_t:s0\n"));
        assert!(!detect(b"/usr/bin/foo\n/etc/x\n/var/y\n"));
        assert!(!detect(b"key=value\nfoo=bar\n"));
    }

    #[test]
    fn comment_lines_ignored() {
        let b = b"# /a/b -- system_u:object_r:x_t:s0\n";
        assert!(!detect(b));
        let c = parse(b);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.entries, 0);
    }
}

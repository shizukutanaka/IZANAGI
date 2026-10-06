//! `.bt` (bpftrace プログラム) 検出モジュール。
//!
//! bpftrace スクリプトは `BEGIN`/`END`/`tracepoint:`/`kprobe:`/
//! `uprobe:`/`interval:`/`hardware:`/`software:`/`watchpoint:` 等の
//! プローブ指定子と `printf`/`@x`/`count()`/`hist()` 関数を持つ。
//!
//! ```
//! let b = br#"BEGIN
//! {
//!     printf("Tracing open syscalls... Hit Ctrl-C to end.\n");
//! }
//! tracepoint:syscalls:sys_enter_openat
//! {
//!     printf("%s %s\n", comm, str(args.filename));
//! }
//! interval:s:1
//! {
//!     exit();
//! }
//! "#;
//! let c = izanagi_kit::bpftrace::parse(b);
//! assert!(izanagi_kit::bpftrace::detect(b));
//! assert_eq!(c.probes, 3);
//! ```

const PROBES: &[&str] = &[
    "BEGIN",
    "END",
    "hardware:",
    "interval:",
    "iter:",
    "kfunc:",
    "kprobe:",
    "kretprobe:",
    "profile:",
    "rawtracepoint:",
    "software:",
    "tracepoint:",
    "uprobe:",
    "uretprobe:",
    "usdt:",
    "watchpoint:",
];

fn is_comment(t: &str) -> bool {
    t.starts_with("//") || t.starts_with('#') || t.starts_with("/*")
}

fn probe_hit(t: &str) -> bool {
    PROBES.iter().any(|p| {
        if p.ends_with(':') {
            t.starts_with(p)
        } else {
            t == *p || t.starts_with(&format!("{p} ")) || t.starts_with(&format!("{p}:"))
        }
    })
}

/// `b` が bpftrace プログラムに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut probes = 0usize;
    let mut braces = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if probe_hit(tr) {
            probes += 1;
        } else if tr == "{" || tr == "}" {
            braces += 1;
        }
    }
    probes >= 2 && braces >= 2
}

/// bpftrace プログラムの統計。
#[derive(Debug, Default, Clone)]
pub struct BpftraceProg {
    /// プローブ指定子行数。
    pub probes: usize,
    /// `{`/`}` 行数。
    pub braces: usize,
    /// `printf`/`print`/`exit`/`clear`/`zero` 呼出行数。
    pub builtin_calls: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を bpftrace プログラムとして統計する。
pub fn parse(b: &[u8]) -> BpftraceProg {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = BpftraceProg::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if probe_hit(tr) {
            c.probes += 1;
            continue;
        }
        if tr == "{" || tr == "}" {
            c.braces += 1;
            continue;
        }
        if tr.starts_with("printf")
            || tr.starts_with("print")
            || tr.starts_with("exit")
            || tr.starts_with("clear")
            || tr.starts_with("zero")
            || tr.starts_with("time")
        {
            c.builtin_calls += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"BEGIN
{
    printf("hi\n");
}
END
{
    exit();
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.probes, 2);
        assert_eq!(c.braces, 4);
    }

    #[test]
    fn detects_probes() {
        let b = br#"kprobe:vfs_read
{
    @reads = count();
}
interval:s:10
{
    exit();
}
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"BEGIN\n{\nx\n}\n"));
        assert!(!detect(b"kprobe:x\n"));
        assert!(!detect(b"{ }\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.probes, 0);
    }
}

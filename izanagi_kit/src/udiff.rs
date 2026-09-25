//! Unified diff text — the wire form for [`diff`](crate::diff):
//! `--- old` / `+++ new` headers, `@@ -a[,b] +c[,d] @@` hunks of
//! ` `/`-`/`+` lines, and `\ No newline at end of file` markers.
//! [`apply`] applies a [`FilePatch`] to a slice of lines: every
//! context/deletion line must match byte-for-byte, else `None`.
//! [`emit`] writes canonical form (count `,1` elided).
//!
//! ```
//! use izanagi_kit::udiff::{parse, apply};
//! let p = parse("--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n a\n-b\n+c\n c\n").unwrap();
//! let old = vec!["a".to_string(), "b".to_string(), "c".to_string()];
//! assert_eq!(apply(&old, &p.files[0]).unwrap(), vec!["a", "c", "c"]);
//! ```

use std::string::String;
use std::vec::Vec;

/// A hunk line.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// Context (starts with `' '` in the file).
    Ctx(String),
    /// Deletion (`-`).
    Del(String),
    /// Addition (`+`).
    Add(String),
}

/// One `@@` hunk.
#[derive(Clone, Debug, PartialEq)]
pub struct Hunk {
    /// First line of the old range (1-based).
    pub old_start: u32,
    /// Lines consumed from the old file.
    pub old_len: u32,
    /// First line of the new range (1-based).
    pub new_start: u32,
    /// Lines produced in the new file.
    pub new_len: u32,
    /// Hunk body in order.
    pub lines: Vec<Line>,
}

/// Per-file patch.
#[derive(Clone, Debug, PartialEq)]
pub struct FilePatch {
    /// Path after `--- ` (e.g. `a/src/x.rs`).
    pub old: String,
    /// Path after `+++ `.
    pub new: String,
    /// Hunks in order.
    pub hunks: Vec<Hunk>,
}

/// A whole diff.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    /// Files in order.
    pub files: Vec<FilePatch>,
}

fn u32_of(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// `a[,b]` → `(start, len)` with `len` defaulting to 1.
fn range(s: &str) -> Option<(u32, u32)> {
    match s.split_once(',') {
        Some((a, b)) => Some((u32_of(a)?, u32_of(b)?)),
        None => Some((u32_of(s)?, 1)),
    }
}

/// Parse a unified diff; `None` on malformed headers, hunk headers,
/// short bodies, or body lines that don't start with ` `, `-`, `+`,
/// or `\`. Multiple files may share one `Patch`.
pub fn parse(src: &str) -> Option<Patch> {
    let src = src.replace("\r\n", "\n");
    let mut it = src.split('\n').peekable();
    let mut patch = Patch::default();
    loop {
        // skip to next `--- ` header
        let mut line = loop {
            match it.next() {
                Some(l) => {
                    if l.starts_with("--- ") {
                        break l;
                    }
                }
                None => return Some(patch),
            }
        };
        let old = line[4..].trim_end().to_string();
        line = it.next()?;
        let new = line.strip_prefix("+++ ")?.trim_end().to_string();
        let mut fp = FilePatch {
            old,
            new,
            hunks: Vec::new(),
        };
        // hunks until next `---` or EOF
        while let Some(&l) = it.peek() {
            if l.starts_with("@@ ") || l.starts_with("@@-") {
                it.next();
                let hdr = l.strip_prefix("@@ ").unwrap_or(l);
                // "-a[,b] +c[,d] @@[ ctx]"
                let (neg, rest) = hdr.split_once(' ')?;
                let (pos, _ctx) = rest.rsplit_once("@@")?;
                let (old_r, new_r) = (neg.strip_prefix('-')?, pos.trim().strip_prefix('+')?);
                let (os, ol) = range(old_r)?;
                let (ns, nl) = range(new_r)?;
                let mut h = Hunk {
                    old_start: os,
                    old_len: ol,
                    new_start: ns,
                    new_len: nl,
                    lines: Vec::new(),
                };
                let (mut got_o, mut got_n) = (0u32, 0u32);
                while got_o < ol || got_n < nl {
                    let body = it.next()?;
                    match body.as_bytes().first() {
                        Some(b' ') => {
                            h.lines.push(Line::Ctx(body[1..].to_string()));
                            got_o += 1;
                            got_n += 1;
                        }
                        Some(b'-') => {
                            h.lines.push(Line::Del(body[1..].to_string()));
                            got_o += 1;
                        }
                        Some(b'+') => {
                            h.lines.push(Line::Add(body[1..].to_string()));
                            got_n += 1;
                        }
                        Some(b'\\') => {} // "\ No newline at end of file"
                        _ => return None,
                    }
                }
                // optional \ marker line right after
                if let Some(&m) = it.peek() {
                    if m.starts_with('\\') {
                        it.next();
                    }
                }
                if got_o != ol || got_n != nl {
                    return None;
                }
                fp.hunks.push(h);
            } else if l.starts_with("--- ") {
                break;
            } else {
                // stray content between hunks (blank lines, context, ...)
                it.next();
            }
        }
        patch.files.push(fp);
    }
}

fn emit_range(start: u32, len: u32, s: &mut String) {
    s.push_str(&std::format!("{start}"));
    if len != 1 {
        s.push_str(&std::format!(",{len}"));
    }
}

/// Canonical emission.
pub fn emit(p: &Patch) -> String {
    let mut s = String::new();
    for f in &p.files {
        s.push_str(&std::format!("--- {}\n+++ {}\n", f.old, f.new));
        for h in &f.hunks {
            s.push_str("@@ -");
            emit_range(h.old_start, h.old_len, &mut s);
            s.push_str(" +");
            emit_range(h.new_start, h.new_len, &mut s);
            s.push_str(" @@\n");
            for l in &h.lines {
                s.push(match l {
                    Line::Ctx(_) => ' ',
                    Line::Del(_) => '-',
                    Line::Add(_) => '+',
                });
                s.push_str(match l {
                    Line::Ctx(t) | Line::Del(t) | Line::Add(t) => t,
                });
                s.push('\n');
            }
        }
    }
    s
}

/// Apply one file patch to `old` (a slice of lines). Context and
/// deletions must match exactly; `None` on any mismatch or overrun.
/// The `\ No newline` marker is semantic-free here — lines are whole.
pub fn apply(old: &[String], fp: &FilePatch) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::with_capacity(old.len());
    let mut cursor = 0usize;
    for h in &fp.hunks {
        let start = if h.old_len == 0 {
            h.old_start as usize // insertion: position is 0-based-after
        } else {
            (h.old_start as usize).checked_sub(1)?
        };
        if start > old.len() {
            return None;
        }
        out.extend_from_slice(&old[cursor..start.min(old.len()).max(cursor)]);
        cursor = start;
        for l in &h.lines {
            match l {
                Line::Ctx(t) | Line::Del(t) => {
                    if old.get(cursor)? != t {
                        return None;
                    }
                    if let Line::Ctx(_) = l {
                        out.push(t.clone());
                    }
                    cursor += 1;
                }
                Line::Add(t) => out.push(t.clone()),
            }
        }
    }
    out.extend_from_slice(&old[cursor.min(old.len())..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &str = "--- a/f.txt\n+++ b/f.txt\n@@ -1,4 +1,4 @@\n a\n-b\n+c\n c\n d\n@@ -7,2 +7,3 @@\n x\n+z\n y\n";

    #[test]
    fn parse_basic() {
        let p = parse(D).unwrap();
        assert_eq!(p.files.len(), 1);
        let f = &p.files[0];
        assert_eq!(f.old, "a/f.txt");
        assert_eq!(f.new, "b/f.txt");
        assert_eq!(f.hunks.len(), 2);
        assert_eq!((f.hunks[0].old_start, f.hunks[0].old_len), (1, 4));
        assert_eq!(f.hunks[0].lines[1], Line::Del("b".into()));
    }

    #[test]
    fn apply_works() {
        let p = parse(D).unwrap();
        let old: Vec<String> = ["a", "b", "c", "d", "e", "f", "x", "y"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let new = apply(&old, &p.files[0]).unwrap();
        assert_eq!(new, vec!["a", "c", "c", "d", "e", "f", "x", "z", "y"]);
    }

    #[test]
    fn apply_insert_and_delete_only() {
        // pure insertion at start: GNU emits -0,0
        let p = parse("--- a\n+++ b\n@@ -0,0 +1,2 @@\n+x\n+y\n").unwrap();
        let old: Vec<String> = vec!["a".into()];
        assert_eq!(apply(&old, &p.files[0]).unwrap(), vec!["x", "y", "a"]);
        // pure delete
        let p = parse("--- a\n+++ b\n@@ -1,2 +1,0 @@\n-a\n-b\n").unwrap();
        let old: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(apply(&old, &p.files[0]).unwrap(), vec!["c"]);
    }

    #[test]
    fn apply_mismatch_rejected() {
        let p = parse("--- a\n+++ b\n@@ -1 +1 @@\n-expected\n+x\n").unwrap();
        let old: Vec<String> = vec!["different".into()];
        assert_eq!(apply(&old, &p.files[0]), None);
    }

    #[test]
    fn no_newline_marker() {
        let p = parse("--- a\n+++ b\n@@ -1 +1 @@\n-a\n\\ No newline at end of file\n+b\n").unwrap();
        let f = &p.files[0].hunks[0];
        assert_eq!(f.lines.len(), 2);
    }

    #[test]
    fn emit_roundtrip() {
        let p = parse(D).unwrap();
        assert_eq!(parse(&emit(&p)).unwrap(), p);
    }

    #[test]
    fn malformed_rejected() {
        for bad in [
            "--- a\n", // missing +++
            "--- a\n+++ b\n@@ bad @@\n",
            "--- a\n+++ b\n@@ -1 +1 @@\n?junk\n",
            "--- a\n+++ b\n@@ -1,3 +1,3 @@\n a\n", // short body
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
        assert_eq!(parse(""), Some(Patch::default()));
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(D), parse(D));
        assert_eq!(emit(&parse(D).unwrap()), emit(&parse(D).unwrap()));
    }
}

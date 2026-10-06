//! Valgrind suppression ファイル検出モジュール。
//!
//! `.supp` ファイルは `{ <name> / <tool>:<kind> / <stack frames> }`
//! ブロックで構成され、`Memcheck:`/`fun:`/`obj:`/`match-leak-kinds:`
//! 等のディレクティブが特徴。
//!
//! ```
//! let b = br#"{
//!    libc-memmove
//!    Memcheck:Addr8
//!    fun:memmove
//!    fun:something
//! }
//! {
//!    zlib-leak
//!    Memcheck:Leak
//!    match-leak-kinds: definite
//!    fun:malloc
//!    obj:*/libz.so
//! }
//! "#;
//! let c = izanagi_kit::valgrindsupp::parse(b);
//! assert!(izanagi_kit::valgrindsupp::detect(b));
//! assert_eq!(c.blocks, 2);
//! ```

const DIRECTIVES: &[&str] = &[
    "Memcheck:",
    "Helgrind:",
    "DRD:",
    "Massif:",
    "SGCheck:",
    "match-leak-kinds:",
    "fun:",
    "functor:",
    "obj:",
    "called-from:",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn dir_hit(t: &str) -> bool {
    DIRECTIVES.iter().any(|d| t.starts_with(d))
}

/// `b` が Valgrind suppression ファイルに見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut braces = 0usize;
    let mut dirs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if tr == "{" || tr == "}" {
            braces += 1;
        } else if dir_hit(tr) {
            dirs += 1;
        }
    }
    (braces >= 2 && dirs >= 1) || dirs >= 4
}

/// Valgrind suppression ファイルの統計。
#[derive(Debug, Default, Clone)]
pub struct ValgrindSupp {
    /// `{`/`}` ブロック行数の半分。
    pub blocks: usize,
    /// `Memcheck:`/`fun:`/`obj:` 等ディレクティブ行数。
    pub directives: usize,
    /// suppression 名行（その他の非空行）数。
    pub names: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を suppression ファイルとして統計する。
pub fn parse(b: &[u8]) -> ValgrindSupp {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = ValgrindSupp::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr == "{" {
            continue;
        }
        if tr == "}" {
            c.blocks += 1;
            continue;
        }
        if dir_hit(tr) {
            c.directives += 1;
        } else {
            c.names += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"{
   libc-memmove
   Memcheck:Addr8
   fun:memmove
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.blocks, 1);
        assert_eq!(c.directives, 2);
        assert_eq!(c.names, 1);
    }

    #[test]
    fn detects_leaks() {
        let b = br#"{
   z-leak
   Memcheck:Leak
   match-leak-kinds: definite
   fun:malloc
}
{
   z-leak2
   Memcheck:Leak
   match-leak-kinds: possible
   obj:*/libz.so
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.blocks, 2);
        assert_eq!(c.directives, 6);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\nfoo\n}\n"));
        assert!(!detect(b"Memcheck:Addr8\n"));
        assert!(!detect(b"key=value\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.blocks, 0);
    }
}

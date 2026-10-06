//! TON FunC smart-contract source (`.fc`/`.func`) parser.
//!
//! Detects FunC by `recv_internal(`/`recv_external(`/`run_ticktock`/
//! `method_id`/`impure` markers or `#include` + `;`-style declarations,
//! and counts functions, specifiers, globals, throw helpers and `;;`
//! comments.
//!
//! ```
//! use izanagi_kit::func::Func;
//! let src = b"() recv_internal(int msg_value, cell in_msg, slice in_msg_body) impure {\n}\n";
//! assert!(izanagi_kit::func::detect(src));
//! let f = Func::parse(src).unwrap();
//! assert_eq!(f.recv_internals, 1);
//! assert_eq!(f.functions, 1);
//! ```

/// Parsed census of a FunC source file.
#[derive(Debug, Clone)]
pub struct Func {
    /// `#include` directives.
    pub includes: usize,
    /// Function definitions (`(params) name` or `name (params)` + `{`).
    pub functions: usize,
    /// `recv_internal` entries.
    pub recv_internals: usize,
    /// `recv_external` entries.
    pub recv_externals: usize,
    /// `run_ticktock` entries.
    pub run_ticktocks: usize,
    /// `method_id` specifiers.
    pub method_ids: usize,
    /// `impure` specifiers.
    pub impures: usize,
    /// `inline`/`inline_ref` specifiers.
    pub inlines: usize,
    /// `forall` polymorphic signatures.
    pub foralls: usize,
    /// `global` declarations.
    pub globals: usize,
    /// `throw`/`throw_if`/`throw_unless`/`throw_arg` calls.
    pub throws: usize,
    /// `;;` comment lines.
    pub comments: usize,
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}
fn code_has(t: &str, needle: &str) -> bool {
    // `;;` コメント行内の言及は証拠にしない。
    t.lines()
        .any(|l| !l.trim_start().starts_with(";;") && l.contains(needle))
}

/// Returns `true` when `b` looks like FunC source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    code_has(t, "recv_internal(")
        || code_has(t, "recv_external(")
        || code_has(t, "run_ticktock(")
        || code_has(t, "method_id")
        || (code_has(t, "#include") && code_has(t, "impure"))
}

impl Func {
    /// Counts FunC constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).ok()?;
        let mut f = Self {
            includes: 0,
            functions: 0,
            recv_internals: t.matches("recv_internal(").count(),
            recv_externals: t.matches("recv_external(").count(),
            run_ticktocks: t.matches("run_ticktock(").count(),
            method_ids: t.matches("method_id").count(),
            impures: 0,
            inlines: 0,
            foralls: 0,
            globals: 0,
            throws: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim_start();
            if l.starts_with("#include") {
                f.includes += 1;
            }
            if l.starts_with(";;") {
                f.comments += 1;
            }
            // Heuristic function header: line contains `)` then `{`.
            if l.contains(')') && l.ends_with('{') && !l.starts_with(";;") {
                f.functions += 1;
            }
        }
        for w in words(t) {
            match w {
                "impure" => f.impures += 1,
                "inline" | "inline_ref" => f.inlines += 1,
                "forall" => f.foralls += 1,
                "global" => f.globals += 1,
                "throw" | "throw_if" | "throw_unless" | "throw_arg" | "throw_arg_if"
                | "throw_arg_unless" => f.throws += 1,
                _ => {}
            }
        }
        Some(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b";; recv_internal()\n"));
    }

    const SRC: &[u8] =
        b"() recv_internal(int msg_value, cell in_msg, slice in_msg_body) impure {\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"int f() method_id { }"));
        assert!(detect(b"#include \"a.fc\";\nint g() impure { }"));
        assert!(!detect(b"int f() { }"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let f = Func::parse(SRC).unwrap();
        assert_eq!(f.recv_internals, 1);
        assert_eq!(f.functions, 1);
        assert_eq!(f.impures, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"#include \"x.fc\";\nglobal int g;\n() recv_external(slice in_msg) impure inline {\n    throw_if(1, 1);\n}\nforall X -> int h(X x) method_id { }\n;; c\n";
        let f = Func::parse(s).unwrap();
        assert_eq!(f.includes, 1);
        assert_eq!(f.globals, 1);
        assert_eq!(f.recv_externals, 1);
        assert_eq!(f.impures, 1);
        assert_eq!(f.inlines, 1);
        assert_eq!(f.foralls, 1);
        assert_eq!(f.method_ids, 1);
        assert!(f.throws >= 1);
        assert_eq!(f.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(Func::parse(b"plain text").is_none());
        assert!(Func::parse(b"").is_none());
    }
}

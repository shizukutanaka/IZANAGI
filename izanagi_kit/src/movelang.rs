//! Move smart-contract language source (`.move`) parser.
//!
//! Detects Move by `module <addr>::<name> {` / `script {` blocks or
//! `public fun`/`entry fun` signatures, and counts functions, structs,
//! `use`/`friend`/`acquires`, abilities and `//`/`///` comments.
//!
//! ```
//! use izanagi_kit::movelang::Move;
//! let src = b"module 0x1::coin {\n    public fun mint() {}\n}\n";
//! assert!(izanagi_kit::movelang::detect(src));
//! let m = Move::parse(src).unwrap();
//! assert_eq!(m.modules, 1);
//! assert_eq!(m.functions, 1);
//! ```

/// Parsed census of a Move source file.
#[derive(Debug, Clone)]
pub struct Move {
    /// `module` declarations.
    pub modules: usize,
    /// `script` blocks.
    pub scripts: usize,
    /// `fun` definitions.
    pub functions: usize,
    /// `public fun` visibility.
    pub public_funs: usize,
    /// `entry fun`.
    pub entry_funs: usize,
    /// `native fun` declarations.
    pub native_funs: usize,
    /// `struct` declarations.
    pub structs: usize,
    /// `has` ability clauses (`copy`/`drop`/`store`/`key`).
    pub abilities: usize,
    /// `use` statements.
    pub uses: usize,
    /// `friend` declarations.
    pub friends: usize,
    /// `acquires` clauses.
    pub acquires: usize,
    /// `spec` blocks (`spec fun`/`spec module`/`spec {`).
    pub specs: usize,
    /// `const` declarations.
    pub consts: usize,
    /// `//`/`///`/`/*` comment markers.
    pub comments: usize,
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}
fn code_has(t: &str, needle: &str) -> bool {
    // `//`/`/*` コメント行内の言及は証拠にしない。
    t.lines().any(|l| {
        let l = l.trim_start();
        !l.starts_with("//") && !l.starts_with("/*") && l.contains(needle)
    })
}

/// Returns `true` when `b` looks like Move source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    (code_has(t, "module ") && code_has(t, "::") && code_has(t, "{"))
        || code_has(t, "script {")
        || code_has(t, "public fun ")
        || code_has(t, "entry fun ")
}

impl Move {
    /// Counts Move constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).ok()?;
        let mut m = Self {
            modules: 0,
            scripts: t.matches("script {").count(),
            functions: 0,
            public_funs: t.matches("public fun").count(),
            entry_funs: t.matches("entry fun").count(),
            native_funs: t.matches("native fun").count(),
            structs: 0,
            abilities: 0,
            uses: 0,
            friends: 0,
            acquires: t.matches("acquires").count(),
            specs: t.matches("spec ").count(),
            consts: 0,
            comments: t.matches("//").count() + t.matches("/*").count(),
        };
        for line in t.lines() {
            match line.split_whitespace().next().unwrap_or("") {
                "module" => m.modules += 1,
                "use" => m.uses += 1,
                "friend" => m.friends += 1,
                "struct" => m.structs += 1,
                "const" => m.consts += 1,
                _ => {}
            }
        }
        for w in words(t) {
            match w {
                "fun" => m.functions += 1,
                "has" => m.abilities += 1,
                _ => {}
            }
        }
        Some(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"// script {\n// public fun f()\n"));
    }

    const SRC: &[u8] = b"module 0x1::coin {\n    public fun mint() {}\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"public fun f() {}"));
        assert!(detect(b"script { fun main() {} }"));
        assert!(!detect(b"fun f() {}"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let m = Move::parse(SRC).unwrap();
        assert_eq!(m.modules, 1);
        assert_eq!(m.functions, 1);
        assert_eq!(m.public_funs, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"module 0x2::t {\n    use 0x1::a;\n    friend 0x1::b;\n    const K: u64 = 1;\n    struct S has copy, drop, store, key { x: u64 }\n    entry fun go() acquires S {}\n    native fun n();\n    spec fun v() {}\n}\n// c\n";
        let m = Move::parse(s).unwrap();
        assert_eq!(m.uses, 1);
        assert_eq!(m.friends, 1);
        assert_eq!(m.consts, 1);
        assert_eq!(m.structs, 1);
        assert_eq!(m.abilities, 1);
        assert_eq!(m.entry_funs, 1);
        assert_eq!(m.native_funs, 1);
        assert_eq!(m.acquires, 1);
        assert_eq!(m.specs, 1);
        assert!(m.comments >= 1);
    }

    #[test]
    fn rejects() {
        assert!(Move::parse(b"plain text").is_none());
        assert!(Move::parse(b"").is_none());
    }
}

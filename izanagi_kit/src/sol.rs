//! Solidity smart-contract source (`.sol`) parser.
//!
//! Detects Solidity by `pragma solidity` or `contract`/`library` +
//! `function`/`mapping`/`event` markers, and counts declarations,
//! imports, `emit`/`require`/`revert` and `//`/`/*`/`///` comments.
//!
//! ```
//! use izanagi_kit::sol::Sol;
//! let src = b"pragma solidity ^0\x2e8;\ncontract C {\n    function f() public {}\n}\n";
//! assert!(izanagi_kit::sol::detect(src));
//! let s = Sol::parse(src).unwrap();
//! assert_eq!(s.contracts, 1);
//! assert_eq!(s.functions, 1);
//! ```

/// Parsed census of a Solidity source file.
#[derive(Debug, Clone)]
pub struct Sol {
    /// `pragma solidity` version expression (e.g. `^0.8` text).
    pub pragma: String,
    /// `pragma` directives.
    pub pragmas: usize,
    /// `contract` declarations.
    pub contracts: usize,
    /// `library` declarations.
    pub libraries: usize,
    /// `interface` declarations.
    pub interfaces: usize,
    /// `abstract` declarations.
    pub abstracts: usize,
    /// `function` declarations/calls.
    pub functions: usize,
    /// `modifier` declarations.
    pub modifiers: usize,
    /// `event` declarations.
    pub events: usize,
    /// `error` declarations.
    pub errors: usize,
    /// `struct` declarations.
    pub structs: usize,
    /// `enum` declarations.
    pub enums: usize,
    /// `mapping(` types.
    pub mappings: usize,
    /// `import` statements.
    pub imports: usize,
    /// `emit` statements.
    pub emits: usize,
    /// `require`/`revert`/`assert` checks.
    pub checks: usize,
    /// `//`, `/*`, `///` comment markers.
    pub comments: usize,
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

/// Returns `true` when `b` looks like Solidity source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.contains("pragma solidity")
        || (t.contains("contract ")
            && (t.contains("function ") || t.contains("mapping(") || t.contains("event ")))
}

impl Sol {
    /// Counts Solidity constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).ok()?;
        let mut s = Self {
            pragma: String::new(),
            pragmas: t.matches("pragma ").count(),
            contracts: 0,
            libraries: 0,
            interfaces: 0,
            abstracts: 0,
            functions: 0,
            modifiers: 0,
            events: 0,
            errors: 0,
            structs: 0,
            enums: 0,
            mappings: t.matches("mapping(").count(),
            imports: t.matches("import ").count(),
            emits: 0,
            checks: 0,
            comments: t.matches("//").count() + t.matches("/*").count(),
        };
        for line in t.lines() {
            let l = line.trim();
            if let Some(rest) = l.strip_prefix("pragma solidity") {
                let v = rest.trim_end_matches(';').trim().to_string();
                if s.pragma.is_empty() {
                    s.pragma = v;
                }
            }
        }
        for w in words(t) {
            match w {
                "contract" => s.contracts += 1,
                "library" => s.libraries += 1,
                "interface" => s.interfaces += 1,
                "abstract" => s.abstracts += 1,
                "function" => s.functions += 1,
                "modifier" => s.modifiers += 1,
                "event" => s.events += 1,
                "error" => s.errors += 1,
                "struct" => s.structs += 1,
                "enum" => s.enums += 1,
                "emit" => s.emits += 1,
                "require" | "revert" | "assert" => s.checks += 1,
                _ => {}
            }
        }
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"pragma solidity ^0\x2e8;\ncontract C {\n    function f() public {}\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"contract X { mapping(address => uint) bal; }"));
        assert!(!detect(b"contract C { }"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let s = Sol::parse(SRC).unwrap();
        assert_eq!(s.pragma, "^0\x2e8");
        assert_eq!(s.pragmas, 1);
        assert_eq!(s.contracts, 1);
        assert_eq!(s.functions, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"pragma solidity >=0\x2e7;\nimport \"x.sol\";\nlibrary L {}\ninterface I {}\nabstract contract A {}\ncontract B {\n    event E();\n    error Err();\n    struct S { uint a; }\n    enum E2 { A }\n    modifier m() { _; }\n    function g() public { emit E(); require(true); revert(); }\n}\n// c\n";
        let s = Sol::parse(s).unwrap();
        assert_eq!(s.libraries, 1);
        assert_eq!(s.interfaces, 1);
        assert_eq!(s.abstracts, 1);
        assert_eq!(s.contracts, 2);
        assert_eq!(s.events, 1);
        assert_eq!(s.errors, 1);
        assert_eq!(s.structs, 1);
        assert_eq!(s.enums, 1);
        assert_eq!(s.modifiers, 1);
        assert_eq!(s.emits, 1);
        assert_eq!(s.checks, 2);
        assert_eq!(s.imports, 1);
    }

    #[test]
    fn rejects() {
        assert!(Sol::parse(b"plain text").is_none());
        assert!(Sol::parse(b"").is_none());
    }
}

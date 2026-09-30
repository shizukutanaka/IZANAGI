//! Cairo (StarkNet) smart-contract source (`.cairo`) parser.
//!
//! Detects Cairo by `%lang starknet`, `#[starknet::contract]`-style
//! attributes or `func`/`felt` idioms, and counts attributes,
//! functions, structs, impls, mods, uses and `//` comments.
//!
//! ```
//! use izanagi_kit::cairo::Cairo;
//! let src = b"#[starknet::contract]\nmod Counter {\n    #[storage]\n    struct Storage { c: u128 }\n}\n";
//! assert!(izanagi_kit::cairo::detect(src));
//! let c = Cairo::parse(src).unwrap();
//! assert_eq!(c.attributes, 2);
//! assert_eq!(c.mods, 1);
//! ```

/// Parsed census of a Cairo source file.
#[derive(Debug, Clone)]
pub struct Cairo {
    /// `%lang starknet` directive (`true` when present).
    pub starknet: bool,
    /// `#[...]` attribute lines (`starknet::contract`/`external`/`view`/`storage`/`event`/…).
    pub attributes: usize,
    /// `#[storage]` specifically.
    pub storage: usize,
    /// `#[external]`/`#[external(v0)]`/`#[abi]` external entries.
    pub externals: usize,
    /// `#[view]` entries.
    pub views: usize,
    /// `#[event]`/`#[l1_handler]` entries.
    pub handlers: usize,
    /// `func`/`fn` definitions.
    pub functions: usize,
    /// `struct` declarations.
    pub structs: usize,
    /// `impl` blocks.
    pub impls: usize,
    /// `mod` declarations.
    pub mods: usize,
    /// `trait` declarations.
    pub traits: usize,
    /// `use` statements.
    pub uses: usize,
    /// `let` bindings.
    pub lets: usize,
    /// `//` comment markers.
    pub comments: usize,
}

fn words(t: &str) -> impl Iterator<Item = &str> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

/// Returns `true` when `b` looks like Cairo source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.contains("%lang starknet")
        || t.contains("#[starknet")
        || t.contains("#[contract]")
        || t.contains("#[storage]")
        || t.contains("#[external")
        || t.contains("#[event]")
        || (t.contains("func ") && t.contains("felt"))
}

impl Cairo {
    /// Counts Cairo constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).ok()?;
        let mut c = Self {
            starknet: t.contains("%lang starknet"),
            attributes: 0,
            storage: t.matches("#[storage]").count(),
            externals: t.matches("#[external").count() + t.matches("#[abi").count(),
            views: t.matches("#[view]").count(),
            handlers: t.matches("#[event]").count() + t.matches("#[l1_handler]").count(),
            functions: 0,
            structs: 0,
            impls: 0,
            mods: 0,
            traits: 0,
            uses: 0,
            lets: 0,
            comments: t.matches("//").count(),
        };
        for line in t.lines() {
            let l = line.trim_start();
            if l.starts_with("#[") {
                c.attributes += 1;
            }
        }
        for w in words(t) {
            match w {
                "func" | "fn" => c.functions += 1,
                "struct" => c.structs += 1,
                "impl" => c.impls += 1,
                "mod" => c.mods += 1,
                "trait" => c.traits += 1,
                "use" => c.uses += 1,
                "let" => c.lets += 1,
                _ => {}
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"#[starknet::contract]\nmod Counter {\n    #[storage]\n    struct Storage { c: u128 }\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"%lang starknet\nfunc f() {}"));
        assert!(detect(b"func g() -> felt { }"));
        assert!(!detect(b"mod m { fn f() {} }"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let c = Cairo::parse(SRC).unwrap();
        assert!(!c.starknet);
        assert_eq!(c.attributes, 2);
        assert_eq!(c.storage, 1);
        assert_eq!(c.mods, 1);
        assert_eq!(c.structs, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"%lang starknet\n#[contract]\nmod T {\n    use x::y;\n    #[external]\n    fn go() { let a = 1; }\n    #[view]\n    fn see() {}\n    #[event]\n    fn ev() {}\n    impl I {}\n    trait Tr {}\n}\n// c\n";
        let c = Cairo::parse(s).unwrap();
        assert!(c.starknet);
        assert_eq!(c.externals, 1);
        assert_eq!(c.views, 1);
        assert_eq!(c.handlers, 1);
        assert!(c.functions >= 3);
        assert_eq!(c.impls, 1);
        assert_eq!(c.traits, 1);
        assert_eq!(c.uses, 1);
        assert_eq!(c.lets, 1);
    }

    #[test]
    fn rejects() {
        assert!(Cairo::parse(b"plain text").is_none());
        assert!(Cairo::parse(b"").is_none());
    }
}

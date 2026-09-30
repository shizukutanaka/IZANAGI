//! TON Tact smart-contract source (`.tact`) parser.
//!
//! Detects Tact by `contract`/`trait` + `receive`/`get fun`/`bounced`/
//! `message(` markers, and counts structs, messages, init/receive/
//! getter/bounced bodies, imports and `//` comments.
//!
//! ```
//! use izanagi_kit::tact::Tact;
//! let src = b"contract Counter {\n    init() {}\n    receive(\"inc\") {}\n}\n";
//! assert!(izanagi_kit::tact::detect(src));
//! let t = Tact::parse(src).unwrap();
//! assert_eq!(t.contracts, 1);
//! assert_eq!(t.receives, 1);
//! ```

/// Parsed census of a Tact source file.
#[derive(Debug, Clone)]
pub struct Tact {
    /// `contract` declarations.
    pub contracts: usize,
    /// `trait` declarations.
    pub traits: usize,
    /// `struct` declarations.
    pub structs: usize,
    /// `message(..)` type declarations.
    pub messages: usize,
    /// `init()` / `init(` bodies.
    pub inits: usize,
    /// `receive` bodies (incl. `receive("text")`).
    pub receives: usize,
    /// `get fun` getters.
    pub gets: usize,
    /// `bounced` handlers.
    pub bounceds: usize,
    /// `fun` definitions (excluding `get fun`).
    pub functions: usize,
    /// `import` statements.
    pub imports: usize,
    /// `require` checks.
    pub requires: usize,
    /// `//` comment markers.
    pub comments: usize,
}

/// Returns `true` when `b` looks like Tact source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    (t.contains("contract ")
        && (t.contains("receive") || t.contains("get fun") || t.contains("init(")))
        || t.contains("message(")
        || t.contains("bounced(")
}

impl Tact {
    /// Counts Tact constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).ok()?;
        let mut tc = Self {
            contracts: 0,
            traits: 0,
            structs: 0,
            messages: t.matches("message(").count(),
            inits: t.matches("init(").count(),
            receives: t.matches("receive").count(),
            gets: t.matches("get fun").count(),
            bounceds: t.matches("bounced").count(),
            functions: 0,
            imports: 0,
            requires: t.matches("require(").count(),
            comments: t.matches("//").count() + t.matches("/*").count(),
        };
        for line in t.lines() {
            match line.split_whitespace().next().unwrap_or("") {
                "contract" => tc.contracts += 1,
                "trait" => tc.traits += 1,
                "struct" => tc.structs += 1,
                "import" => tc.imports += 1,
                _ => {}
            }
        }
        let funs = t.matches("fun ").count();
        tc.functions = funs.saturating_sub(tc.gets);
        Some(tc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"contract Counter {\n    init() {}\n    receive(\"inc\") {}\n}\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"message(0x1) Ping { seq: Int }"));
        assert!(!detect(b"contract C { }"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let tc = Tact::parse(SRC).unwrap();
        assert_eq!(tc.contracts, 1);
        assert_eq!(tc.inits, 1);
        assert_eq!(tc.receives, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b"import \"a.tact\";\ntrait T {}\nstruct S { x: Int }\nmessage(0x1) M { v: Int }\ncontract C {\n    init() {}\n    receive(m: M) {}\n    bounced(b: M) {}\n    get fun total(): Int { return 0; }\n    fun helper() {}\n}\n// c\n";
        let tc = Tact::parse(s).unwrap();
        assert_eq!(tc.imports, 1);
        assert_eq!(tc.traits, 1);
        assert_eq!(tc.structs, 1);
        assert_eq!(tc.messages, 1);
        assert_eq!(tc.inits, 1);
        assert_eq!(tc.receives, 1);
        assert_eq!(tc.bounceds, 1);
        assert_eq!(tc.gets, 1);
        assert_eq!(tc.functions, 1);
        assert!(tc.comments >= 1);
    }

    #[test]
    fn rejects() {
        assert!(Tact::parse(b"plain text").is_none());
        assert!(Tact::parse(b"").is_none());
    }
}

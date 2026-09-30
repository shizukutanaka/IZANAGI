//! OMG Interface Definition Language (CORBA IDL / PSM IDL) — `module`
//! namespaces with `interface`/`struct`/`union`/`enum`/`typedef`/`exception`
//! members, `attribute`/`readonly attribute` properties, `in`/`out`/`inout`
//! parameter directions, and `sequence<T>` collection types. `};` after
//! braces is the classic IDL terminator.
//!
//! ```
//! let d = b"module Bank {\n  interface Account {\n    readonly attribute long balance;\n    void deposit(in long amount);\n  };\n  struct Tx { long id; };\n};\n";
//! let i = izanagi_kit::idl::parse(d).unwrap();
//! assert_eq!(i.modules, 1);
//! assert_eq!(i.interfaces, 1);
//! assert_eq!(i.structs, 1);
//! assert_eq!(i.attributes, 1);
//! assert_eq!(i.operations, 1);
//! assert!(izanagi_kit::idl::detect(d));
//! ```

/// A censused OMG IDL document.
pub struct Idl {
    /// `module X {` count.
    pub modules: u32,
    /// `interface X`/`interface X :` count.
    pub interfaces: u32,
    /// `struct X` count.
    pub structs: u32,
    /// `union X switch` count.
    pub unions: u32,
    /// `enum X` count.
    pub enums: u32,
    /// `typedef` count.
    pub typedefs: u32,
    /// `exception X` count.
    pub exceptions: u32,
    /// `attribute`/`readonly attribute` count.
    pub attributes: u32,
    /// Operation lines: `ret name(params);` inside interfaces — counted as
    /// `in`/`out`/`inout`-parameterised calls plus attribute-free `()` calls.
    pub operations: u32,
    /// `sequence<` usage count.
    pub sequences: u32,
    /// `in`/`out`/`inout` parameter-direction occurrences.
    pub directions: u32,
}

fn word_count(s: &str, word: &str) -> u32 {
    let mut n = 0u32;
    for (i, _) in s.match_indices(word) {
        let before_ok =
            i == 0 || !s.as_bytes()[i - 1].is_ascii_alphanumeric() && s.as_bytes()[i - 1] != b'_';
        let j = i + word.len();
        let after_ok =
            j >= s.len() || !s.as_bytes()[j].is_ascii_alphanumeric() && s.as_bytes()[j] != b'_';
        if before_ok && after_ok {
            n += 1;
        }
    }
    n
}

fn count(s: &str, pat: &str) -> u32 {
    u32::try_from(s.matches(pat).count()).unwrap_or(u32::MAX)
}

/// `module`/`interface`/`struct` + `};` block terminators.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    (word_count(s, "module") + word_count(s, "interface") > 0) && s.contains("};")
}

/// Parses the document; `None` without IDL declarations.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Idl> {
    let s = core::str::from_utf8(b).ok()?;
    let operations = s
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.ends_with(';')
                && t.contains('(')
                && t.contains(')')
                && !t.starts_with("//")
                && !t.starts_with('#')
                && !word_is(t, "if")
                && !t.contains("attribute")
        })
        .count();
    let idl = Idl {
        modules: word_count(s, "module"),
        interfaces: word_count(s, "interface"),
        structs: word_count(s, "struct"),
        unions: word_count(s, "union"),
        enums: word_count(s, "enum"),
        typedefs: word_count(s, "typedef"),
        exceptions: word_count(s, "exception"),
        attributes: word_count(s, "attribute"),
        operations: u32::try_from(operations).unwrap_or(u32::MAX),
        sequences: count(s, "sequence<"),
        directions: word_count(s, "in") + word_count(s, "out") + word_count(s, "inout"),
    };
    (idl.modules + idl.interfaces + idl.structs > 0).then_some(idl)
}

fn word_is(t: &str, w: &str) -> bool {
    t == w || t.starts_with(&format!("{w} ")) || t.starts_with(&format!("{w}("))
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDL: &[u8] = b"module Bank {\n  interface Account {\n    readonly attribute long balance;\n    void deposit(in long amount);\n  };\n  struct Tx { long id; };\n};\n";

    #[test]
    fn detect_works() {
        assert!(detect(IDL));
        assert!(!detect(b"module x;"));
        assert!(!detect(b"interface{}"));
    }

    #[test]
    fn parses() {
        let i = parse(IDL).unwrap();
        assert_eq!(i.modules, 1);
        assert_eq!(i.interfaces, 1);
        assert_eq!(i.structs, 1);
        assert_eq!(i.attributes, 1);
        assert_eq!(i.operations, 1);
        assert_eq!(i.directions, 1);
        assert_eq!(i.sequences, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"// just comments").is_none());
    }
}

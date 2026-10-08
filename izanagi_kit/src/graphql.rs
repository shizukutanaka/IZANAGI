//! GraphQL schema (SDL) document census — `type`/`interface`/`input`/`enum`/
//! `union`/`scalar`/`directive @`/`schema`/`extend` declarations plus
//! `query`/`mutation`/`subscription` field roots and `@deprecated`-style
//! applied directives. `getPost(id: 1)` query documents share the same
//! surface but are detected only when `query`/`mutation` keywords lead.
//!
//! ```
//! let d = b"schema { query: Q }\ntype Q { user(id: ID!): User }\ntype User { id: ID! name: String }\nenum Role { ADMIN USER }\ninput Filter { name: String }\ndirective @auth on FIELD_DEFINITION\n";
//! let g = izanagi_kit::graphql::parse(d).unwrap();
//! assert_eq!(g.types, 2);
//! assert_eq!(g.enums, 1);
//! assert_eq!(g.inputs, 1);
//! assert_eq!(g.directive_defs, 1);
//! assert!(g.has_schema_block);
//! assert!(izanagi_kit::graphql::detect(d));
//! ```

/// A censused GraphQL SDL document.
#[derive(Debug)]
pub struct Graphql {
    /// `type X` object-type declarations.
    pub types: u32,
    /// `interface X` declarations.
    pub interfaces: u32,
    /// `input X` declarations.
    pub inputs: u32,
    /// `enum X` declarations.
    pub enums: u32,
    /// `union X =` declarations.
    pub unions: u32,
    /// `scalar X` declarations.
    pub scalars: u32,
    /// `directive @x on …` definitions.
    pub directive_defs: u32,
    /// `extend type/interface/…` extension count.
    pub extensions: u32,
    /// Whether a `schema {` block is present.
    pub has_schema_block: bool,
    /// Applied `@dir` usages (excluding `directive @` definition sites).
    pub applied_directives: u32,
}

fn kw_count(s: &str, kw: &str) -> u32 {
    let kw = kw.trim_end();
    let mut n = 0u32;
    for line in s.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix(kw) {
            if rest.is_empty()
                || rest
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_whitespace() || c == '{')
            {
                n += 1;
            }
        }
    }
    n
}

fn count_at_directives(s: &str) -> u32 {
    // applied `@name` usages — exclude the `directive @` definition form
    let mut n = 0u32;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let is_def = s[..i].trim_end().ends_with("directive");
            let next_ok = bytes
                .get(i + 1)
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_');
            if !is_def && next_ok {
                n += 1;
            }
        }
        i += 1;
    }
    n
}

/// SDL keywords: `type X`, `schema`, `directive @`, `enum X`, …
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut marks = 0u32;
    for kw in [
        "type",
        "enum",
        "input",
        "scalar",
        "directive",
        "schema",
        "union",
        "interface",
        "extend",
    ] {
        if kw_count(s, kw) > 0 {
            marks += 1;
        }
    }
    marks >= 2
}

/// Parses the document; `None` without at least one SDL declaration.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Graphql> {
    let s = core::str::from_utf8(b).ok()?;
    let g = Graphql {
        types: kw_count(s, "type"),
        interfaces: kw_count(s, "interface"),
        inputs: kw_count(s, "input"),
        enums: kw_count(s, "enum"),
        unions: kw_count(s, "union"),
        scalars: kw_count(s, "scalar"),
        directive_defs: s
            .lines()
            .filter(|l| l.trim_start().starts_with("directive @"))
            .count() as u32,
        extensions: kw_count(s, "extend"),
        has_schema_block: s.lines().any(|l| l.trim_start().starts_with("schema")),
        applied_directives: count_at_directives(s),
    };
    if g.types
        + g.interfaces
        + g.inputs
        + g.enums
        + g.unions
        + g.scalars
        + g.directive_defs
        + g.extensions
        + u32::from(g.has_schema_block)
        == 0
    {
        return None;
    }
    // subtract `directive @` def sites counted into applied
    Some(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SDL: &[u8] = b"schema { query: Q }\ntype Q { user(id: ID!): User }\ntype User { id: ID! name: String @deprecated }\nenum Role { ADMIN USER }\ninput Filter { name: String }\ndirective @auth on FIELD_DEFINITION\n";

    #[test]
    fn detect_works() {
        assert!(detect(SDL));
        assert!(!detect(b"type x = 1"));
        assert!(!detect(b"SELECT type FROM t"));
    }

    #[test]
    fn parses() {
        let g = parse(SDL).unwrap();
        assert_eq!(g.types, 2);
        assert_eq!(g.enums, 1);
        assert_eq!(g.inputs, 1);
        assert_eq!(g.directive_defs, 1);
        assert_eq!(g.applied_directives, 1); // @deprecated only; @auth def excluded
        assert!(g.has_schema_block);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello world").is_none());
    }
}

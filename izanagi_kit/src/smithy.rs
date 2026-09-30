//! AWS Smithy model definition — `$version: "2"` control + `namespace a.b` +
//! shape declarations (`service`/`operation`/`resource`/`structure`/`list`/
//! `map`/`string`/`integer`/`enum`/`union`/`blob`/`timestamp`) and `@trait`
//! applications (`@readonly`, `@http`, `@input`, …).
//!
//! ```
//! let d = b"$version: \"2\"\nnamespace example.svc\nservice Svc { version: \"1\x2e0\" }\noperation Get { input: GetIn }\nstructure GetIn { id: String }\n@readonly\n";
//! let s = izanagi_kit::smithy::parse(d).unwrap();
//! assert_eq!(s.namespace, "example.svc");
//! assert_eq!(s.services, 1);
//! assert_eq!(s.operations, 1);
//! assert_eq!(s.structures, 1);
//! assert_eq!(s.traits, 1);
//! assert!(izanagi_kit::smithy::detect(d));
//! ```

/// A censused Smithy model.
pub struct Smithy {
    /// `namespace a.b.c` value.
    pub namespace: String,
    /// `$version:` metadata value (`"1"`/`"2"`).
    pub version: String,
    /// `service X` declarations.
    pub services: u32,
    /// `operation X` declarations.
    pub operations: u32,
    /// `resource X` declarations.
    pub resources: u32,
    /// `structure X` declarations.
    pub structures: u32,
    /// `list`/`map`/`set` aggregate declarations.
    pub collections: u32,
    /// `string`/`integer`/`long`/`boolean`/`blob`/`timestamp`/`double`/`bigDecimal`
    /// simple-shape declarations.
    pub simple_shapes: u32,
    /// `enum`/`intEnum`/`union`/`document` declarations.
    pub other_shapes: u32,
    /// `@trait` applications (excluding `@` inside strings? kept simple).
    pub traits: u32,
    /// `use`/`apply` statement count.
    pub statements: u32,
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

/// `$version:` control + `namespace` decl.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("namespace ")
        && (s.contains("$version:") || word_count(s, "service") + word_count(s, "structure") > 0)
}

/// Parses the model; `None` without Smithy markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Smithy> {
    let s = core::str::from_utf8(b).ok()?;
    let namespace = s
        .lines()
        .find_map(|l| {
            let t = l.trim_start();
            t.strip_prefix("namespace ").map(|r| r.trim().to_string())
        })
        .unwrap_or_default();
    let version = s
        .find("$version:")
        .map(|i| {
            s[i + 9..]
                .trim()
                .trim_matches('"')
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('"')
                .to_string()
        })
        .unwrap_or_default();
    let sm = Smithy {
        namespace,
        version,
        services: word_count(s, "service"),
        operations: word_count(s, "operation"),
        resources: word_count(s, "resource"),
        structures: word_count(s, "structure"),
        collections: word_count(s, "list") + word_count(s, "map") + word_count(s, "set"),
        simple_shapes: word_count(s, "string")
            + word_count(s, "integer")
            + word_count(s, "long")
            + word_count(s, "boolean")
            + word_count(s, "blob")
            + word_count(s, "timestamp")
            + word_count(s, "bigDecimal"),
        other_shapes: word_count(s, "enum")
            + word_count(s, "intEnum")
            + word_count(s, "union")
            + word_count(s, "document"),
        traits: count(s, "@"),
        statements: word_count(s, "use") + word_count(s, "apply"),
    };
    (!sm.namespace.is_empty() || !sm.version.is_empty() || sm.services > 0 || sm.structures > 0)
        .then_some(sm)
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: &[u8] = b"$version: \"2\"\nnamespace example.svc\nservice Svc { version: \"1\x2e0\" }\noperation Get { input: GetIn }\nstructure GetIn { id: String }\n@readonly\n";

    #[test]
    fn detect_works() {
        assert!(detect(M));
        assert!(!detect(b"namespace foo"));
        assert!(!detect(b"$version: 1"));
    }

    #[test]
    fn parses() {
        let s = parse(M).unwrap();
        assert_eq!(s.namespace, "example.svc");
        assert_eq!(s.version, "2");
        assert_eq!(s.services, 1);
        assert_eq!(s.operations, 1);
        assert_eq!(s.structures, 1);
        assert_eq!(s.traits, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"nothing").is_none());
    }
}

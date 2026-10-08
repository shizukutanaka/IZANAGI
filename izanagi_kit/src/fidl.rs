//! Fuchsia IDL (FIDL) — `library fuchsia.foo;` + `using` imports +
//! `type`/`struct`/`table`/`union`/`enum`/`bits`/`protocol` declarations,
//! `@discoverable` attributes, `->` request arrows, `client_end:`/`server_end:`
//! protocol endpoints.
//!
//! ```
//! let d = b"library fuchsia.example;\nusing zx;\n@discoverable\nprotocol Echo {\n    Send(struct { msg string:256 }) -> ();\n};\ntype Msg = struct { id uint64 };\n";
//! let f = izanagi_kit::fidl::parse(d).unwrap();
//! assert_eq!(f.library, "fuchsia.example");
//! assert_eq!(f.usings, 1);
//! assert_eq!(f.protocols, 1);
//! assert_eq!(f.decls, 2); // protocol + struct
//! assert_eq!(f.methods, 1);
//! assert!(izanagi_kit::fidl::detect(d));
//! ```

/// A censused FIDL file.
pub struct Fidl {
    /// `library a.b.c;` name.
    pub library: String,
    /// `using x.y;` import count.
    pub usings: u32,
    /// `protocol X {` count.
    pub protocols: u32,
    /// `struct`/`table`/`union`/`enum`/`bits`/`type`/`service`/`const` decls
    /// (including `protocol`).
    pub decls: u32,
    /// Method lines inside `protocol` (name + `(` … `)`).
    pub methods: u32,
    /// `->` response-arrow count.
    pub arrows: u32,
    /// `@attr` attribute occurrences.
    pub attributes: u32,
    /// `client_end:`/`server_end:` endpoint type uses.
    pub endpoints: u32,
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
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `library a.b;` at line start + one declaration keyword.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let s = strip_bom(s);
    s.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("library ") && t.contains(';')
    })
}

/// Parses the file; `None` without a `library` decl.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Fidl> {
    let s = core::str::from_utf8(b).ok()?;
    let s = strip_bom(s);
    let library = s.lines().find_map(|l| {
        let t = l.trim_start();
        t.strip_prefix("library ")
            .and_then(|r| r.trim_end_matches(';').split_whitespace().next())
            .map(str::to_string)
    })?;
    let mut in_proto_depth = 0i32;
    let mut in_proto = false;
    let mut depth = 0i32;
    let mut methods = 0u32;
    for line in s.lines() {
        let t = line.trim();
        let opens = t.matches('{').count() as i32;
        let closes = t.matches('}').count() as i32;
        if !in_proto && t.starts_with("protocol") {
            in_proto = true;
            in_proto_depth = depth + opens;
        } else if in_proto && t.contains('(') && !t.starts_with("//") {
            methods += 1;
        }
        depth += opens - closes;
        if in_proto && depth < in_proto_depth {
            in_proto = false;
        }
    }
    let decls = s
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            [
                "protocol ",
                "struct ",
                "table ",
                "union ",
                "enum ",
                "bits ",
                "service ",
                "type ",
                "const ",
                "alias ",
                "resource ",
            ]
            .iter()
            .any(|k| t.starts_with(k))
        })
        .count();
    Some(Fidl {
        library,
        usings: s
            .lines()
            .filter(|l| l.trim_start().starts_with("using "))
            .count() as u32,
        protocols: word_count(s, "protocol"),
        decls: u32::try_from(decls).unwrap_or(u32::MAX),
        methods,
        arrows: count(s, "->"),
        attributes: count(s, "@"),
        endpoints: word_count(s, "client_end:") + word_count(s, "server_end:"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: &[u8] = b"library fuchsia.example;\nusing zx;\n@discoverable\nprotocol Echo {\n    Send(struct { msg string:256 }) -> ();\n};\ntype Msg = struct { id uint64 };\n";

    #[test]
    fn detect_works() {
        assert!(detect(F));
        assert!(!detect(b"library not fidl"));
    }

    #[test]
    fn parses() {
        let f = parse(F).unwrap();
        assert_eq!(f.library, "fuchsia.example");
        assert_eq!(f.usings, 1);
        assert_eq!(f.protocols, 1);
        assert_eq!(f.decls, 2);
        assert_eq!(f.methods, 1);
        assert_eq!(f.arrows, 1);
        assert_eq!(f.attributes, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"no library here").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

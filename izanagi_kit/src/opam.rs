//! OCaml `.opam` package definition — `opam-version:` signature plus
//! `name:`/`version:`/`depends:`/`depexts:`/`build:`/`install:` census.
//!
//! ```
//! let d = b"opam-version: \"2\x2e0\"\nname: \"demo\"\nversion: \"0\x2e1\"\nsynopsis: \"demo package\"\nmaintainer: \"j@example\x2ecom\"\nlicense: \"MIT\"\nbuild: [\n  [\"dune\" \"build\" \"-p\" name]\n]\ndepends: [\n  \"ocaml\" {>= \"4\x2e08\"}\n  \"dune\" {>= \"2\x2e0\"}\n  \"lwt\"\n]\ndepopts: [\"odoc\"]\n";
//! let o = izanagi_kit::opam::parse(d).unwrap();
//! assert_eq!(o.name, "demo");
//! assert_eq!(o.depends, 3);
//! assert_eq!(o.build_blocks, 1);
//! assert!(izanagi_kit::opam::detect(d));
//! ```

/// A parsed `.opam` file summary.
#[derive(Debug, Clone)]
pub struct Opam {
    /// `opam-version:` value (`2.0`/`2.1`).
    pub opam_version: String,
    /// `name:` value.
    pub name: String,
    /// `version:` value.
    pub version: String,
    /// `synopsis:` value.
    pub synopsis: String,
    /// `license:` value.
    pub license: String,
    /// `maintainer:` entries.
    pub maintainers: usize,
    /// `authors:` entries.
    pub authors: usize,
    /// `build:`/`install:`/`run-test:`/`build-test:` command blocks.
    pub build_blocks: usize,
    /// `depends:` package atoms (`"pkg"` tokens in the `[ … ]` list).
    pub depends: usize,
    /// `depopts:` package atoms.
    pub depopts: usize,
    /// `conflicts:`/`conflict-class:` package atoms.
    pub conflicts: usize,
    /// `depexts:` entries.
    pub depexts: usize,
    /// `flags:` entries (`light-uninstall`/`plugin`/`compiler`).
    pub flags: usize,
    /// `url { src: "…" }` block present.
    pub url: bool,
    /// `available:`/`os`/`arch` constraint expressions present.
    pub constraints: usize,
}

/// `key:` scalar value (any depth-0 line), stripped of quotes.
fn top_value<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for line in t.lines() {
        if line.starts_with(char::is_whitespace) || line.starts_with('#') {
            continue;
        }
        let l = line.trim_end();
        if let Some(rest) = l.strip_prefix(key) {
            let rest = rest.strip_prefix(':')?.trim();
            let v = rest.trim_end_matches(';').trim_matches('"');
            return Some(v);
        }
    }
    None
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with(key) && l[key.len()..].trim_start().starts_with(':')
    })
}

/// Lines belonging to a `key:` field — until the next depth-0 `word:`.
fn field_body(t: &str, key: &str) -> String {
    let mut out = String::new();
    let mut in_field = false;
    for line in t.lines() {
        let l = line.trim_end();
        if l.starts_with(char::is_whitespace) {
            if in_field {
                out.push_str(l);
                out.push('\n');
            }
            continue;
        }
        let starts_key = |s: &str| {
            s.trim_start().starts_with(key)
                && s.trim_start()[key.len()..].trim_start().starts_with(':')
        };
        if starts_key(l) {
            in_field = true;
            out.push_str(&l[key.len() + 1..]);
            out.push('\n');
            continue;
        }
        in_field = false;
    }
    out
}

/// `"pkg"` atoms inside a field body — `{…}` constraint groups are stripped
/// first so `{>= "4.08"}` does not inflate the count.
fn quoted_atoms(s: &str) -> usize {
    let mut stripped = String::with_capacity(s.len());
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => stripped.push(c),
            _ => {}
        }
    }
    stripped
        .split('"')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .count()
}

/// Detects an `.opam` file: `opam-version:` signature.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    has_key(t, "opam-version")
        || (has_key(t, "depends") && has_key(t, "build") && has_key(t, "maintainer"))
}

/// Parses an `.opam` file; `None` without `opam-version:`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Opam> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut build_blocks = 0;
    for k in ["build", "install", "run-test", "build-test", "build-env"] {
        if has_key(t, k) {
            build_blocks += 1;
        }
    }
    let mut constraints = 0;
    for k in ["available", "os", "arch", "os-family", "os-distribution"] {
        if has_key(t, k) {
            constraints += 1;
        }
    }
    let depends = quoted_atoms(&field_body(t, "depends"));
    Some(Opam {
        opam_version: top_value(t, "opam-version").unwrap_or("").to_string(),
        name: top_value(t, "name").unwrap_or("").to_string(),
        version: top_value(t, "version").unwrap_or("").to_string(),
        synopsis: top_value(t, "synopsis").unwrap_or("").to_string(),
        license: top_value(t, "license").unwrap_or("").to_string(),
        maintainers: quoted_atoms(&field_body(t, "maintainer")),
        authors: quoted_atoms(&field_body(t, "authors")),
        build_blocks,
        depends,
        depopts: quoted_atoms(&field_body(t, "depopts")),
        conflicts: quoted_atoms(&field_body(t, "conflicts"))
            + quoted_atoms(&field_body(t, "conflict-class")),
        depexts: quoted_atoms(&field_body(t, "depexts")),
        flags: quoted_atoms(&field_body(t, "flags")),
        url: has_key(t, "url") || t.contains("url {") || t.contains("url{"),
        constraints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"opam-version: \"2\x2e0\"\nname: \"demo\"\nversion: \"0\x2e1\"\nsynopsis: \"demo package\"\nmaintainer: \"j@example\x2ecom\"\nauthors: \"Jane\"\nlicense: \"MIT\"\nhomepage: \"https://example\x2ecom\"\nbuild: [\n  [\"dune\" \"build\" \"-p\" name]\n]\nrun-test: [\n  [\"dune\" \"runtest\"]\n]\ndepends: [\n  \"ocaml\" {>= \"4\x2e08\"}\n  \"dune\" {>= \"2\x2e0\"}\n  \"lwt\"\n  \"cmdliner\"\n]\ndepopts: [\"odoc\"]\nconflicts: [\"old-demo\"]\nurl { src: \"https://example\x2ecom/demo\x2etar\x2egz\" }\n";

    #[test]
    fn parses() {
        let o = parse(D).unwrap();
        assert_eq!(o.opam_version, "2\x2e0");
        assert_eq!(o.name, "demo");
        assert_eq!(o.license, "MIT");
        assert_eq!(o.maintainers, 1);
        assert_eq!(o.build_blocks, 2);
        assert_eq!(o.depends, 4);
        assert_eq!(o.depopts, 1);
        assert_eq!(o.conflicts, 1);
        assert!(o.url);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"opam-version: \"2\x2e1\"\n"));
        assert!(!detect(b"name: x\nversion: 1\x2e0\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}

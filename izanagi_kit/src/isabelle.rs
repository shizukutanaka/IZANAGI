//! Isabelle/Isar theory (`.thy`) census.
//!
//! A theory file is `theory Name` + `imports A B …` + `begin` … `end`,
//! with `(* … *)` comments and declarative keywords
//! (`lemma`/`theorem`/`definition`/`fun`/`function`/`inductive`/
//! `datatype`/`locale`/`class`/`instantiation`/`primrec`/`recdef`/
//! `proof`/`qed`/`by`/`apply`/`done`/`sorry`/`oops`).
//!
//! ```
//! let d = b"theory Demo\nimports Main HOL\nbegin\n\n\
//! lemma foo: \"True\" by simp\n\nend\n";
//! let t = izanagi_kit::isabelle::parse(d).unwrap();
//! assert_eq!(t.imports, 2);
//! assert_eq!(t.lemmas, 1);
//! ```

use crate::textutil::strip_bom;
/// Census fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Isabelle {
    /// `theory` header seen.
    pub theory: bool,
    /// Imported theory names on the `imports` line(s).
    pub imports: u32,
    /// `begin`/`end` block markers.
    pub begin_end: u32,
    /// `lemma`/`theorem`/`corollary`/`proposition`/`schematic_goal` decls.
    pub lemmas: u32,
    /// `definition`/`primrec`/`fun`/`function`/`recdef` decls.
    pub definitions: u32,
    /// `inductive`/`coinductive`/`datatype`/`codatatype` decls.
    pub datatypes: u32,
    /// `locale`/`class`/`instantiation`/`instance` decls.
    pub locales: u32,
    /// `proof`/`qed`/`apply`/`done`/`by` tactic keywords.
    pub tactics: u32,
    /// `sorry`/`oops` holes.
    pub holes: u32,
}

const KW: &[(&str, u8)] = &[
    ("lemma", 1),
    ("theorem", 1),
    ("corollary", 1),
    ("proposition", 1),
    ("schematic_goal", 1),
    ("definition", 2),
    ("primrec", 2),
    ("fun", 2),
    ("function", 2),
    ("recdef", 2),
    ("inductive", 3),
    ("coinductive", 3),
    ("datatype", 3),
    ("codatatype", 3),
    ("locale", 4),
    ("class", 4),
    ("instantiation", 4),
    ("instance", 4),
    ("proof", 5),
    ("qed", 5),
    ("apply", 5),
    ("done", 5),
    ("by", 5),
    ("sorry", 6),
    ("oops", 6),
];

fn word_at(s: &str, i: usize) -> &str {
    let rest = &s[i..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    &rest[..end]
}

fn census(s: &str, t: &mut Isabelle) {
    // Strip (* ... *) comments (non-nested approximation).
    let mut clean = String::with_capacity(s.len());
    let mut depth = 0u32;
    let mut it = s.as_bytes().iter().copied().peekable();
    while let Some(c) = it.next() {
        if c == b'(' && it.peek() == Some(&b'*') {
            it.next();
            depth += 1;
            continue;
        }
        if depth > 0 {
            if c == b'*' && it.peek() == Some(&b')') {
                it.next();
                depth -= 1;
            }
            continue;
        }
        clean.push(c as char);
    }
    for (i, _) in clean.match_indices(|c: char| c.is_ascii_alphabetic()) {
        if i > 0 {
            let p = clean.as_bytes()[i - 1];
            if p.is_ascii_alphanumeric() || p == b'_' {
                continue;
            }
        }
        let w = word_at(&clean, i);
        if let Some(&(_, g)) = KW.iter().find(|(k, _)| *k == w) {
            match g {
                1 => t.lemmas += 1,
                2 => t.definitions += 1,
                3 => t.datatypes += 1,
                4 => t.locales += 1,
                5 => t.tactics += 1,
                _ => t.holes += 1,
            }
        }
    }
}

/// `true` on a `theory … imports …` + `begin`/`end` skeleton.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let s = strip_bom(s);
    s.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("theory ")
            && t[7..]
                .trim_start()
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
    }) && s.lines().any(|l| l.trim() == "begin")
}

/// Census; `None` without a theory skeleton.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Isabelle> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let s = strip_bom(s);
    let mut t = Isabelle::default();
    for line in s.lines() {
        let l = line.trim();
        if l.starts_with("theory ") {
            t.theory = true;
        }
        if let Some(rest) = l.strip_prefix("imports") {
            let rest = rest.strip_suffix("begin").unwrap_or(rest);
            t.imports += rest
                .split_ascii_whitespace()
                .filter(|w| {
                    w.trim_start_matches('"')
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_uppercase())
                })
                .count() as u32;
        }
        if l == "begin" || l == "end" {
            t.begin_end += 1;
        }
    }
    census(s, &mut t);
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"(* comment *)\ntheory Demo\nimports Main \"HOL-Library.Maps\"\nbegin\n\n\
definition f :: nat where \"f = 0\"\ndatatype color = Red | Green\n\
lemma l1: \"True\" by simp\nlemma l2: \"True\" proof - qed\n\
fun g where \"g x = x\"\nend\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"theory demo\nbegin\nend\n")); // lowercase theory name
        assert!(!detect(b"lemma x\n"));
    }

    #[test]
    fn parses() {
        let t = parse(D).unwrap();
        assert!(t.theory);
        assert_eq!(t.imports, 2);
        assert_eq!(t.begin_end, 2);
        assert_eq!(t.lemmas, 2);
        assert_eq!(t.definitions, 2); // definition + fun
        assert_eq!(t.datatypes, 1);
        assert!(t.tactics >= 3); // by proof qed
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"theory X\nno begin end\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

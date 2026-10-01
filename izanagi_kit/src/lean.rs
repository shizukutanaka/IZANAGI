//! Lean 4 `.lean` source census.
//!
//! `import Mathlib.Foo` dotted imports; declarations `theorem|def|
//! inductive|structure|class|instance|abbrev|opaque|axiom|noncomputable
//! def`; `namespace|section|open|variable|universe|attribute`; term-mode
//! `by` blocks; commands `#check|#eval|#print|#find|#guard`; `example`;
//! `sorry` holes; `--` line and `/- … -/` block comments.
//!
//! ```
//! let d = b"import Mathlib.Tactic\nnamespace N\ntheorem t : True := by\n\
//!   trivial\n#check t\nend N\n";
//! let l = izanagi_kit::lean::parse(d).unwrap();
//! assert_eq!(l.imports, 1);
//! assert_eq!(l.theorems, 1);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lean {
    /// `import` lines; module names summed separately.
    pub imports: u32,
    /// Dotted module-name atoms across all imports.
    pub import_names: u32,
    /// `theorem` decls.
    pub theorems: u32,
    /// `def`/`abbrev`/`opaque` decls.
    pub defs: u32,
    /// `inductive`/`structure`/`class` decls.
    pub inductives: u32,
    /// `instance`/`axiom`/`noncomputable` decls.
    pub instances: u32,
    /// `example` decls.
    pub examples: u32,
    /// `namespace`/`section`/`end`/`open`/`variable`/`universe`/`attribute` decls.
    pub scopes: u32,
    /// `#check`/`#eval`/`#print`/`#find`/`#guard`/`#reduce` commands.
    pub hash_cmds: u32,
    /// `by` tactic-block keywords.
    pub bys: u32,
    /// `sorry`/`admit`/`stop` holes.
    pub sorries: u32,
}

fn strip(s: &str) -> String {
    // Remove -- line comments and /- -/ blocks (non-nested approximation).
    let mut out = String::with_capacity(s.len());
    let mut depth = 0u32;
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if depth == 0 && b[i] == b'-' && i + 1 < b.len() && b[i + 1] == b'-' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'-' {
            depth += 1;
            i += 2;
            continue;
        }
        if depth > 0 {
            if b[i] == b'-' && i + 1 < b.len() && b[i + 1] == b'/' {
                depth = depth.saturating_sub(1);
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

/// `true` on `import`+decl shape or `#check`/`theorem`/`namespace`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut marks = 0u8;
    for line in s.lines() {
        let t = line.trim_start();
        if t.starts_with("import ") {
            marks |= 1;
        } else if t.starts_with("theorem ") || t.starts_with("namespace ") {
            marks |= 2;
        } else if t.starts_with("#check") || t.starts_with("#eval") || t.starts_with("#print") {
            marks |= 4;
        }
    }
    marks != 0 && marks.count_ones() >= 2 || marks & 4 != 0
}

/// Census; `None` without Lean markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Lean> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let clean = strip(s);
    let mut l = Lean::default();
    for line in clean.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("import ") {
            l.imports += 1;
            l.import_names += rest.split_ascii_whitespace().count() as u32;
            continue;
        }
        if t.starts_with('#') {
            l.hash_cmds += 1;
            continue;
        }
        for w in t.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')) {
            match w {
                "theorem" => l.theorems += 1,
                "def" | "abbrev" | "opaque" => l.defs += 1,
                "inductive" | "structure" | "class" => l.inductives += 1,
                "instance" | "axiom" | "noncomputable" => l.instances += 1,
                "example" => l.examples += 1,
                "namespace" | "section" | "end" | "open" | "variable" | "universe"
                | "attribute" => l.scopes += 1,
                "by" => l.bys += 1,
                "sorry" | "admit" | "stop" => l.sorries += 1,
                _ => {}
            }
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"-- comment\nimport Mathlib.Tactic\nimport Lean\n\nnamespace N\n\
theorem t : True := by\n  trivial\ndef f : Nat := 1\ninductive C where | mk\n\
instance : Inhabited C := sorry\nexample : True := trivial\n#check t\nsorry\nend N\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"#check Nat\n")); // hash cmd alone
        assert!(!detect(b"import os\n")); // python-ish single mark
    }

    #[test]
    fn parses() {
        let l = parse(D).unwrap();
        assert_eq!(l.imports, 2);
        assert_eq!(l.import_names, 2);
        assert_eq!(l.theorems, 1);
        assert_eq!(l.defs, 1);
        assert_eq!(l.inductives, 1);
        assert_eq!(l.instances, 1);
        assert_eq!(l.examples, 1);
        assert_eq!(l.hash_cmds, 1);
        assert_eq!(l.bys, 1);
        assert_eq!(l.sorries, 2);
        assert!(l.scopes >= 2); // namespace + end
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello world\n").is_none());
    }
}

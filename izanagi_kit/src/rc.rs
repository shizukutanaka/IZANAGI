//! Windows resource script `.rc` scanner.
//!
//! `.rc` files are text files of resource statements: `NAME TYPE …`
//! triples plus block keywords (`DIALOG`, `MENU`, `DIALOGEX`,
//! `VERSIONINFO`, `STRINGTABLE`, `ACCELERATORS`, `ICON`, `CURSOR`,
//! `BITMAP`, `FONT`, `RCDATA`, `MESSAGETABLE`, `TOOLBAR`, `HTML`,
//! `MANIFEST`) and `#include`/`LANGUAGE` directives.
//!
//! ```
//! let d = b"#include <windows.h>\n1 ICON \"a.ico\"\nIDD_MAIN DIALOG 0,0,100,50\nBEGIN\nEND\n";
//! let r = izanagi_kit::rc::parse(d).unwrap();
//! assert_eq!(r.includes, 1);
//! assert_eq!(r.get("ICON"), Some(&1));
//! assert_eq!(r.get("DIALOG"), Some(&1));
//! ```
//!
//! Reference: Microsoft `RC File` documentation — the resource-type
//! keyword set and `BEGIN`/`END` block structure.

/// Parsed `.rc` statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Rc {
    /// `#include` directive count.
    pub includes: usize,
    /// `LANGUAGE` directive count.
    pub languages: usize,
    /// Resource-type keyword → occurrence count (e.g. `ICON`, `DIALOG`).
    pub resources: Vec<(String, usize)>,
    /// `BEGIN`/`END` block pair count.
    pub blocks: usize,
}

const TYPES: &[&str] = &[
    "DIALOG",
    "DIALOGEX",
    "MENU",
    "MENUEX",
    "VERSIONINFO",
    "STRINGTABLE",
    "ACCELERATORS",
    "ICON",
    "CURSOR",
    "BITMAP",
    "FONT",
    "RCDATA",
    "MESSAGETABLE",
    "TOOLBAR",
    "HTML",
    "MANIFEST",
    "TEXTINCLUDE",
];

/// Parse a `.rc` script; `None` when no resource statement exists.
pub fn parse(d: &[u8]) -> Option<Rc> {
    let text = core::str::from_utf8(d).ok()?;
    let mut includes = 0usize;
    let mut languages = 0usize;
    let mut blocks = 0usize;
    let mut counts: Vec<(String, usize)> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("#include") {
            includes += 1;
            continue;
        }
        if line.starts_with("#") {
            continue;
        }
        if line.starts_with("LANGUAGE") {
            languages += 1;
            continue;
        }
        if line == "BEGIN" {
            blocks += 1;
            continue;
        }
        if line == "END" || line == "{" || line == "}" {
            continue;
        }
        // `<id-or-name> <TYPE> …` — the TYPE keyword is the second word.
        let mut it = line.split_whitespace();
        it.next();
        if let Some(ty) = it.next() {
            if TYPES.contains(&ty) {
                match counts.iter_mut().find(|(k, _)| k == ty) {
                    Some((_, n)) => *n += 1,
                    None => counts.push((ty.to_string(), 1)),
                }
                continue;
            }
        }
        // TYPE-first form for DIALOG/MENU blocks is also accepted: a bare
        // keyword at line start (e.g. `STRINGTABLE` alone).
        if TYPES.contains(&line) {
            match counts.iter_mut().find(|(k, _)| *k == line) {
                Some((_, n)) => *n += 1,
                None => counts.push((line.to_string(), 1)),
            }
        }
    }
    if counts.is_empty() {
        return None;
    }
    Some(Rc {
        includes,
        languages,
        resources: counts,
        blocks,
    })
}

impl Rc {
    /// Count for one resource-type keyword.
    pub fn get(&self, ty: &str) -> Option<&usize> {
        self.resources.iter().find(|(k, _)| k == ty).map(|(_, n)| n)
    }
}

/// `true` if the buffer looks like a `.rc` script.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"#include <windows.h>\nLANGUAGE LANG_ENGLISH, SUBLANG_DEFAULT\n1 ICON \"a.ico\"\n2 BITMAP \"b.bmp\"\nIDD_MAIN DIALOG 0,0,100,50\nBEGIN\nEND\nSTRINGTABLE\nBEGIN\n1 \"x\"\nEND\n";

    #[test]
    fn parses() {
        let r = parse(DOC).unwrap();
        assert_eq!(r.includes, 1);
        assert_eq!(r.languages, 1);
        assert_eq!(r.get("ICON"), Some(&1));
        assert_eq!(r.get("BITMAP"), Some(&1));
        assert_eq!(r.get("DIALOG"), Some(&1));
        assert_eq!(r.get("STRINGTABLE"), Some(&1));
        assert_eq!(r.blocks, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"just text\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"#include <x>"));
    }
}

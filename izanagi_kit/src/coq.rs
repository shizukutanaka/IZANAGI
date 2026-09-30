//! Coq/Rocq `.v` vernacular file census.
//!
//! Vernacular commands end in `.`: `Theorem|Lemma|Proof.|Qed.|Definition|
//! Inductive|Fixpoint|Module|Section|Require Import|From … Require|
//! Instance|Class|Notation|Hint|Ltac`. Comments `(* … *)`.
//!
//! ```
//! let d = b"From Coq Require Import List.\nModule M.\nSection S.\n\
//! Lemma l : True. Proof. exact I. Qed.\nEnd S.\nEnd M.\n";
//! let c = izanagi_kit::coq::parse(d).unwrap();
//! assert_eq!(c.proofs, 1);
//! assert_eq!(c.lemmas, 1);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Coq {
    /// `Theorem|Lemma|Fact|Remark|Corollary|Proposition|Example` commands.
    pub lemmas: u32,
    /// `Proof.` openings.
    pub proofs: u32,
    /// `Qed.`/`Defined.`/`Admitted.`/`Abort.` closers.
    pub closed: u32,
    /// `Admitted`/`admit`/`give_up` holes.
    pub admitted: u32,
    /// `Definition|Example|Let` definitional commands (non-lemma subset).
    pub definitions: u32,
    /// `Inductive|CoInductive|Variant|Record|Structure` type decls.
    pub inductives: u32,
    /// `Fixpoint|CoFixpoint|Function` recursive decls.
    pub fixpoints: u32,
    /// `Module`/`Module Type`/`Section`/`End`/`Instance`/`Class`/`Context`/`Variable` commands.
    pub modules: u32,
    /// `Require`/`Import`/`Export`/`From … Require`/`Load` commands.
    pub requires: u32,
    /// `Notation`/`Hint`/`Ltac`/`Tactic`/`Set`/`Unset`/`Axiom`/`Parameter`/`Hypothesis` other vernac.
    pub other: u32,
}

const CMDS: &[(&str, u8)] = &[
    ("Theorem", 1),
    ("Lemma", 1),
    ("Fact", 1),
    ("Remark", 1),
    ("Corollary", 1),
    ("Proposition", 1),
    ("Proof", 2),
    ("Qed", 3),
    ("Defined", 3),
    ("Admitted", 4),
    ("Abort", 3),
    ("admit", 4),
    ("Definition", 5),
    ("Let", 5),
    ("Example", 5),
    ("Inductive", 6),
    ("CoInductive", 6),
    ("Variant", 6),
    ("Record", 6),
    ("Structure", 6),
    ("Fixpoint", 7),
    ("CoFixpoint", 7),
    ("Function", 7),
    ("Module", 8),
    ("Section", 8),
    ("End", 8),
    ("Instance", 8),
    ("Class", 8),
    ("Context", 8),
    ("Variable", 8),
    ("Variables", 8),
    ("Hypothesis", 8),
    ("Hypotheses", 8),
    ("Require", 9),
    ("Import", 9),
    ("Export", 9),
    ("From", 9),
    ("Load", 9),
    ("Axiom", 0),
    ("Parameter", 0),
    ("Parameters", 0),
    ("Notation", 0),
    ("Hint", 0),
    ("Ltac", 0),
    ("Tactic", 0),
    ("Set", 0),
    ("Unset", 0),
    ("Scheme", 0),
    ("Goal", 0),
];

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
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
                depth = depth.saturating_sub(1);
            }
            continue;
        }
        out.push(c as char);
    }
    out
}

fn cmd_kind(w: &str) -> Option<u8> {
    CMDS.iter().find(|(k, _)| *k == w).map(|(_, g)| *g)
}

/// `true` on `Proof.`/`Qed.` style vernacular or `Require` decls.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let clean = strip_comments(s);
    let mut lemma = false;
    let mut close = false;
    for w in clean.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        match w {
            "Lemma" | "Theorem" | "Proof" | "Goal" => lemma = true,
            "Qed" | "Defined" | "Admitted" => close = true,
            _ => {}
        }
    }
    lemma && close
}

/// Census; `None` without vernacular commands.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Coq> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let clean = strip_comments(s);
    let mut c = Coq::default();
    for w in clean.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        let Some(g) = cmd_kind(w) else {
            continue;
        };
        match g {
            1 => c.lemmas += 1,
            2 => c.proofs += 1,
            3 => c.closed += 1,
            4 => c.admitted += 1,
            5 => c.definitions += 1,
            6 => c.inductives += 1,
            7 => c.fixpoints += 1,
            8 => c.modules += 1,
            9 => c.requires += 1,
            _ => c.other += 1,
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"(* header *)\nFrom Coq Require Import List.\nModule M.\n\
Section S.\nVariable x : nat.\nInductive t := A | B.\nFixpoint f n := n.\n\
Lemma l : True. Proof. exact I. Qed.\nDefinition d := 0.\n\
Theorem t2 : False. Admitted.\nEnd S.\nEnd M.\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"Lemma l : True.\n")); // no closer
        assert!(!detect(b"plain text\n"));
    }

    #[test]
    fn parses() {
        let c = parse(D).unwrap();
        assert_eq!(c.lemmas, 2); // Lemma + Theorem
        assert_eq!(c.proofs, 1);
        assert_eq!(c.closed, 1); // Qed
        assert_eq!(c.admitted, 1);
        assert_eq!(c.definitions, 1);
        assert_eq!(c.inductives, 1);
        assert_eq!(c.fixpoints, 1);
        assert_eq!(c.requires, 3); // From Require Import
        assert!(c.modules >= 5); // Module Section Variable End End
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"Proof. Qed. (* in comment? *)\n").is_some()); // still coq-ish
    }
}

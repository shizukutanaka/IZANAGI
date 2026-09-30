//! TLA+ specification (`*.tla`) census.
//!
//! A TLA+ module is `---- MODULE Name ----` … `====` with `EXTENDS`,
//! `CONSTANTS`/`CONSTANT`, `VARIABLES`/`VARIABLE`, `ASSUME`, `INIT`,
//! `NEXT`, `SPEC`, `INVARIANT`/`PROPERTY`, `THEOREM`/`LEMMA`/`PROOF`,
//! `INSTANCE`, `LOCAL`, operator definitions `Name(args) == expr`,
//! `<<...>>` tuples, `[a |-> b]`/`[x \in S |-> e]` functions, `\*`/`(*`
//! comments and temporal operators `[]`/`<>`/`~>`/`\A`/`\E`.
//!
//! ```rust
//! let t = concat!(
//!     "---- MODULE Counter ----\n",
//!     "EXTENDS Naturals\n",
//!     "VARIABLE n\n",
//!     "Init == n = 0\n",
//!     "Next == n' = n + 1\n",
//!     "Spec == Init /\\ [][Next]_n\n",
//!     "====\n",
//! );
//! let c = izanagi_kit::tlaplus::Tlaplus::parse(t.as_bytes()).unwrap();
//! assert_eq!(c.operators, 3);
//! ```

/// TLA+ module census.
#[derive(Debug, Clone)]
pub struct Tlaplus {
    /// `---- MODULE` header and `====` footer lines.
    pub modules: usize,
    /// `EXTENDS`/`INSTANCE`/`LOCAL` keyword lines.
    pub extends: usize,
    /// `CONSTANTS`/`CONSTANT`/`VARIABLES`/`VARIABLE` declaration lines.
    pub vars: usize,
    /// `ASSUME`/`AXIOM`/`THEOREM`/`LEMMA`/`COROLLARY`/`PROPOSITION`/`PROOF`/`OBVIOUS`/`BY `/`SUFFICES`/`CASE`/`QED`/`PICK`/`WITNESS`/`DEFINE`/`HAVE`/`TAKE` proof constructs.
    pub proofs: usize,
    /// `Name ==`/`Name(args) ==` operator/function definitions.
    pub operators: usize,
    /// `Init`/`Next`/`Spec`/`Invariant`/`TypeOK` standard names referenced in definitions.
    pub standards: usize,
    /// `[]`/`<>`/`~>`/`<>[]`/`[]<>` temporal operators.
    pub temporals: usize,
    /// `\A`/`\E`/`\in`/`\\/`/`/\\`/`\/`/`=>`/`<=`/`/=` set/logic tokens.
    pub logic: usize,
}

/// Whether the buffer looks like a TLA+ module.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("MODULE") && t.contains("----") && (t.contains("EXTENDS") || t.contains("=="))
        || t.contains("---- MODULE")
}

impl Tlaplus {
    /// Parse a TLA+ module into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            modules: 0,
            extends: 0,
            vars: 0,
            proofs: 0,
            operators: 0,
            standards: 0,
            temporals: 0,
            logic: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("\\*") || s.starts_with("(*") {
                continue;
            }
            c.temporals +=
                s.matches("[]").count() + s.matches("<>").count() + s.matches("~>").count();
            c.logic += s.matches("\\A").count()
                + s.matches("\\E").count()
                + s.matches("\\in").count()
                + s.matches("/\\").count()
                + s.matches("\\/").count()
                + s.matches("/=").count();
            if s.starts_with("----") && s.contains("MODULE") {
                c.modules += 1;
                continue;
            }
            if s.starts_with("====") {
                c.modules += 1;
                continue;
            }
            if s.starts_with("EXTENDS") || s.starts_with("INSTANCE") || s.starts_with("LOCAL") {
                c.extends += 1;
                continue;
            }
            if s.starts_with("CONSTANT")
                || s.starts_with("VARIABLE")
                || s.starts_with("STATE")
                || s.starts_with("ACTION")
            {
                c.vars += 1;
                continue;
            }
            if s.starts_with("ASSUME")
                || s.starts_with("AXIOM")
                || s.starts_with("THEOREM")
                || s.starts_with("LEMMA")
                || s.starts_with("COROLLARY")
                || s.starts_with("PROPOSITION")
                || s.starts_with("PROOF")
                || s.starts_with("OBVIOUS")
                || s.starts_with("BY ")
                || s.starts_with("SUFFICES")
                || s.starts_with("CASE")
                || s.starts_with("QED")
                || s.starts_with("PICK")
                || s.starts_with("WITNESS")
                || s.starts_with("DEFINE")
                || s.starts_with("HAVE")
                || s.starts_with("TAKE")
                || s.starts_with("USE")
            {
                c.proofs += 1;
                continue;
            }
            if s.contains("==") {
                c.operators += 1;
                let head = s.split([' ', '(']).next().unwrap_or("");
                if ["Init", "Next", "Spec", "TypeOK", "Invariant", "Inv"].contains(&head) {
                    c.standards += 1;
                }
                continue;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_module() {
        let b = concat!(
            "\\* comment\n",
            "---- MODULE Counter ----\n",
            "EXTENDS Naturals\n",
            "VARIABLE n\n",
            "CONSTANT Max\n",
            "Init == n = 0\n",
            "Next == n' = n + 1\n",
            "Spec == Init /\\ [][Next]_n\n",
            "Inv == n \\in Nat\n",
            "THEOREM Spec => []Inv\n",
            "PROOF\n",
            "====\n",
        );
        let c = Tlaplus::parse(b.as_bytes()).unwrap();
        assert_eq!(c.modules, 2);
        assert_eq!(c.extends, 1);
        assert_eq!(c.vars, 2);
        assert_eq!(c.operators, 4);
        assert_eq!(c.standards, 4);
        assert_eq!(c.proofs, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Tlaplus::parse(b"foo = 1").is_none());
    }
}

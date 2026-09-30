//! B/Event-B machine (`.mch`/`.ref`/`.imp`) census.
//!
//! A B machine file has a `MACHINE`/`REFINEMENT`/`IMPLEMENTATION` `<name>`
//! header plus `CONSTRAINTS`/`SETS`/`CONSTANTS`/`ABSTRACT_CONSTANTS`/`PROPERTIES`/
//! `VARIABLES`/`ABSTRACT_VARIABLES`/`CONCRETE_VARIABLES`/`INVARIANT`/
//! `ASSERTIONS`/`DEFINITIONS`/`INITIALISATION`/`OPERATIONS`/`EVENTS`/
//! `VARIANT` clauses, `SEES`/`USES`/`INCLUDES`/`EXTENDS`/`PROMOTES`/
//! `REFINES`/`IMPORTS`/`VALUES`/`CONCRETE_CONSTANTS` dependencies,
//! `PRE`/`THEN`/`WHEN`/`WHERE`/`ANY`/`SELECT`/`CHOICE`/`IF`/`CASE`/`OR`/
//! `LET`/`BE`/`BEGIN`/`BEGIN` substitutions, `:=`/`||`/`;`/`=`/`:`/
//! `<:`/`<<:`/`<=>`/`=>`/`/\`/`\/`/`not`/`NAT`/`INT`/`BOOL`/`POW`/`SEQ`/
//! `NATURAL`/`INTEGER`/`STRING`/`FIN`/`INTER`/`UNION`/`SIGMA`/`PI`/
//! `skip`/`PRE`/`ANY x WHERE`/`ASSERT`/`CHOICE`/`SELECT`/`CASE`/`WHILE`/
//! `INVARIANT`/`VARIANT`/`MODIFIES`/`ELSIF`/`ELSE`.
//!
//! ```rust
//! let m = concat!(
//!     "MACHINE Counter\n",
//!     "VARIABLES n\n",
//!     "INVARIANT n : NAT\n",
//!     "INITIALISATION n := 0\n",
//!     "OPERATIONS\n",
//!     "  inc = BEGIN n := n + 1 END\n",
//!     "END\n",
//! );
//! let c = izanagi_kit::mch::Mch::parse(m.as_bytes()).unwrap();
//! assert_eq!(c.machines, 1);
//! ```

const CLAUSES: &[&str] = &[
    "MACHINE",
    "REFINEMENT",
    "IMPLEMENTATION",
    "MODEL",
    "SYSTEM",
    "CONSTRAINTS",
    "SETS",
    "CONSTANTS",
    "ABSTRACT_CONSTANTS",
    "CONCRETE_CONSTANTS",
    "PROPERTIES",
    "VARIABLES",
    "ABSTRACT_VARIABLES",
    "CONCRETE_VARIABLES",
    "INVARIANT",
    "ASSERTIONS",
    "DEFINITIONS",
    "INITIALISATION",
    "OPERATIONS",
    "EVENTS",
    "VARIANT",
    "SEES",
    "USES",
    "INCLUDES",
    "EXTENDS",
    "PROMOTES",
    "REFINES",
    "IMPORTS",
    "VALUES",
    "END",
];

const SUBST: &[&str] = &[
    "PRE", "THEN", "WHEN", "WHERE", "ANY", "SELECT", "CHOICE", "IF", "CASE", "OR", "LET", "BE",
    "BEGIN", "ELSIF", "ELSE", "WHILE", "MODIFIES", "IN", "ASSERT", "NOT", "EITHER", "SELECT",
    "CASE",
];

/// B/Event-B machine census.
#[derive(Debug, Clone)]
pub struct Mch {
    /// `MACHINE`/`REFINEMENT`/`IMPLEMENTATION`/`MODEL`/`SYSTEM` headers.
    pub machines: usize,
    /// `SETS`/`CONSTANTS`/`VARIABLES`/`INVARIANT`/`INITIALISATION`/`OPERATIONS`/`EVENTS`/`VARIANT`/`ASSERTIONS`/`PROPERTIES`/`CONSTRAINTS`/`DEFINITIONS` clause lines.
    pub clauses: usize,
    /// `SEES`/`USES`/`INCLUDES`/`EXTENDS`/`PROMOTES`/`REFINES`/`IMPORTS`/`VALUES` dependency lines.
    pub deps: usize,
    /// `PRE`/`THEN`/`WHEN`/`WHERE`/`ANY`/`SELECT`/`CHOICE`/`IF`/`CASE`/`OR`/`LET`/`BE`/`BEGIN`/`ELSIF`/`ELSE`/`WHILE`/`MODIFIES` substitution headers.
    pub substitutions: usize,
    /// `:=`/`;`/`||`/`<|`/`<:` assignments + parallel-comp operators.
    pub assigns: usize,
    /// `:`/`<:`/`<<:`/`=`/`/=`/`<=>`/`=>`/`/\`/`\/`/`not` predicates + `NAT`/`INT`/`BOOL`/`POW`/`SEQ`/`NATURAL`/`INTEGER`/`STRING`/`FIN`/`INTER`/`UNION`/`SIGMA`/`PI` type atoms.
    pub logic: usize,
}

/// Whether the buffer looks like a B/Event-B machine.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        let s = l.trim();
        s.starts_with("MACHINE") || s.starts_with("REFINEMENT") || s.starts_with("IMPLEMENTATION")
    }) && (t.contains("VARIABLES")
        || t.contains("SETS")
        || t.contains("OPERATIONS")
        || t.contains("INVARIANT")
        || t.contains("EVENTS"))
}

impl Mch {
    /// Parse a B/Event-B machine into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            machines: 0,
            clauses: 0,
            deps: 0,
            substitutions: 0,
            assigns: 0,
            logic: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("//") || s.starts_with("/*") {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if ["MACHINE", "REFINEMENT", "IMPLEMENTATION", "MODEL", "SYSTEM"].contains(&head) {
                c.machines += 1;
                continue;
            }
            if [
                "SEES", "USES", "INCLUDES", "EXTENDS", "PROMOTES", "REFINES", "IMPORTS", "VALUES",
            ]
            .contains(&head)
            {
                c.deps += 1;
                continue;
            }
            if CLAUSES.contains(&head) {
                c.clauses += 1;
                continue;
            }
            if SUBST.contains(&head) {
                c.substitutions += 1;
                continue;
            }
            c.substitutions += s
                .split_whitespace()
                .filter(|w| SUBST.contains(&w.trim_end_matches(';')))
                .count();
            c.assigns +=
                s.matches(":=").count() + s.matches("||").count() + s.matches("<|").count();
            c.logic += s.matches(" : ").count()
                + s.matches("<=").count()
                + s.matches("=>").count()
                + s.matches("<=>").count()
                + s.matches("/=").count()
                + s.matches("/\\").count()
                + s.matches("\\/").count()
                + s.matches("not ").count()
                + s.matches("NAT").count()
                + s.matches("INT").count()
                + s.matches("BOOL").count()
                + s.matches("POW").count()
                + s.matches("SEQ").count()
                + s.matches("NATURAL").count()
                + s.matches("INTEGER").count()
                + s.matches("STRING").count()
                + s.matches("FIN").count()
                + s.matches("INTER").count()
                + s.matches("UNION").count()
                + s.matches("SIGMA").count()
                + s.matches("PI").count();
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_machine() {
        let b = concat!(
            "MACHINE Counter\n",
            "SEES Ctx\n",
            "VARIABLES n\n",
            "INVARIANT n : NAT\n",
            "INITIALISATION n := 0\n",
            "OPERATIONS\n",
            "  inc = BEGIN n := n + 1 END;\n",
            "  add(x) = PRE x : NAT THEN n := n + x END\n",
            "END\n",
        );
        let c = Mch::parse(b.as_bytes()).unwrap();
        assert_eq!(c.machines, 1);
        assert_eq!(c.deps, 1);
        assert_eq!(c.clauses, 5);
        assert_eq!(c.substitutions, 3);
        assert_eq!(c.assigns, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Mch::parse(b"foo = 1").is_none());
    }
}

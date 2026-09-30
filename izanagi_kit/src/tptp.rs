//! TPTP (Thousands of Problems for Theorem Provers) file census.
//!
//! TPTP problems are a sequence of annotated formula declarations
//! `kind(name, role, formula).` where `kind` is one of
//! `fof`/`cnf`/`tff`/`thf`/`tfa`/`tpi`/`ddt`, plus `include('path').`
//! directives and `%` comments.
//!
//! ```
//! let d = b"% a problem\nfof(a1, axiom, p).\ncnf(c1, conjecture, ~ q).\n";
//! let t = izanagi_kit::tptp::parse(d).unwrap();
//! assert_eq!(t.axioms, 1);
//! assert_eq!(t.conjectures, 1);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tptp {
    /// `cnf(...)` declarations.
    pub cnf: u32,
    /// `fof(...)` declarations.
    pub fof: u32,
    /// `tff(...)` declarations (typed first-order).
    pub tff: u32,
    /// `thf(...)` declarations (typed higher-order).
    pub thf: u32,
    /// `tfa`/`tpi`/`ddt` declarations combined.
    pub other_kinds: u32,
    /// Declarations whose role is `axiom`/`hypothesis`/`definition`/`assumption`.
    pub axioms: u32,
    /// Declarations whose role is `conjecture`/`negated_conjecture`/`lemma`/`theorem`.
    pub conjectures: u32,
    /// `include(...)` directives.
    pub includes: u32,
}

const KINDS: &[&str] = &["cnf", "fof", "tff", "thf", "tfa", "tpi", "ddt"];

fn decl_kind(t: &str) -> Option<&'static str> {
    let open = t.find('(')?;
    let k = &t[..open];
    KINDS.iter().find(|x| *x == &k).copied()
}

fn role_of(t: &str) -> &str {
    // `kind(name, role,` — role is the second comma-separated field.
    let mut commas = t.match_indices(',');
    let Some((c1, _)) = commas.next() else {
        return "";
    };
    let Some((c2, _)) = commas.next() else {
        return "";
    };
    t[c1 + 1..c2].trim()
}

fn is_axiom_role(r: &str) -> bool {
    matches!(
        r,
        "axiom" | "hypothesis" | "definition" | "assumption" | "type"
    )
}

fn is_goal_role(r: &str) -> bool {
    matches!(
        r,
        "conjecture" | "negated_conjecture" | "lemma" | "theorem" | "question"
    )
}

/// `true` on ≥1 `kind(name,` declaration line with a known kind.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.lines().any(|l| decl_kind(l.trim_start()).is_some())
}

/// Census; `None` without a TPTP declaration.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Tptp> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut t = Tptp::default();
    for line in s.lines() {
        let t_ = line.trim_start();
        if t_.starts_with("include(") {
            t.includes += 1;
            continue;
        }
        let Some(k) = decl_kind(t_) else {
            continue;
        };
        match k {
            "cnf" => t.cnf += 1,
            "fof" => t.fof += 1,
            "tff" => t.tff += 1,
            "thf" => t.thf += 1,
            _ => t.other_kinds += 1,
        }
        let r = role_of(t_);
        if is_axiom_role(r) {
            t.axioms += 1;
        }
        if is_goal_role(r) {
            t.conjectures += 1;
        }
    }
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"% header\ninclude('axioms.ax').\n\
fof(a1, axiom, ! [X] : p(X)).\n\
tff(t1, type, p: $tType).\n\
cnf(c1, conjecture, ~ p(a)).\n\
thf(h1, lemma, t @ f).\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"fofx(a1, axiom, p).\n"));
        assert!(!detect(b"just text\n"));
    }

    #[test]
    fn parses() {
        let t = parse(D).unwrap();
        assert_eq!(t.fof, 1);
        assert_eq!(t.tff, 1);
        assert_eq!(t.cnf, 1);
        assert_eq!(t.thf, 1);
        assert_eq!(t.axioms, 2); // axiom + type role
        assert_eq!(t.conjectures, 2); // conjecture + lemma
        assert_eq!(t.includes, 1);
    }

    #[test]
    fn roles() {
        assert_eq!(role_of("fof(n, axiom, x)."), "axiom");
        assert_eq!(
            role_of("cnf(n,negated_conjecture, x)."),
            "negated_conjecture"
        );
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"% only a comment\n").is_none());
    }
}

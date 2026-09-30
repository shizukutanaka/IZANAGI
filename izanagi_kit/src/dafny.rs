//! Dafny program (`.dfy`) census.
//!
//! A Dafny file has `method`/`function`/`predicate`/`lemma`/`datatype`/
//! `codatatype`/`class`/`trait`/`type`/`module`/`import`/`include`/
//! `export`/`iterator`/`newtype`/`subset type`/`const`/`var`/`ghost`/
//! `static`/`returns`/`requires`/`ensures`/`invariant`/`decreases`/
//! `reads`/`modifies`/`assert`/`assume`/`expect`/`print`/`if`/`else`/
//! `while`/`for`/`match`/`calc`/`by`/`forall`/`exists`/`old`/`fresh`/
//! `allocated` constructs.
//!
//! ```rust
//! let d = concat!(
//!     "method Abs(x: int) returns (r: int)\n",
//!     "    ensures r >= 0\n",
//!     "{\n",
//!     "    if x < 0 { return -x; }\n",
//!     "    return x;\n",
//!     "}\n",
//! );
//! let c = izanagi_kit::dafny::Dafny::parse(d.as_bytes()).unwrap();
//! assert_eq!(c.methods, 1);
//! ```

/// Dafny program census.
#[derive(Debug, Clone)]
pub struct Dafny {
    /// `module`/`import`/`include`/`export`/`opened`/`refines` lines.
    pub modules: usize,
    /// `method`/`constructor`/`function`/`predicate`/`copredicate`/`lemma`/`colemma`/`twostate`/`inductive`/`opaque`/`least`/`greatest`/`axiom` declarations.
    pub methods: usize,
    /// `datatype`/`codatatype`/`class`/`trait`/`type`/`newtype`/`subset type`/`iterator`/`const`/`var`/`ghost`/`static` declarations.
    pub types: usize,
    /// `requires`/`ensures`/`invariant`/`decreases`/`reads`/`modifies`/`frame`/`calc`/`forall`/`exists` contracts/specs.
    pub specs: usize,
    /// `assert`/`assume`/`expect`/`print`/`label` statements.
    pub assertions: usize,
    /// `if`/`else`/`while`/`for`/`foreach`/`match`/`case`/`return`/`yield`/`break`/`continue`/`then`/`by` flow statements.
    pub flows: usize,
    /// `old`/`fresh`/`allocated`/`unchanged`/`in`/`as`/`is`/`null`/`this`/`true`/`false` expression tokens.
    pub exprs: usize,
}

/// Whether the buffer looks like a Dafny program.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("method ") && t.contains("ensures"))
        || (t.contains("lemma ") || t.contains("datatype "))
        || (t.contains("requires") && t.contains("decreases"))
}

impl Dafny {
    /// Parse a Dafny program into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            modules: 0,
            methods: 0,
            types: 0,
            specs: 0,
            assertions: 0,
            flows: 0,
            exprs: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("//") || s == "{" || s == "}" {
                continue;
            }
            let head = s
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('{');
            if head.is_empty() {
                continue;
            }
            if ["module", "import", "include", "export", "opened", "refines"].contains(&head) {
                c.modules += 1;
                continue;
            }
            if [
                "method",
                "constructor",
                "function",
                "predicate",
                "copredicate",
                "lemma",
                "colemma",
                "twostate",
                "inductive",
                "opaque",
                "least",
                "greatest",
                "axiom",
            ]
            .contains(&head)
            {
                c.methods += 1;
                continue;
            }
            if [
                "datatype",
                "codatatype",
                "class",
                "trait",
                "type",
                "newtype",
                "iterator",
                "const",
                "var",
                "ghost",
                "static",
                "subset",
            ]
            .contains(&head)
            {
                c.types += 1;
                continue;
            }
            if [
                "requires",
                "ensures",
                "invariant",
                "decreases",
                "reads",
                "modifies",
                "frame",
                "calc",
                "forall",
                "exists",
            ]
            .contains(&head)
            {
                c.specs += 1;
                continue;
            }
            if ["assert", "assume", "expect", "print", "label"].contains(&head) {
                c.assertions += 1;
                continue;
            }
            if [
                "if", "else", "while", "for", "foreach", "match", "case", "return", "yield",
                "break", "continue", "then", "by",
            ]
            .contains(&head)
            {
                c.flows += 1;
                continue;
            }
            if [
                "old",
                "fresh",
                "allocated",
                "unchanged",
                "in",
                "as",
                "is",
                "null",
                "this",
                "true",
                "false",
                "abstemious",
            ]
            .contains(&head)
            {
                c.exprs += 1;
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
    fn parses_program() {
        let b = concat!(
            "module M {\n",
            "import opened Util\n",
            "datatype List = Nil | Cons(head: int, tail: List)\n",
            "class C { var f: int }\n",
            "method Abs(x: int) returns (r: int)\n",
            "    ensures r >= 0\n",
            "    requires x == x\n",
            "{\n",
            "    if x < 0 { return -x; }\n",
            "    assert r >= 0;\n",
            "    return x;\n",
            "}\n",
            "lemma L(x: int)\n",
            "    ensures x == x\n",
            "{}\n",
            "}\n",
        );
        let c = Dafny::parse(b.as_bytes()).unwrap();
        assert_eq!(c.modules, 2);
        assert_eq!(c.methods, 2);
        assert_eq!(c.types, 2);
        assert_eq!(c.specs, 3);
        assert_eq!(c.assertions, 1);
        assert_eq!(c.flows, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Dafny::parse(b"foo = 1").is_none());
    }
}

//! Alloy model (`.als`) census.
//!
//! An Alloy model is `module name` + `sig`/`abstract sig`/`lone sig`/
//! `one sig`/`sig` declarations, `fact`/`pred`/`fun`/`assert`/`check`/
//! `run`, `extends`/`in`, `open` imports, `let`/`=>`/`iff`/`and`/`or`/
//! `not`/`all`/`some`/`no`/`lone`/`disj` quantifiers, `->` relations,
//! `+`/`-`/`&`/`.`/`~`/`^`/`*` set operators and `{}` blocks.
//!
//! ```rust
//! let a = concat!(
//!     "module l3\n",
//!     "sig Node {}\n",
//!     "sig List {}\n",
//!     "fact { all n: Node | n != n }\n",
//!     "assert a1 { no n: Node | n in n.*next }\n",
//!     "check a1 for 5\n",
//! );
//! let c = izanagi_kit::alloy::Alloy::parse(a.as_bytes()).unwrap();
//! assert_eq!(c.sigs, 2);
//! ```

/// Alloy model census.
#[derive(Debug, Clone)]
pub struct Alloy {
    /// `module`/`open`/`private` lines.
    pub modules: usize,
    /// `sig`/`abstract sig`/`lone sig`/`one sig`/`sig`/`enum`/`var sig` declarations.
    pub sigs: usize,
    /// `fact`/`pred`/`fun`/`assert`/`check`/`run`/`expect`/`inst`/`as` blocks.
    pub blocks: usize,
    /// `extends`/`in`/`subset`/`partof`/`subsetof` declarations.
    pub inherits: usize,
    /// `all`/`some`/`no`/`lone`/`one`/`set`/`seq` quantifier/multiplicity tokens.
    pub quants: usize,
    /// `->`/`+`/`-`/`&`/`.`/`~`/`^`/`*` relation/set operators.
    pub ops: usize,
    /// `let`/`=>`/`iff`/`implies`/`and`/`or`/`not`/`else`/`if`/`then` expressions.
    pub exprs: usize,
}

/// Whether the buffer looks like an Alloy model.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("sig ") || t.contains("sig{") || t.contains("abstract sig"))
        && (t.contains("fact")
            || t.contains("pred")
            || t.contains("assert")
            || t.contains("run")
            || t.contains("check"))
        || t.contains("module ") && (t.contains("sig") || t.contains("fact") || t.contains("pred"))
}

impl Alloy {
    /// Parse an Alloy model into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            modules: 0,
            sigs: 0,
            blocks: 0,
            inherits: 0,
            quants: 0,
            ops: 0,
            exprs: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("//") || s.starts_with("--") {
                continue;
            }
            c.quants += s.matches(" all ").count()
                + s.matches(" some ").count()
                + s.matches(" no ").count()
                + s.matches(" lone ").count()
                + s.matches(" one ").count()
                + s.matches(" set ").count()
                + s.matches(" seq ").count()
                + s.matches(" disj ").count();
            c.ops += s.matches("->").count()
                + s.matches(" *").count()
                + s.matches(" ^").count()
                + s.matches(" ~").count();
            let head = s.split_whitespace().next().unwrap_or("");
            if head == "module" || head == "open" || head == "private" || head == "as" {
                c.modules += 1;
                continue;
            }
            if head == "sig"
                || head == "abstract"
                || head == "enum"
                || head == "lone"
                || head == "one"
                || head == "var"
                || head == "some" && s.contains("sig")
            {
                c.sigs += 1;
                continue;
            }
            if head == "fact"
                || head == "pred"
                || head == "fun"
                || head == "assert"
                || head == "check"
                || head == "run"
                || head == "expect"
                || head == "inst"
                || head == "idiv"
            {
                c.blocks += 1;
                continue;
            }
            if head == "extends"
                || head == "in"
                || head == "partof"
                || head == "subsetof"
                || head == "subset"
            {
                c.inherits += 1;
                continue;
            }
            if head == "let"
                || head == "=>"
                || head == "iff"
                || head == "implies"
                || head == "and"
                || head == "or"
                || head == "not"
                || head == "else"
                || head == "if"
                || head == "then"
                || head == "for"
                || head == "in "
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
    fn parses_model() {
        let b = concat!(
            "module l3\n",
            "open util/ordering[Node]\n",
            "abstract sig List {}\n",
            "sig Node {}\n",
            "sig List {\n",
            "  header: set Node\n",
            "}\n",
            "fact { all n: Node | n != n }\n",
            "pred show {}\n",
            "assert a1 { no n: Node | n in n.*next }\n",
            "check a1 for 5\n",
        );
        let c = Alloy::parse(b.as_bytes()).unwrap();
        assert_eq!(c.modules, 2);
        assert_eq!(c.sigs, 3);
        assert_eq!(c.blocks, 4);
        assert!(c.quants >= 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Alloy::parse(b"foo = 1").is_none());
    }
}

//! WhyML / Why3 (`.mlw`/`.why`) program census.
//!
//! A WhyML file has `module`/`use`/`clone`/`import`/`export`/`include`/
//! `scope`/`namespace`, `let`/`let rec`/`val`/`predicate`/`function`/
//! `type`/`inductive`/`coinductive`/`constant`/`exception`, `requires`/
//! `ensures`/`invariant`/`variant`/`writes`/`reads`/`raises`/`alias`/
//! `by`/`so`/`check`/`assert`/`assume`/`absurd`, `axiom`/`lemma`/`goal`,
//! `if then else`/`while do done`/`for to downto`/`match with`/`try with`/
//! `begin end`/`abstract`/`ghost`/`pure`, `==>`/`<->`/`\/`/`/\`/`->`
//! logic operators and `forall`/`exists`/`lemma`/`axiom`/`goal`.
//!
//! ```rust
//! let w = concat!(
//!     "module M\n",
//!     "  use int.Int\n",
//!     "  let f (x:int) : int\n",
//!     "    requires x >= 0\n",
//!     "    ensures result >= 0\n",
//!     "  = x\n",
//!     "end\n",
//! );
//! let c = izanagi_kit::whyml::Whyml::parse(w.as_bytes()).unwrap();
//! assert_eq!(c.modules, 2);
//! ```

/// WhyML program census.
#[derive(Debug, Clone)]
pub struct Whyml {
    /// `module`/`use`/`clone`/`import`/`export`/`include`/`scope`/`namespace`/`theory`/`meta` lines.
    pub modules: usize,
    /// `let`/`val`/`predicate`/`function`/`type`/`inductive`/`coinductive`/`constant`/`exception`/`rec`/`fun`/`abstract`/`ghost`/`pure`/`mutable`/`private` declarations.
    pub decls: usize,
    /// `requires`/`ensures`/`invariant`/`variant`/`writes`/`reads`/`raises`/`alias`/`diverges`/`partial` contracts.
    pub contracts: usize,
    /// `axiom`/`lemma`/`goal`/`prop`/`assert`/`assume`/`check`/`absurd`/`by`/`so`/`epsilon`/`any` statements.
    pub assertions: usize,
    /// `if`/`else`/`while`/`for`/`match`/`with`/`try`/`begin`/`end`/`in`/`of`/`as`/`not`/`and`/`or`/`loop`/`break`/`continue`/`return`/`case` control/logic heads.
    pub flows: usize,
    /// `==>`/`<->`/`\/`/`/\`/`->`/`=`/`forall`/`exists` logic operators.
    pub logic: usize,
}

/// Whether the buffer looks like a WhyML file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("module ") && (t.contains("use ") || t.contains("let ")))
        || t.contains("axiom ")
        || t.contains("goal ")
        || (t.contains("requires") && t.contains("ensures"))
}

impl Whyml {
    /// Parse a WhyML file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            modules: 0,
            decls: 0,
            contracts: 0,
            assertions: 0,
            flows: 0,
            logic: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("(*") || s.starts_with("//") {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            c.logic += s.matches("==>").count()
                + s.matches("<->").count()
                + s.matches("\\/").count()
                + s.matches("/\\").count()
                + s.matches("forall").count()
                + s.matches("exists").count();
            if [
                "module",
                "use",
                "clone",
                "import",
                "export",
                "include",
                "scope",
                "namespace",
                "theory",
                "meta",
                "declarations",
            ]
            .contains(&head)
            {
                c.modules += 1;
                continue;
            }
            if [
                "let",
                "val",
                "predicate",
                "function",
                "type",
                "inductive",
                "coinductive",
                "constant",
                "exception",
                "rec",
                "fun",
                "abstract",
                "ghost",
                "pure",
                "mutable",
                "private",
            ]
            .contains(&head)
            {
                c.decls += 1;
                continue;
            }
            if [
                "requires",
                "ensures",
                "invariant",
                "variant",
                "writes",
                "reads",
                "raises",
                "alias",
                "diverges",
                "partial",
            ]
            .contains(&head)
            {
                c.contracts += 1;
                continue;
            }
            if [
                "axiom", "lemma", "goal", "prop", "assert", "assume", "check", "absurd", "by",
                "so", "epsilon", "any",
            ]
            .contains(&head)
            {
                c.assertions += 1;
                continue;
            }
            if [
                "if", "else", "while", "for", "match", "with", "try", "begin", "end", "in", "of",
                "as", "not", "and", "or", "loop", "break", "continue", "return", "case",
            ]
            .contains(&head)
            {
                c.flows += 1;
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
            "module M\n",
            "  use int.Int\n",
            "  use ref.Ref\n",
            "  let f (x:int) : int\n",
            "    requires x >= 0\n",
            "    ensures result >= 0\n",
            "    writes r\n",
            "  = x\n",
            "  predicate p (x:int) = x > 0\n",
            "  lemma L : forall x. p x -> true\n",
            "  val g () : unit\n",
            "  function h (x:int) : int\n",
            "end\n",
        );
        let c = Whyml::parse(b.as_bytes()).unwrap();
        assert_eq!(c.modules, 3);
        assert_eq!(c.decls, 4);
        assert_eq!(c.contracts, 3);
        assert_eq!(c.assertions, 1);
        assert_eq!(c.flows, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Whyml::parse(b"foo = 1").is_none());
    }
}

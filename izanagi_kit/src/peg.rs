//! Packrat parsing for Parsing Expression Grammars (PEGs) — Ford's
//! ordered-choice recognizer with `(rule, position)` memoization.
//!
//! A grammar is a table of [`Expr`] bodies; rule 0 is the start.
//! [`parse`] returns the end position of the longest start-rule match
//! (prefix semantics); [`parse_full`] additionally requires the input
//! to be consumed entirely. Left recursion is detected and fails the
//! offending alternative rather than diverging. Two deliberate
//! totality rules keep the engine safe on arbitrary grammars:
//! `Star`/`Plus` stop when the body matches empty (no-progress break),
//! and `Class` byte sets are sorted+deduplicated at construction.
//!
//! ```
//! use izanagi_kit::peg::{parse_full, Expr, Grammar};
//! // S -> 'a' S 'b' | '' — PEG handles a^n b^n natively.
//! let g = Grammar::new(vec![Expr::Alt(vec![
//!     Expr::Seq(vec![
//!         Expr::Lit(b"a".to_vec()),
//!         Expr::Call(0),
//!         Expr::Lit(b"b".to_vec()),
//!     ]),
//!     Expr::Empty,
//! ])]);
//! assert_eq!(parse_full(&g, b"aaabbb"), Some(()));
//! assert_eq!(parse_full(&g, b"aaabb"), None);
//! ```
//!
//! Reference: Ford (2004), "Parsing Expression Grammars: a
//! recognition-based syntactic foundation" (POPL '04).

/// One PEG expression. `Class` holds a sorted, deduplicated byte set.
#[derive(Clone, Debug)]
pub enum Expr {
    /// Matches the empty string.
    Empty,
    /// Exact byte sequence.
    Lit(Vec<u8>),
    /// One byte from the set.
    Class(Vec<u8>),
    /// One arbitrary byte (fails at end of input).
    AnyByte,
    /// All children in order.
    Seq(Vec<Expr>),
    /// Ordered choice — first match wins.
    Alt(Vec<Expr>),
    /// Zero or one occurrence.
    Opt(Box<Expr>),
    /// Zero or more (greedy, stops on failure or no progress).
    Star(Box<Expr>),
    /// One or more (greedy).
    Plus(Box<Expr>),
    /// Zero-width positive lookahead.
    And(Box<Expr>),
    /// Zero-width negative lookahead.
    Not(Box<Expr>),
    /// Invoke rule `u32` (memoized).
    Call(u32),
}

/// A PEG: rule bodies indexed by `u32`; rule 0 is the start.
pub struct Grammar {
    rules: Vec<Expr>,
}

impl Grammar {
    /// Build a grammar; `rules[0]` is the start symbol.
    pub fn new(rules: Vec<Expr>) -> Grammar {
        Grammar { rules }
    }

    /// Number of rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the grammar has no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Run the start rule over `input`; `Some(end)` on match.
pub fn parse(g: &Grammar, input: &[u8]) -> Option<usize> {
    if g.is_empty() {
        return None;
    }
    let mut ctx = Ctx {
        g,
        input,
        memo: std::collections::BTreeMap::new(),
        active: std::collections::BTreeSet::new(),
        use_memo: true,
    };
    eval_call(&mut ctx, 0, 0)
}

/// [`parse`], requiring the whole input to be consumed.
pub fn parse_full(g: &Grammar, input: &[u8]) -> Option<()> {
    let end = parse(g, input)?;
    if end == input.len() {
        Some(())
    } else {
        None
    }
}

struct Ctx<'a> {
    g: &'a Grammar,
    input: &'a [u8],
    memo: std::collections::BTreeMap<(u32, usize), Option<usize>>,
    /// Rules currently on the recursion stack — left-recursion guard.
    active: std::collections::BTreeSet<(u32, usize)>,
    use_memo: bool,
}

fn eval_call(ctx: &mut Ctx<'_>, rule: u32, pos: usize) -> Option<usize> {
    let key = (rule, pos);
    if ctx.use_memo {
        if let Some(&hit) = ctx.memo.get(&key) {
            return hit;
        }
    }
    // A rule already being evaluated at this position is left
    // recursion — fail this alternative, don't diverge.
    if !ctx.active.insert(key) {
        return None;
    }
    let body = match ctx.g.rules.get(rule as usize) {
        Some(e) => e.clone(),
        None => {
            ctx.active.remove(&key);
            return None;
        }
    };
    let out = eval_expr(ctx, &body, pos);
    ctx.active.remove(&key);
    if ctx.use_memo {
        ctx.memo.insert(key, out);
    }
    out
}

fn eval_expr(ctx: &mut Ctx<'_>, e: &Expr, pos: usize) -> Option<usize> {
    let input = ctx.input;
    match e {
        Expr::Empty => Some(pos),
        Expr::Lit(bs) => {
            if input.len() >= pos + bs.len() && input[pos..pos + bs.len()] == bs[..] {
                Some(pos + bs.len())
            } else {
                None
            }
        }
        Expr::Class(set) => {
            let b = *input.get(pos)?;
            if set.binary_search(&b).is_ok() {
                Some(pos + 1)
            } else {
                None
            }
        }
        Expr::AnyByte => {
            if pos < input.len() {
                Some(pos + 1)
            } else {
                None
            }
        }
        Expr::Seq(parts) => {
            let mut p = pos;
            for part in parts {
                p = eval_expr(ctx, part, p)?;
            }
            Some(p)
        }
        Expr::Alt(arms) => {
            for arm in arms {
                if let Some(p) = eval_expr(ctx, arm, pos) {
                    return Some(p);
                }
            }
            None
        }
        Expr::Opt(inner) => Some(eval_expr(ctx, inner, pos).unwrap_or(pos)),
        Expr::Star(inner) => {
            let mut p = pos;
            while let Some(next) = eval_expr(ctx, inner, p) {
                if next == p {
                    break; // no-progress guard
                }
                p = next;
            }
            Some(p)
        }
        Expr::Plus(inner) => {
            let mut p = eval_expr(ctx, inner, pos)?;
            loop {
                match eval_expr(ctx, inner, p) {
                    Some(next) if next > p => p = next,
                    _ => break,
                }
            }
            Some(p)
        }
        Expr::And(inner) => {
            if eval_expr(ctx, inner, pos).is_some() {
                Some(pos)
            } else {
                None
            }
        }
        Expr::Not(inner) => {
            if eval_expr(ctx, inner, pos).is_some() {
                None
            } else {
                Some(pos)
            }
        }
        Expr::Call(rule) => eval_call(ctx, *rule, pos),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn lit(s: &str) -> Expr {
        Expr::Lit(s.as_bytes().to_vec())
    }

    #[test]
    fn literal_and_class() {
        let g = Grammar::new(vec![Expr::Seq(vec![
            lit("ab"),
            Expr::Class(b"xyz".to_vec()),
        ])]);
        assert_eq!(parse(&g, b"abx"), Some(3));
        assert_eq!(parse(&g, b"aby"), Some(3));
        assert_eq!(parse(&g, b"abq"), None);
        assert_eq!(parse(&g, b"ab"), None);
    }

    #[test]
    fn ordered_choice_is_greedy() {
        // First arm wins even though it leaves input unconsumed.
        let g = Grammar::new(vec![Expr::Alt(vec![lit("ab"), lit("abc")])]);
        assert_eq!(parse(&g, b"abcdef"), Some(2));
        assert_eq!(parse_full(&g, b"ab"), Some(()));
        assert_eq!(parse_full(&g, b"abc"), None);
    }

    #[test]
    fn repetition_and_predicates() {
        // word = [a-z]+ !digit — "word" then not-a-digit.
        let word = Expr::Seq(vec![
            Expr::Plus(Box::new(Expr::Class((b'a'..=b'z').collect()))),
            Expr::Not(Box::new(Expr::Class((b'0'..=b'9').collect()))),
        ]);
        let g = Grammar::new(vec![word]);
        assert_eq!(parse(&g, b"hello"), Some(5));
        assert_eq!(parse(&g, b"abc123"), None);
        // Star with an empty-matching body must terminate.
        let g2 = Grammar::new(vec![Expr::Star(Box::new(Expr::Empty))]);
        assert_eq!(parse(&g2, b"zzz"), Some(0));
    }

    #[test]
    fn anbn() {
        let g = Grammar::new(vec![Expr::Alt(vec![
            Expr::Seq(vec![lit("a"), Expr::Call(0), lit("b")]),
            Expr::Empty,
        ])]);
        for n in 0..=6 {
            let s: Vec<u8> = std::iter::repeat(b'a')
                .take(n)
                .chain(std::iter::repeat(b'b').take(n))
                .collect();
            assert_eq!(parse_full(&g, &s), Some(()));
        }
        assert_eq!(parse_full(&g, b"aabbb"), None);
        assert_eq!(parse_full(&g, b"abab"), None);
    }

    #[test]
    fn left_recursion_fails_cleanly() {
        // S -> S 'a' | 'a' — direct left recursion; must not hang.
        let g = Grammar::new(vec![Expr::Alt(vec![
            Expr::Seq(vec![Expr::Call(0), lit("a")]),
            lit("a"),
        ])]);
        assert_eq!(parse(&g, b"aaa"), Some(1)); // falls back to 'a' arm
    }

    /// Naive backtracking evaluator — identical semantics, no memo.
    /// The oracle for the packrat layer: memoization must not change
    /// the answer.
    fn naive(g: &Grammar, input: &[u8]) -> Option<usize> {
        let mut ctx = Ctx {
            g,
            input,
            memo: std::collections::BTreeMap::new(),
            active: std::collections::BTreeSet::new(),
            use_memo: false,
        };
        eval_call(&mut ctx, 0, 0)
    }

    /// Random expression of bounded depth over alphabet {a,b} with at
    /// most `nrules` rules; returns at depth 0 → leaves only.
    fn rand_expr(rng: &mut SplitMix64, depth: usize, nrules: u32) -> Expr {
        let kind = rng.below(if depth == 0 { 4 } else { 9 });
        match kind {
            0 => Expr::Empty,
            1 => Expr::Lit(vec![b'a' + rng.below(2) as u8]),
            2 => Expr::Class(vec![b'a', b'b']),
            3 => Expr::AnyByte,
            4 => Expr::Seq(vec![
                rand_expr(rng, depth - 1, nrules),
                rand_expr(rng, depth - 1, nrules),
            ]),
            5 => Expr::Alt(vec![
                rand_expr(rng, depth - 1, nrules),
                rand_expr(rng, depth - 1, nrules),
            ]),
            6 => Expr::Star(Box::new(rand_expr(rng, depth - 1, nrules))),
            7 => Expr::Opt(Box::new(rand_expr(rng, depth - 1, nrules))),
            _ => Expr::Call(rng.below(nrules)),
        }
    }

    #[test]
    fn memo_equivalence_oracle() {
        let mut rng = SplitMix64::new(0x9e6d_f17b_c3a5_8842);
        for _ in 0..200 {
            let nrules = 1 + rng.below(3);
            let rules: Vec<Expr> = (0..nrules)
                .map(|_| rand_expr(&mut rng, 2, nrules))
                .collect();
            let g = Grammar::new(rules);
            for _ in 0..20 {
                let len = rng.below(9) as usize;
                let input: Vec<u8> = (0..len).map(|_| b'a' + rng.below(3) as u8).collect();
                assert_eq!(parse(&g, &input), naive(&g, &input));
            }
        }
    }

    #[test]
    fn expression_grammar() {
        // expr = term ('+' term)* ; term = number ('*' number)* ;
        // number = [0-9]+. Rule indices: 0=expr 1=term 2=number.
        let g = Grammar::new(vec![
            Expr::Seq(vec![
                Expr::Call(1),
                Expr::Star(Box::new(Expr::Seq(vec![lit("+"), Expr::Call(1)]))),
            ]),
            Expr::Seq(vec![
                Expr::Call(2),
                Expr::Star(Box::new(Expr::Seq(vec![lit("*"), Expr::Call(2)]))),
            ]),
            Expr::Plus(Box::new(Expr::Class((b'0'..=b'9').collect()))),
        ]);
        assert_eq!(parse_full(&g, b"12+34*56"), Some(()));
        assert_eq!(parse_full(&g, b"12+"), None);
        assert_eq!(parse_full(&g, b"1a"), None);
    }
}

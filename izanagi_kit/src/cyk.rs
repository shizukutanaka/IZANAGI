//! CYK recognition for context-free grammars in Chomsky normal
//! form — the classic `O(n³·|G|)` dynamic program that decides
//! membership of a byte string without any backtracking or
//! recursion. Rules are binary productions `A → B C` or terminal
//! productions `A → b`; nonterminals are small integer ids, so the
//! whole parse is a bitset triangle — a pure function of
//! `(grammar, input)` with no allocation order effects.
//!
//! ```
//! use izanagi_kit::cyk::{Cyk, Rule};
//!
//! // Balanced parens: S → ( S ) | ( ) | S S — in CNF:
//! //   S → L R' | L R | S S ;  R' → S R ;  L → '(' ;  R → ')'
//! let g = Cyk::new(
//!     4, // S=0, R'=1, L=2, R=3
//!     &[
//!         Rule::Bin(0, 2, 1),
//!         Rule::Bin(0, 2, 3),
//!         Rule::Bin(0, 0, 0),
//!         Rule::Bin(1, 0, 3),
//!         Rule::Term(2, b'('),
//!         Rule::Term(3, b')'),
//!     ],
//! );
//! assert!(g.accepts(b"(()())"));
//! assert!(!g.accepts(b"(()"));
//! assert!(!g.accepts(b""));
//! ```
//!
//! References: Cocke–Younger–Kasami; Hopcroft & Ullman,
//! "Introduction to Automata Theory" (1979).

/// One production in Chomsky normal form: either a binary
/// `lhs → a b` with nonterminal ids, or a terminal `lhs → byte`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rule {
    /// `lhs → a b` — concatenation of nonterminals `a`, `b`.
    Bin(u32, u32, u32),
    /// `lhs → byte` — a single terminal.
    Term(u32, u8),
}

/// CYK recognizer over a CNF grammar with `nt` nonterminals
/// (`0` is the start symbol by convention).
pub struct Cyk {
    nt: usize,
    /// `bin[a][b]` = bitset of left-hand sides producing `a b`.
    bin: Vec<Vec<u64>>,
    /// `term[b]` = bitset of left-hand sides producing byte `b`.
    term: [u64; 256],
}

impl Cyk {
    /// Build the recognizer; rules may repeat and order never
    /// matters — the grammar is a set.
    pub fn new(nt: usize, rules: &[Rule]) -> Self {
        let mut bin = vec![vec![0u64; nt]; nt];
        let mut term = [0u64; 256];
        for &r in rules {
            match r {
                Rule::Bin(lhs, a, b) => {
                    if (lhs as usize) < nt && (a as usize) < nt && (b as usize) < nt {
                        bin[a as usize][b as usize] |= 1u64 << lhs;
                    }
                }
                Rule::Term(lhs, t) => {
                    if (lhs as usize) < nt {
                        term[t as usize] |= 1u64 << lhs;
                    }
                }
            }
        }
        Self { nt, bin, term }
    }

    /// Bitset of nonterminals deriving `input[i..j]` — the full
    /// parse triangle's cell, exposed for ambiguity inspection.
    pub fn cell(&self, input: &[u8], i: usize, j: usize) -> u64 {
        // Recompute the prefix triangle up to (i,j) — callers
        // inspecting one cell pay the same O(n³) as `accepts`.
        let n = input.len();
        if i >= j || j > n || self.nt == 0 {
            return 0;
        }
        let table = self.parse(input);
        table[j * (n + 1) + i]
    }

    /// The parse triangle: `t[j][i]` = nonterminals deriving
    /// `input[i..j]`, stored row-major over a `(n+1)²` grid.
    fn parse(&self, input: &[u8]) -> Vec<u64> {
        let n = input.len();
        let w = n + 1;
        let mut t = vec![0u64; w * w];
        for i in 0..n {
            t[(i + 1) * w + i] = self.term[input[i] as usize];
        }
        for len in 2..=n {
            for i in 0..=(n - len) {
                let j = i + len;
                let mut cell = 0u64;
                for k in (i + 1)..j {
                    let left = t[k * w + i];
                    if left == 0 {
                        continue;
                    }
                    let right = t[j * w + k];
                    if right == 0 {
                        continue;
                    }
                    // Union over set bits of left/right.
                    let mut l = left;
                    while l != 0 {
                        let a = l.trailing_zeros() as usize;
                        l &= l - 1;
                        let mut r = right;
                        while r != 0 {
                            let b = r.trailing_zeros() as usize;
                            r &= r - 1;
                            cell |= self.bin[a][b];
                        }
                    }
                }
                t[j * w + i] = cell;
            }
        }
        t
    }

    /// Whether the start symbol `0` derives `input` exactly.
    pub fn accepts(&self, input: &[u8]) -> bool {
        let n = input.len();
        if n == 0 || self.nt == 0 {
            return false;
        }
        self.parse(input)[n * (n + 1)] & 1 != 0
    }

    /// Bitset of every nonterminal deriving all of `input` —
    /// the start cell of the triangle.
    pub fn derive(&self, input: &[u8]) -> u64 {
        let n = input.len();
        if n == 0 || self.nt == 0 {
            return 0;
        }
        self.parse(input)[n * (n + 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::{BTreeMap, BTreeSet};

    /// Grammar data for the oracle: terminal map + binary rules.
    struct G {
        term: BTreeMap<u8, BTreeSet<u32>>,
        bin: Vec<(u32, u32, u32)>,
    }

    /// Naive recognizer: does `nt` derive `input[i..j]`? Direct
    /// recursive expansion memoized on (nt, i, j) — an independent
    /// path from the bitset triangle.
    fn derives(
        g: &G,
        nt: u32,
        s: &[u8],
        i: usize,
        j: usize,
        memo: &mut BTreeMap<(u32, usize, usize), bool>,
    ) -> bool {
        if let Some(&v) = memo.get(&(nt, i, j)) {
            return v;
        }
        let mut ok = false;
        if j == i + 1 {
            ok = g.term.get(&s[i]).is_some_and(|set| set.contains(&nt));
        } else {
            for &(_, a, b) in g.bin.iter().filter(|&&(l, _, _)| l == nt) {
                for k in (i + 1)..j {
                    if derives(g, a, s, i, k, memo) && derives(g, b, s, k, j, memo) {
                        ok = true;
                    }
                }
            }
        }
        memo.insert((nt, i, j), ok);
        ok
    }

    fn parens() -> (Cyk, G) {
        let rules = [
            Rule::Bin(0, 2, 1),
            Rule::Bin(0, 2, 3),
            Rule::Bin(0, 0, 0),
            Rule::Bin(1, 0, 3),
            Rule::Term(2, b'('),
            Rule::Term(3, b')'),
        ];
        let g = G {
            term: BTreeMap::from([(b'(', BTreeSet::from([2])), (b')', BTreeSet::from([3]))]),
            bin: vec![(0, 2, 1), (0, 2, 3), (0, 0, 0), (1, 0, 3)],
        };
        (Cyk::new(4, &rules), g)
    }

    #[test]
    fn known_languages() {
        let (cyk, _) = parens();
        for s in [&b"()"[..], b"(())", b"()()", b"(()())", b"((()()))()"] {
            assert!(cyk.accepts(s), "{:?}", s);
        }
        for s in [&b"("[..], b")", b"(", b"())(", b"(()", b"abc"] {
            assert!(!cyk.accepts(s), "{:?}", s);
        }
        // derive() exposes non-start nonterminals: "(" is only L.
        assert_eq!(cyk.derive(b"("), 1 << 2);
    }

    #[test]
    fn cell_matches_triangle() {
        let (cyk, _) = parens();
        let s = b"(())";
        // "((" derives nothing; whole span derives S plus helpers.
        assert_eq!(cyk.cell(s, 0, 2), 0);
        assert!(cyk.cell(s, 0, 4) & 1 != 0);
        // sub-span "(())" at [1..3] = "()" derives S.
        assert!(cyk.cell(s, 1, 3) & 1 != 0);
    }

    #[test]
    fn oracle_random_cfgs() {
        let mut rng = SplitMix64::new(0xC0DE_C0DE);
        for _ in 0..150 {
            let nt = 1 + (rng.next_u64() % 4) as u32;
            let mut rules = Vec::new();
            let mut g = G {
                term: BTreeMap::new(),
                bin: Vec::new(),
            };
            for _ in 0..(1 + rng.next_u64() % 8) {
                let lhs = (rng.next_u64() % nt as u64) as u32;
                if rng.next_u64() % 2 == 0 {
                    let t = b'a' + (rng.next_u64() % 3) as u8;
                    rules.push(Rule::Term(lhs, t));
                    g.term.entry(t).or_default().insert(lhs);
                } else {
                    let a = (rng.next_u64() % nt as u64) as u32;
                    let b = (rng.next_u64() % nt as u64) as u32;
                    rules.push(Rule::Bin(lhs, a, b));
                    g.bin.push((lhs, a, b));
                }
            }
            let cyk = Cyk::new(nt as usize, &rules);
            let n = 1 + (rng.next_u64() % 6) as usize;
            for _ in 0..40 {
                let s: Vec<u8> = (0..n).map(|_| b'a' + (rng.next_u64() % 3) as u8).collect();
                let mut memo = BTreeMap::new();
                assert_eq!(
                    cyk.accepts(&s),
                    derives(&g, 0, &s, 0, s.len(), &mut memo),
                    "grammar {:?} input {:?}",
                    rules,
                    s
                );
            }
        }
    }

    #[test]
    fn edge_cases() {
        // Empty grammar / empty input / out-of-range nt ids.
        let empty = Cyk::new(0, &[]);
        assert!(!empty.accepts(b"a"));
        assert_eq!(empty.derive(b"a"), 0);
        let g = Cyk::new(2, &[Rule::Term(0, b'x'), Rule::Bin(0, 9, 9)]);
        assert!(g.accepts(b"x"));
        assert!(!g.accepts(b"xx"));
        assert_eq!(g.cell(b"x", 0, 0), 0);
        assert_eq!(g.cell(b"", 0, 0), 0);
    }
}

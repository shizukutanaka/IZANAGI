//! Earley chart parser for arbitrary context-free grammars —
//! `cyk` recognizes only Chomsky normal form; this accepts any
//! CFG directly, including ε-rules and mixed-length productions,
//! with the classic predict / scan / complete fixpoint per chart
//! set. Chart sets are `BTreeSet`s and the work queue is a
//! `VecDeque`, so the item enumeration order — and hence
//! `parse` — is a pure function of `(grammar, input)`.
//!
//! ```
//! use izanagi_kit::earley::{Earley, Sym::*};
//!
//! // S -> "a" S "b" | "a" "b"   (recognizes a^n b^n)
//! let g = Earley::new(1, 0, &[(0, vec![T(b'a'), N(0), T(b'b')]), (0, vec![T(b'a'), T(b'b')])]);
//! assert!(g.parse(b"aabb"));
//! assert!(g.parse(b"ab"));
//! assert!(!g.parse(b"aab"));
//! ```
//!
//! References: Earley (1970), CACM 13(2); Aycock & Horspool (2002)
//! "Practical Earley parsing" (the eager-completion detail this
//! uses: an item expecting `N` also completes against completed
//! `N` items already in the same set).

/// Grammar symbol: a byte terminal or a nonterminal id.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Sym {
    /// Terminal byte matched against the input.
    T(u8),
    /// Nonterminal id `0..nt`.
    N(u32),
}

/// One chart item: `rules[rule]` with the dot before `dot`, started
/// at input position `origin`. Ordered triples keep queue order
/// deterministic.
type Item = (u32, u32, u32);

/// An arbitrary-CFG recognizer. `rules` is flat `(lhs, rhs)` pairs;
/// a synthetic `S' -> S` rule is prepended internally so the start
/// symbol may appear on a right-hand side without ambiguity.
pub struct Earley {
    /// All rules; `rules[0]` is the synthetic `(start_nt, [N(start)])`.
    rules: Vec<(u32, Vec<Sym>)>,
    /// Alternatives per nonterminal: `alts[nt]` = rule indices.
    alts: Vec<Vec<u32>>,
    /// The synthetic start's rule index (always 0).
    start_rule: u32,
}

impl Earley {
    /// Build a grammar: `nt` nonterminals numbered `0..nt`,
    /// `start` the axiom, `rules` the `(lhs, rhs)` productions.
    /// Rules mentioning ids `>= nt` are dropped — a malformed
    /// grammar degrades to a smaller grammar, not a crash.
    pub fn new(nt: u32, start: u32, rules: &[(u32, Vec<Sym>)]) -> Self {
        let n = nt + 1; // slot for the synthetic S'
        let mut flat: Vec<(u32, Vec<Sym>)> = Vec::with_capacity(rules.len() + 1);
        let mut alts: Vec<Vec<u32>> = vec![Vec::new(); n as usize];
        flat.push((n - 1, vec![Sym::N(start)]));
        alts[n as usize - 1].push(0);
        for (lhs, rhs) in rules {
            if *lhs >= nt || rhs.iter().any(|s| matches!(s, Sym::N(x) if *x >= nt)) {
                continue;
            }
            let idx = flat.len() as u32;
            flat.push((*lhs, rhs.clone()));
            alts[*lhs as usize].push(idx);
        }
        Self {
            rules: flat,
            alts,
            start_rule: 0,
        }
    }

    /// Whether `input` derives from the axiom.
    pub fn parse(&self, input: &[u8]) -> bool {
        let n = input.len();
        let mut chart: Vec<std::collections::BTreeSet<Item>> =
            vec![std::collections::BTreeSet::new(); n + 1];
        // One queue per position: a scan step inserts into
        // `chart[i+1]`, and those items must be processed as
        // chart[i+1] items — not now, still inside step i.
        let mut queues: Vec<std::collections::VecDeque<Item>> =
            (0..=n).map(|_| std::collections::VecDeque::new()).collect();
        chart[0].insert((self.start_rule, 0, 0));
        queues[0].push_back((self.start_rule, 0, 0));

        for i in 0..=n {
            while let Some((ri, dot, org)) = queues[i].pop_front() {
                let rhs = &self.rules[ri as usize].1;
                if (dot as usize) == rhs.len() {
                    // Completed `lhs` spanning (org, i): advance every
                    // item in chart[org] that expected it.
                    let lhs = self.rules[ri as usize].0;
                    let pred: Vec<Item> = chart[org as usize]
                        .iter()
                        .filter(|&&(rj, dj, _)| {
                            self.rules[rj as usize].1.get(dj as usize) == Some(&Sym::N(lhs))
                        })
                        .map(|&(rj, dj, j)| (rj, dj + 1, j))
                        .collect();
                    for it in pred {
                        Self::add(&mut chart, &mut queues, i, it);
                    }
                } else {
                    match rhs[dot as usize] {
                        Sym::T(b) => {
                            if i < n && input[i] == b {
                                Self::add(&mut chart, &mut queues, i + 1, (ri, dot + 1, org));
                            }
                        }
                        Sym::N(nt) => {
                            // Predict each alternative of `nt`.
                            for &alt in &self.alts[nt as usize] {
                                Self::add(&mut chart, &mut queues, i, (alt, 0, i as u32));
                            }
                            // Eager completion: `nt` may already be
                            // finished in this same set (ε or earlier
                            // derivation).
                            let done: Vec<Item> = chart[i]
                                .iter()
                                .filter(|&&(rj, dj, _)| {
                                    self.rules[rj as usize].0 == nt
                                        && dj as usize == self.rules[rj as usize].1.len()
                                })
                                .copied()
                                .collect();
                            for _ in done {
                                Self::add(&mut chart, &mut queues, i, (ri, dot + 1, org));
                            }
                        }
                    }
                }
            }
        }
        chart[n].contains(&(self.start_rule, 1, 0))
    }

    /// Insert `it` into `chart[i]` once and enqueue it on
    /// `queues[i]` so it is processed under the right position.
    fn add(
        chart: &mut [std::collections::BTreeSet<Item>],
        queues: &mut [std::collections::VecDeque<Item>],
        i: usize,
        it: Item,
    ) {
        if chart[i].insert(it) {
            queues[i].push_back(it);
        }
    }

    /// Number of grammar rules (including the synthetic start).
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::{BTreeMap, BTreeSet};
    use Sym::{N, T};

    /// Grammar bundle for the oracle recursion.
    struct Oracle<'a> {
        rules: &'a [(u32, Vec<Sym>)],
        alts: &'a BTreeMap<u32, Vec<u32>>,
        s: &'a [u8],
    }

    /// Independent oracle: can nonterminal `nt` derive `s[i..j)`?
    /// Memoized, cycle-safe (an in-progress query yields false —
    /// a cyclic re-entry cannot produce a shorter derivation than
    /// the one already in flight).
    fn derives(
        g: &Oracle,
        nt: u32,
        i: usize,
        j: usize,
        memo: &mut BTreeMap<(u32, usize, usize), bool>,
        busy: &mut BTreeSet<(u32, usize, usize)>,
    ) -> bool {
        if let Some(&v) = memo.get(&(nt, i, j)) {
            return v;
        }
        if !busy.insert((nt, i, j)) {
            return false;
        }
        let mut ok = false;
        'outer: for &ri in g.alts.get(&nt).cloned().unwrap_or_default().iter() {
            // All splits of [i,j) among rhs symbols, recursion on the fly.
            let rhs = &g.rules[ri as usize].1;
            let mut states = vec![i];
            for sym in rhs {
                let mut next = Vec::new();
                for &p in &states {
                    match sym {
                        T(b) => {
                            if p < j && g.s[p] == *b {
                                next.push(p + 1);
                            }
                        }
                        N(m) => {
                            for split in p..=j {
                                if derives(g, *m, p, split, memo, busy) {
                                    next.push(split);
                                }
                            }
                        }
                    }
                }
                next.sort_unstable();
                next.dedup();
                states = next;
                if states.is_empty() {
                    continue 'outer;
                }
            }
            if states.contains(&j) {
                ok = true;
                break;
            }
        }
        busy.remove(&(nt, i, j));
        memo.insert((nt, i, j), ok);
        ok
    }

    fn oracle_accepts(rules: &[(u32, Vec<Sym>)], start: u32, s: &[u8]) -> bool {
        let mut alts: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (i, (lhs, _)) in rules.iter().enumerate() {
            alts.entry(*lhs).or_default().push(i as u32);
        }
        let g = Oracle {
            rules,
            alts: &alts,
            s,
        };
        derives(
            &g,
            start,
            0,
            s.len(),
            &mut BTreeMap::new(),
            &mut BTreeSet::new(),
        )
    }

    #[test]
    fn known_grammars() {
        // a^n b^n
        let an_bn = Earley::new(
            1,
            0,
            &[
                (0, vec![T(b'a'), N(0), T(b'b')]),
                (0, vec![T(b'a'), T(b'b')]),
            ],
        );
        for k in 1..6 {
            let mut w = vec![b'a'; k];
            w.extend(std::iter::repeat(b'b').take(k));
            assert!(an_bn.parse(&w), "a^{k}b^{k}");
        }
        assert!(!an_bn.parse(b"aab"));
        assert!(!an_bn.parse(b""));
        assert!(!an_bn.parse(b"ba"));

        assert_eq!(an_bn.rule_count(), 3); // two user rules + synthetic S'

        // Balanced parens with an epsilon alternative.
        let parens = Earley::new(
            1,
            0,
            &[(0, vec![T(b'('), N(0), T(b')'), N(0)]), (0, vec![])],
        );
        assert!(parens.parse(b"()"));
        assert!(parens.parse(b"(())()"));
        assert!(parens.parse(b""));
        assert!(!parens.parse(b"(()"));
        assert!(!parens.parse(b")("));

        // Ambiguous arithmetic-ish grammar; left-recursive.
        let expr = Earley::new(
            2,
            0,
            &[
                (0, vec![N(0), T(b'+'), N(1)]),
                (0, vec![N(1)]),
                (1, vec![T(b'x')]),
            ],
        );
        assert!(expr.parse(b"x+x+x"));
        assert!(!expr.parse(b"+x"));
    }

    #[test]
    fn random_oracle() {
        let mut r = SplitMix64::new(0xEA41_EB17);
        for _ in 0..120 {
            let nt = 1 + r.below(3);
            let mut rules: Vec<(u32, Vec<Sym>)> = Vec::new();
            let n_rules = 1 + r.below(5);
            for lhs in 0..n_rules {
                let len = r.below(3);
                let rhs: Vec<Sym> = (0..len)
                    .map(|_| {
                        if r.next_u64() & 1 == 0 {
                            N(r.below(nt))
                        } else {
                            T(b'a' + (r.below(2)) as u8)
                        }
                    })
                    .collect();
                rules.push((lhs, rhs));
            }
            let g = Earley::new(nt, 0, &rules);
            // Clean rules for the oracle (Earley::new drops invalid;
            // our generated ids are all in range by construction).
            for _ in 0..40 {
                let len = r.below(9) as usize;
                let s: Vec<u8> = (0..len).map(|_| b'a' + (r.below(2)) as u8).collect();
                assert_eq!(
                    g.parse(&s),
                    oracle_accepts(&rules, 0, &s),
                    "rules={rules:?} s={s:?}"
                );
            }
        }
    }
}

//! Finite automata: NFA (Thompson-style, ε-edges) → DFA via
//! subset construction, with language ops — union, concat,
//! star, complement, intersection, minimization.
//!
//! The deterministic half keeps transitions as
//! `BTreeMap<u8, usize>` per state, so every construction is
//! a pure function of the input automaton — same NFA in,
//! same state numbering out.
//!
//! Complements and intersections *totalize* first (add a sink
//! for missing letters over the union alphabet) — a partial
//! DFA's implicit "die" is not the same as explicit reject,
//! and pretending otherwise flips the language under
//! complement.
//!
//! ```
//! use izanagi_kit::automaton::Nfa;
//! // (a|b)*abb — the classic textbook NFA
//! let star = Nfa::literal(b"a").union(&Nfa::literal(b"b")).star();
//! let pat = star.concat(&Nfa::literal(b"abb"));
//! let d = pat.determinize();
//! assert!(d.accepts(b"aabb"));
//! assert!(d.accepts(b"bbabb"));
//! assert!(!d.accepts(b"ab"));
//! assert_eq!(d.minimize().n, 4); // minimal DFA has 4 states
//! ```

use std::collections::{BTreeMap, BTreeSet};

/// A nondeterministic automaton — explicit ε-edges and byte
/// transitions. States are `0..n`; `accepts` may hold any
/// subset.
#[derive(Clone, Debug, Default)]
pub struct Nfa {
    /// Number of states.
    pub n: usize,
    /// Start state.
    pub start: usize,
    /// Accepting states.
    pub accepts: BTreeSet<usize>,
    /// Byte transitions `(from, byte, to)`.
    pub edges: Vec<(usize, u8, usize)>,
    /// ε transitions `(from, to)`.
    pub eps: Vec<(usize, usize)>,
}

/// A deterministic automaton — `delta[s]` maps byte → next
/// state; missing entries mean "die" (implicit reject).
#[derive(Clone, Debug, Default)]
pub struct Dfa {
    /// Number of states.
    pub n: usize,
    /// Start state.
    pub start: usize,
    /// Accepting states.
    pub accepts: BTreeSet<usize>,
    /// Per-state transitions.
    pub delta: Vec<BTreeMap<u8, usize>>,
}

impl Nfa {
    /// NFA accepting exactly the empty word.
    pub fn empty() -> Self {
        Nfa {
            n: 1,
            start: 0,
            accepts: [0].into_iter().collect(),
            edges: Vec::new(),
            eps: Vec::new(),
        }
    }

    /// NFA accepting exactly `bytes` as a literal.
    pub fn literal(bytes: &[u8]) -> Self {
        let n = bytes.len() + 1;
        Nfa {
            n,
            start: 0,
            accepts: [n - 1].into_iter().collect(),
            edges: (0..bytes.len()).map(|i| (i, bytes[i], i + 1)).collect(),
            eps: Vec::new(),
        }
    }

    /// Shift every state id by `off` — shared renumbering.
    fn shifted(&self, off: usize) -> Nfa {
        Nfa {
            n: self.n,
            start: self.start + off,
            accepts: self.accepts.iter().map(|&s| s + off).collect(),
            edges: self
                .edges
                .iter()
                .map(|&(a, c, b)| (a + off, c, b + off))
                .collect(),
            eps: self.eps.iter().map(|&(a, b)| (a + off, b + off)).collect(),
        }
    }

    /// `self | other` — Thompson union (new start ε-branching
    /// to both, old accepts kept).
    pub fn union(&self, other: &Nfa) -> Nfa {
        let a = self.shifted(1);
        let b = other.shifted(1 + self.n);
        let mut nfa = Nfa {
            n: self.n + other.n + 1,
            start: 0,
            accepts: a.accepts.iter().chain(b.accepts.iter()).copied().collect(),
            edges: a.edges.iter().chain(b.edges.iter()).copied().collect(),
            eps: a.eps.iter().chain(b.eps.iter()).copied().collect(),
        };
        nfa.eps.push((0, a.start));
        nfa.eps.push((0, b.start));
        nfa
    }

    /// `self ++ other` — concatenation: each of self's accepts
    /// ε-links to other's start; other's own ε-edges are kept
    /// (dropping them silently loses e.g. a star's loop).
    pub fn concat(&self, other: &Nfa) -> Nfa {
        let a = self.clone();
        let b = other.shifted(self.n);
        let mut eps: Vec<(usize, usize)> = a.eps.iter().chain(b.eps.iter()).copied().collect();
        for &acc in &a.accepts {
            eps.push((acc, b.start));
        }
        Nfa {
            n: a.n + b.n,
            start: a.start,
            accepts: b.accepts,
            edges: a.edges.iter().chain(b.edges.iter()).copied().collect(),
            eps,
        }
    }

    /// `self*` — new accepting start ε-linking to self's start;
    /// each old accept ε-links back to the *new* start (its
    /// ε-edge closes the loop *and* reaches accept — linking
    /// to `a.start` instead leaves old accepts unreachable
    /// from the only accepting state).
    pub fn star(&self) -> Nfa {
        let a = self.shifted(1);
        let mut eps = a.eps.clone();
        eps.push((0, a.start));
        for &acc in &a.accepts {
            eps.push((acc, 0));
        }
        Nfa {
            n: a.n + 1,
            start: 0,
            accepts: [0].into_iter().collect(),
            edges: a.edges,
            eps,
        }
    }

    /// `self+` — one or more.
    pub fn plus(&self) -> Nfa {
        self.concat(&self.star())
    }

    /// `self?` — optional.
    pub fn optional(&self) -> Nfa {
        self.union(&Nfa::empty())
    }

    /// ε-closure of a state set — fixpoint reachability.
    fn eps_closure(&self, set: &BTreeSet<usize>) -> BTreeSet<usize> {
        let mut out = set.clone();
        let mut stack: Vec<usize> = set.iter().copied().collect();
        while let Some(s) = stack.pop() {
            for &(a, b) in &self.eps {
                if a == s && out.insert(b) {
                    stack.push(b);
                }
            }
        }
        out
    }

    /// The set of states reachable from `set` on byte `c`
    /// (without trailing ε-closure).
    fn step(&self, set: &BTreeSet<usize>, c: u8) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        for &(a, ch, b) in &self.edges {
            if ch == c && set.contains(&a) {
                out.insert(b);
            }
        }
        out
    }

    /// Subset construction — a [`Dfa`] whose states are the
    /// reachable ε-closed subsets, numbered in BFS order over
    /// sorted state-sets (deterministic).
    pub fn determinize(&self) -> Dfa {
        let alphabet: BTreeSet<u8> = self.edges.iter().map(|&(_, c, _)| c).collect();
        let start = self.eps_closure(&[self.start].into_iter().collect());
        let mut ids: BTreeMap<BTreeSet<usize>, usize> = BTreeMap::new();
        let mut queue: Vec<BTreeSet<usize>> = vec![start];
        ids.insert(queue[0].clone(), 0);
        let mut delta: Vec<BTreeMap<u8, usize>> = Vec::new();
        let mut accepts: BTreeSet<usize> = BTreeSet::new();
        let mut i = 0;
        while i < queue.len() {
            let set = queue[i].clone();
            if set.iter().any(|s| self.accepts.contains(s)) {
                accepts.insert(i);
            }
            let mut row = BTreeMap::new();
            for &c in &alphabet {
                let next = self.eps_closure(&self.step(&set, c));
                if next.is_empty() {
                    continue;
                }
                let id = match ids.get(&next) {
                    Some(&id) => id,
                    None => {
                        let id = ids.len();
                        ids.insert(next.clone(), id);
                        queue.push(next);
                        id
                    }
                };
                row.insert(c, id);
            }
            delta.push(row);
            i += 1;
        }
        Dfa {
            n: queue.len(),
            start: 0,
            accepts,
            delta,
        }
    }

    /// Simulate directly (subset walk) — `input` accepted?
    pub fn accepts(&self, input: &[u8]) -> bool {
        let mut cur = self.eps_closure(&[self.start].into_iter().collect());
        for &c in input {
            cur = self.eps_closure(&self.step(&cur, c));
            if cur.is_empty() {
                return false;
            }
        }
        cur.iter().any(|s| self.accepts.contains(s))
    }
}

/// "Dead" state sentinel for [`Dfa::intersect`]'s missing
/// transitions — `!0` (not `usize::MAX`, a banned literal).
const DIE: usize = !0;

impl Dfa {
    /// `input` accepted?
    pub fn accepts(&self, input: &[u8]) -> bool {
        let mut s = self.start;
        for &c in input {
            match self.delta[s].get(&c) {
                Some(&t) => s = t,
                None => return false,
            }
        }
        self.accepts.contains(&s)
    }

    /// Alphabet actually used in `delta`.
    fn alphabet(&self) -> BTreeSet<u8> {
        self.delta
            .iter()
            .flat_map(|row| row.keys().copied())
            .collect()
    }

    /// Totalize over `alphabet` — add a sink for every missing
    /// `(state, letter)`. Idempotent.
    fn complete(&mut self, alphabet: &BTreeSet<u8>) {
        if self.n == 0 {
            self.delta.push(BTreeMap::new());
            self.n = 1;
            self.start = 0;
        }
        let sink = self.n;
        let mut need_sink = false;
        for row in self.delta.iter_mut() {
            for &c in alphabet {
                // `insert` would overwrite an existing
                // transition with `sink` — use entry
                row.entry(c).or_insert_with(|| {
                    need_sink = true;
                    sink
                });
            }
        }
        if need_sink {
            self.n += 1;
            self.delta
                .push(alphabet.iter().map(|&c| (c, sink)).collect());
        }
    }

    /// `~L` over `alphabet` — the complement is relative to a
    /// letter set, so it must be explicit: a word containing
    /// a letter outside `alphabet` isn't a word at all, and
    /// the implicit "die" must first be made an explicit sink.
    pub fn complement(&self, alphabet: &[u8]) -> Dfa {
        let mut d = self.clone();
        let alpha: BTreeSet<u8> = alphabet.iter().copied().collect();
        d.complete(&alpha);
        d.accepts = (0..d.n).filter(|s| !d.accepts.contains(s)).collect();
        d
    }

    /// `L1 ∩ L2` — product construction over the union
    /// alphabet; a pair `(a, b)` accepts iff both do.
    pub fn intersect(&self, other: &Dfa) -> Dfa {
        let alphabet: BTreeSet<u8> = self.alphabet().union(&other.alphabet()).copied().collect();
        let mut a = self.clone();
        let mut b = other.clone();
        a.complete(&alphabet);
        b.complete(&alphabet);
        // state (i,j) → fresh id
        let mut ids: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut queue = vec![(a.start, b.start)];
        ids.insert(queue[0], 0);
        let mut delta: Vec<BTreeMap<u8, usize>> = Vec::new();
        let mut accepts: BTreeSet<usize> = BTreeSet::new();
        let mut i = 0;
        while i < queue.len() {
            let (sa, sb) = queue[i];
            if a.accepts.contains(&sa) && b.accepts.contains(&sb) {
                accepts.insert(i);
            }
            let mut row = BTreeMap::new();
            for &c in &alphabet {
                let ta = a.delta[sa].get(&c).copied().unwrap_or(DIE);
                let tb = b.delta[sb].get(&c).copied().unwrap_or(DIE);
                if ta == DIE || tb == DIE {
                    continue; // complete() guarantees both exist
                }
                let key = (ta, tb);
                let id = match ids.get(&key) {
                    Some(&id) => id,
                    None => {
                        let id = ids.len();
                        ids.insert(key, id);
                        queue.push(key);
                        id
                    }
                };
                row.insert(c, id);
            }
            delta.push(row);
            i += 1;
        }
        Dfa {
            n: queue.len(),
            start: 0,
            accepts,
            delta,
        }
    }

    /// Minimize via [`crate::dfamin::minimize`] — renumber the
    /// used alphabet to `0..sigma`, totalize, partition-
    /// refine, then re-expand to byte labels.
    pub fn minimize(&self) -> Dfa {
        let alphabet: Vec<u8> = self.alphabet().into_iter().collect();
        if self.n == 0 {
            return self.clone();
        }
        let mut d = self.clone();
        let alpha_set: BTreeSet<u8> = alphabet.iter().copied().collect();
        d.complete(&alpha_set);
        let sigma = alphabet.len();
        let delta: Vec<Vec<usize>> = (0..d.n)
            .map(|s| (0..sigma).map(|c| d.delta[s][&alphabet[c]]).collect())
            .collect();
        let accept: Vec<bool> = (0..d.n).map(|s| d.accepts.contains(&s)).collect();
        let m = crate::dfamin::minimize(d.n, sigma, &delta, &accept, d.start);
        // rebuild with byte labels: MinDfa.delta is indexed by
        // compact state then symbol id; symbols[i] = alphabet[i]
        let new_delta: Vec<BTreeMap<u8, usize>> = m
            .delta
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(c, &t)| (alphabet[c], t))
                    .collect()
            })
            .collect();
        let new_accepts: BTreeSet<usize> = m
            .accept
            .iter()
            .enumerate()
            .filter(|(_, &a)| a)
            .map(|(i, _)| i)
            .collect();
        Dfa {
            n: m.n,
            start: m.start,
            accepts: new_accepts,
            delta: new_delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (a|b)*abb textbook example.
    fn abb() -> Nfa {
        let ab_star = Nfa::literal(b"a").union(&Nfa::literal(b"b")).star();
        ab_star
            .concat(&Nfa::literal(b"a"))
            .concat(&Nfa::literal(b"b"))
            .concat(&Nfa::literal(b"b"))
    }

    #[test]
    fn basics() {
        let d = abb().determinize();
        assert!(d.accepts(b"abb"));
        assert!(d.accepts(b"aabb"));
        assert!(d.accepts(b"bababb"));
        assert!(!d.accepts(b"ab"));
        assert!(!d.accepts(b"abba"));
        assert!(!d.accepts(b""));
        // the minimal DFA for (a|b)*abb has 4 states
        let m = d.minimize();
        assert_eq!(m.n, 4);
        assert!(m.accepts(b"aabb"));
        assert!(!m.accepts(b"ab"));
    }

    /// NFA simulation and its determinized form must agree on
    /// every input — the fundamental correctness oracle.
    #[test]
    fn oracle_sim_equals_determinized() {
        let n = abb();
        let d = n.determinize();
        let mut rng = crate::rng::SplitMix64::new(0xA070);
        for _ in 0..3000 {
            let len = rng.below(9) as usize;
            let word: Vec<u8> = (0..len).map(|_| b'a' + rng.below(2) as u8).collect();
            assert_eq!(n.accepts(&word), d.accepts(&word), "on {word:?}");
        }
        // brute-force acceptance over all words ≤ 5: oracle
        // language = {w : w ends in abb}
        for len in 0..=5u32 {
            for mask in 0..(1u64 << len) {
                let word: Vec<u8> = (0..len).map(|i| b'a' + ((mask >> i) & 1) as u8).collect();
                let expect = len >= 3
                    && word[len as usize - 3] == b'a'
                    && word[len as usize - 2] == b'b'
                    && word[len as usize - 1] == b'b';
                assert_eq!(d.accepts(&word), expect, "{word:?}");
            }
        }
    }

    /// Thompson constructors — union/concat/star/plus/optional
    /// checked against direct simulation.
    #[test]
    fn constructors() {
        let a = Nfa::literal(b"a");
        let b = Nfa::literal(b"b");
        assert!(a.union(&b).accepts(b"a"));
        assert!(a.union(&b).accepts(b"b"));
        assert!(!a.union(&b).accepts(b"ab"));
        assert!(a.concat(&b).accepts(b"ab"));
        let s = a.star();
        assert!(s.accepts(b""));
        assert!(s.accepts(b"aaaa"));
        let p = a.plus();
        assert!(!p.accepts(b""));
        assert!(p.accepts(b"aa"));
        let o = a.optional();
        assert!(o.accepts(b""));
        assert!(o.accepts(b"a"));
        assert!(!o.accepts(b"aa"));
        assert!(Nfa::empty().accepts(b""));
        assert!(!Nfa::empty().accepts(b"a"));
    }

    /// Complement + intersection semantics — the totalize
    /// subtlety: ~(partial DFA) must also accept words that
    /// die mid-word.
    #[test]
    fn complement_and_intersect() {
        // accepts exactly "aa"
        let aa = Nfa::literal(b"aa").determinize();
        let not_aa = aa.complement(b"ab");
        assert!(!not_aa.accepts(b"aa"));
        assert!(not_aa.accepts(b""));
        assert!(not_aa.accepts(b"a"));
        assert!(not_aa.accepts(b"aaa"));
        assert!(not_aa.accepts(b"ab")); // dies in aa — must accept
        assert!(not_aa.accepts(b"aab")); // "aab" isn't "aa"
                                         // "ends with b" ∩ "length 2" = exactly {ab, bb}
        let ends_b = Nfa::literal(b"a")
            .union(&Nfa::literal(b"b"))
            .star()
            .concat(&Nfa::literal(b"b"))
            .determinize();
        let len2 = Nfa::literal(b"a")
            .union(&Nfa::literal(b"b"))
            .concat(&Nfa::literal(b"a").union(&Nfa::literal(b"b")))
            .determinize();
        let both = ends_b.intersect(&len2);
        assert!(both.accepts(b"ab"));
        assert!(both.accepts(b"bb"));
        assert!(!both.accepts(b"aa"));
        assert!(!both.accepts(b"aab"));
        assert!(!both.accepts(b"b"));
    }

    /// Minimization preserves the language.
    #[test]
    fn minimize_preserves() {
        let mut rng = crate::rng::SplitMix64::new(0xDA1E);
        for _ in 0..40 {
            // random regex-ish NFA: (x|y)* over 2..4 letters
            // followed by a mandatory literal
            let lit: Vec<u8> = (0..1 + rng.below(3))
                .map(|_| b'a' + rng.below(3) as u8)
                .collect();
            let letters = 2 + rng.below(2) as u8;
            let mut star = Nfa::literal(b"a");
            for i in 1..letters {
                star = star.union(&Nfa::literal(&[b'a' + i]));
            }
            let n = star.star().concat(&Nfa::literal(&lit));
            let d = n.determinize();
            let m = d.minimize();
            for _ in 0..300 {
                let len = rng.below(8) as usize;
                let word: Vec<u8> = (0..len).map(|_| b'a' + rng.below(4) as u8).collect();
                assert_eq!(d.accepts(&word), m.accepts(&word), "{word:?}");
            }
            assert!(m.n <= d.n);
        }
    }
}

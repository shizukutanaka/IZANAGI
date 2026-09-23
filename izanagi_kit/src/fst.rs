//! Minimal acyclic deterministic automaton (FST-style
//! dictionary index) over byte words.
//!
//! `compile` builds a trie from the word set and then
//! registers equivalent subtrees bottom-up (hash-consing):
//! two states with the same finality and the same
//! `(symbol → child)` map are merged. For an *acyclic*
//! DFA this yields the unique minimal automaton —
//! Myhill–Nerode equivalence classes collapse to exactly
//! the distinguishable right-languages.
//!
//! Everything is a pure function of the word set:
//! insertion order does not change the resulting
//! transition table (input is sorted and deduplicated
//! internally).
//!
//! ```
//! use izanagi_kit::fst::Fst;
//! let dict = Fst::compile(&[b"bar".as_ref(), b"baz", b"foo"]);
//! assert!(dict.contains(b"bar"));
//! assert!(dict.contains(b"baz"));
//! assert!(dict.contains(b"foo"));
//! assert!(!dict.contains(b"ba")); // prefix only
//! assert!(!dict.contains(b"bars")); // superword
//! assert!(!dict.contains(b""));
//! // the word list round-trips, in sorted order
//! assert_eq!(
//!     dict.enumerate(),
//!     vec![b"bar".to_vec(), b"baz".to_vec(), b"foo".to_vec()]
//! );
//! ```

use std::collections::{BTreeMap, BTreeSet};

/// Trie node used during construction.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Trie {
    fin: bool,
    kids: BTreeMap<u8, usize>,
}

/// A compiled minimal acyclic DFA. `states[0]` is the
/// start state; empty word lists produce a single
/// non-final start state that accepts nothing.
#[derive(Clone, Debug)]
pub struct Fst {
    /// `true` if the state is an accept state.
    final_: Vec<bool>,
    /// Transition table: `trans[s]` is sorted by symbol.
    trans: Vec<BTreeMap<u8, u32>>,
}

impl Fst {
    /// Build the minimal automaton accepting exactly
    /// `words`. Order and duplicates in the input do not
    /// matter — the language is a set.
    pub fn compile(words: &[&[u8]]) -> Fst {
        // dedup via BTreeSet → canonical word order
        let set: BTreeSet<Vec<u8>> = words.iter().map(|w| w.to_vec()).collect();
        // 1) full trie
        let mut trie: Vec<Trie> = vec![Trie {
            fin: false,
            kids: BTreeMap::new(),
        }];
        for w in &set {
            let mut cur = 0usize;
            for &b in w {
                let nxt = match trie[cur].kids.get(&b) {
                    Some(&n) => n,
                    None => {
                        let n = trie.len();
                        trie.push(Trie {
                            fin: false,
                            kids: BTreeMap::new(),
                        });
                        trie[cur].kids.insert(b, n);
                        n
                    }
                };
                cur = nxt;
            }
            trie[cur].fin = true;
        }
        // 2) bottom-up register: signature → canonical id.
        // Process nodes in decreasing depth so a node's
        // children are canonical before its signature is
        // computed.
        let mut depth = vec![0u32; trie.len()];
        {
            let mut stack: Vec<usize> = vec![0];
            let mut seen = vec![false; trie.len()];
            seen[0] = true;
            while let Some(v) = stack.pop() {
                for &c in trie[v].kids.values() {
                    if !seen[c] {
                        seen[c] = true;
                        depth[c] = depth[v] + 1;
                        stack.push(c);
                    }
                }
            }
        }
        let mut order: Vec<usize> = (0..trie.len()).collect();
        order.sort_by_key(|&v| u32::MAX - depth[v]);
        type Sig = (bool, Vec<(u8, u32)>);
        let mut registry: BTreeMap<Sig, u32> = BTreeMap::new();
        let mut canon = vec![0u32; trie.len()];
        let mut fin: Vec<bool> = Vec::new();
        let mut trans: Vec<BTreeMap<u8, u32>> = Vec::new();
        for &v in &order {
            let sig: Sig = (
                trie[v].fin,
                trie[v].kids.iter().map(|(&b, &c)| (b, canon[c])).collect(),
            );
            let id = match registry.get(&sig) {
                Some(&id) => id,
                None => {
                    let id = fin.len() as u32;
                    fin.push(trie[v].fin);
                    trans.push(trie[v].kids.iter().map(|(&b, &c)| (b, canon[c])).collect());
                    registry.insert(sig, id);
                    id
                }
            };
            canon[v] = id;
        }
        // Root must be state 0 for the documented API; the
        // register pass may have assigned it elsewhere —
        // swap it into place and fix every reference.
        let root = canon[0] as usize;
        if root != 0 {
            fin.swap(0, root);
            trans.swap(0, root);
            for t in trans.iter_mut() {
                for v in t.values_mut() {
                    if *v as usize == root {
                        *v = 0;
                    } else if *v == 0 {
                        *v = root as u32;
                    }
                }
            }
        }
        Fst { final_: fin, trans }
    }

    /// Number of states in the minimal automaton.
    pub fn states(&self) -> usize {
        self.final_.len()
    }

    /// Follow one transition from state `s` on byte `b`.
    pub fn next_state(&self, s: u32, b: u8) -> Option<u32> {
        self.trans.get(s as usize)?.get(&b).copied()
    }

    /// Whether state `s` accepts (ends a word).
    pub fn is_final(&self, s: u32) -> bool {
        self.final_.get(s as usize).copied().unwrap_or(false)
    }

    /// Membership test.
    pub fn contains(&self, word: &[u8]) -> bool {
        let mut s = 0u32;
        for &b in word {
            match self.next_state(s, b) {
                Some(n) => s = n,
                None => return false,
            }
        }
        self.is_final(s)
    }

    /// Length of the longest prefix of `word` that leads
    /// to a valid state — dictionary-completion primitive.
    pub fn longest_prefix(&self, word: &[u8]) -> usize {
        let mut s = 0u32;
        let mut n = 0usize;
        for &b in word {
            match self.next_state(s, b) {
                Some(nx) => {
                    s = nx;
                    n += 1;
                }
                None => break,
            }
        }
        n
    }

    /// Emit every accepted word, sorted.
    pub fn enumerate(&self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut cur = Vec::new();
        fn dfs(f: &Fst, s: u32, cur: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
            if f.is_final(s) {
                out.push(cur.clone());
            }
            for (&b, &t) in &f.trans[s as usize] {
                cur.push(b);
                dfs(f, t, cur, out);
                cur.pop();
            }
        }
        dfs(self, 0, &mut cur, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn random_words(rng: &mut SplitMix64, n: usize) -> BTreeSet<Vec<u8>> {
        (0..n)
            .map(|_| {
                (0..rng.below(10) + 1)
                    .map(|_| b'a' + rng.below(5) as u8)
                    .collect()
            })
            .collect()
    }

    #[test]
    fn basics() {
        let d = Fst::compile(&[b"car".as_ref(), b"card", b"care", b"dog"]);
        for w in [&b"car"[..], b"card", b"care", b"dog"] {
            assert!(d.contains(w));
        }
        for w in [&b"ca"[..], b"c", b"cardd", b"do", b"dogs", b"", b"cat"] {
            assert!(!d.contains(w));
        }
        // empty dict → rejects everything
        let e = Fst::compile(&[]);
        assert!(!e.contains(b"a"));
        assert!(!e.contains(b""));
        // dict containing "" accepts it
        let z = Fst::compile(&[b"".as_ref()]);
        assert!(z.contains(b""));
        assert!(!z.contains(b"a"));
        assert_eq!(d.longest_prefix(b"carpet"), 3);
        assert_eq!(d.longest_prefix(b"xy"), 0);
    }

    /// Invariants the register pass must leave: every
    /// state reachable from the root, and no two states
    /// sharing an identical signature (a missed merge).
    #[test]
    fn invariant_reachable_and_merged() {
        let mut rng = SplitMix64::new(0xF57);
        for _ in 0..40 {
            let words = random_words(&mut rng, 30);
            let wv: Vec<&[u8]> = words.iter().map(|v| v.as_ref()).collect();
            let d = Fst::compile(&wv);
            // reachability
            let mut seen = vec![false; d.states()];
            let mut stack = vec![0u32];
            seen[0] = true;
            while let Some(s) = stack.pop() {
                for &t in d.trans[s as usize].values() {
                    if !seen[t as usize] {
                        seen[t as usize] = true;
                        stack.push(t);
                    }
                }
            }
            assert!(seen.iter().all(|&b| b), "unreachable state");
            // no two identical signatures
            let mut sigs = BTreeSet::new();
            for s in 0..d.states() {
                let sig: (bool, Vec<(u8, u32)>) = (
                    d.is_final(s as u32),
                    d.trans[s].iter().map(|(&b, &t)| (b, t)).collect(),
                );
                assert!(sigs.insert(sig), "unmerged equivalent states");
            }
        }
    }

    /// BTreeSet oracle: random dictionaries — every member
    /// and random probes must agree exactly, and
    /// enumerate() must equal the set.
    #[test]
    fn oracle_membership_and_enumerate() {
        let mut rng = SplitMix64::new(0xDEAD);
        for _ in 0..80 {
            let nw = rng.below(40) as usize;
            let words = random_words(&mut rng, nw);
            let wv: Vec<&[u8]> = words.iter().map(|v| v.as_ref()).collect();
            let d = Fst::compile(&wv);
            for w in &words {
                assert!(d.contains(w), "missed {w:?}");
            }
            for _ in 0..50 {
                let probe: Vec<u8> = (0..rng.below(10) + 1)
                    .map(|_| b'a' + rng.below(5) as u8)
                    .collect();
                assert_eq!(d.contains(&probe), words.contains(&probe), "{probe:?}");
            }
            assert_eq!(d.enumerate(), words.iter().cloned().collect::<Vec<_>>());
        }
    }

    /// Order-independence: any permutation of the input
    /// yields the identical table.
    #[test]
    fn order_independent() {
        let a = Fst::compile(&[b"ab".as_ref(), b"ba", b"abc", b"b"]);
        let b = Fst::compile(&[b"b".as_ref(), b"abc", b"ba", b"ab"]);
        assert_eq!(a.enumerate(), b.enumerate());
        assert_eq!(a.trans, b.trans);
    }

    /// Minimality sanity on hand-computed cases.
    #[test]
    fn minimality_collapses() {
        // {a, ab, abc, abcd}: chain states root→a→ab→abc→
        // abcd; finals at a,ab,abc,abcd — each suffix class
        // distinct → 5 states is the minimum.
        let d = Fst::compile(&[b"a".as_ref(), b"ab", b"abc", b"abcd"]);
        assert_eq!(d.states(), 5);
        // all 8 length-3 strings over {a,b}: the 8 leaf
        // finals merge to 1, the 4 depth-2 nodes are all
        // equivalent? No — a depth-2 node's children cover
        // the SAME two symbols → 4 nodes all merge → 1;
        // depth-1 similarly merges → 1. Total:
        // root + d1 + d2 + leaf = 4 states.
        let d2 = Fst::compile(&[
            b"aaa".as_ref(),
            b"aab",
            b"aba",
            b"abb",
            b"baa",
            b"bab",
            b"bba",
            b"bbb",
        ]);
        assert_eq!(d2.states(), 4);
    }
}

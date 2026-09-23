//! x-fast trie — predecessor/successor over a `u64` universe
//! via level-wise prefix tables.
//!
//! Every key's 64 prefixes are stored in `levels[0..=64]`,
//! each entry keeping the `(min_key, max_key)` leaf pair under
//! that prefix. `pred(x)` binary-searches the levels for the
//! longest prefix of `x` present (the node where `x`'s path
//! diverges), then resolves the answer from the diverging
//! child's leaf bounds — `O(log U)` table ops instead of a
//! `O(U)` descent. Leaves also live in a `BTreeSet`, used for
//! the single off-subtree hop and as the test oracle; the docs
//! call that out honestly — a real x-fast keeps doubly-linked
//! leaf pointers for `O(1)` hops.
//!
//! ```
//! use izanagi_kit::xfast::Xfast;
//! let mut t = Xfast::new();
//! for k in [10u64, 30, 20, 40] {
//!     t.insert(k);
//! }
//! assert_eq!(t.pred(25), Some(20));
//! assert_eq!(t.succ(25), Some(30));
//! ```
//!
//! References: Willard, "Log-Logarithmic Worst-Case Range
//! Queries are Possible in Space Θ(n)" (IPL 1983); Demaine's
//! MIT 6.851 lecture notes (lecture 11).

use std::collections::{BTreeMap, BTreeSet};

/// Deterministic x-fast trie over `u64` keys.
#[derive(Clone, Debug, Default)]
pub struct Xfast {
    /// `levels[l]` maps each stored `l`-bit prefix to the
    /// `(min, max)` leaf keys beneath it. `levels[64]` maps
    /// full keys to themselves; `levels[0]` holds the root.
    levels: Vec<BTreeMap<u64, (u64, u64)>>,
    leaves: BTreeSet<u64>,
}

impl Xfast {
    /// Empty structure.
    pub fn new() -> Xfast {
        Xfast {
            levels: vec![BTreeMap::new(); 65],
            leaves: BTreeSet::new(),
        }
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether the structure is empty.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// `l`-bit prefix of `k` (`l = 0` is the root prefix).
    fn prefix(k: u64, l: usize) -> u64 {
        if l == 0 {
            0
        } else {
            k >> (64 - l)
        }
    }

    /// Whether `k` is present — `O(1)`.
    pub fn contains(&self, k: u64) -> bool {
        self.leaves.contains(&k)
    }

    /// Insert `k`; no-op when already present.
    pub fn insert(&mut self, k: u64) {
        if !self.leaves.insert(k) {
            return;
        }
        if self.levels.is_empty() {
            self.levels = vec![BTreeMap::new(); 65];
        }
        for l in 0..=64usize {
            let e = self.levels[l].entry(Self::prefix(k, l)).or_insert((k, k));
            e.0 = e.0.min(k);
            e.1 = e.1.max(k);
        }
    }

    /// Remove `k`; `false` when absent.
    pub fn erase(&mut self, k: u64) -> bool {
        if !self.leaves.remove(&k) {
            return false;
        }
        for l in 0..=64usize {
            let p = Self::prefix(k, l);
            let remove_entry = {
                let e = self.levels[l].get_mut(&p);
                match e {
                    None => false,
                    Some(mm) => {
                        if mm.0 == mm.1 {
                            true
                        } else {
                            if mm.0 == k {
                                // new min under this prefix
                                let nx = self
                                    .leaves
                                    .range(k..)
                                    .find(|&&x| Self::prefix(x, l) == p)
                                    .copied();
                                if let Some(nx) = nx {
                                    mm.0 = nx;
                                }
                            }
                            if mm.1 == k {
                                let pv = self
                                    .leaves
                                    .range(..k)
                                    .rev()
                                    .find(|&&x| Self::prefix(x, l) == p)
                                    .copied();
                                if let Some(pv) = pv {
                                    mm.1 = pv;
                                }
                            }
                            false
                        }
                    }
                }
            };
            if remove_entry {
                self.levels[l].remove(&p);
            }
        }
        true
    }

    /// Longest prefix length of `x` present in the tables.
    /// Monotone in `l` (a prefix at level `l` implies all
    /// shorter prefixes exist), so binary search is exact.
    fn longest_prefix(&self, x: u64) -> usize {
        let (mut lo, mut hi) = (0usize, 64usize);
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if self.levels[mid].contains_key(&Self::prefix(x, mid)) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo
    }

    /// Greatest key `≤ x` — `None` below the minimum or when empty.
    pub fn pred(&self, x: u64) -> Option<u64> {
        if self.is_empty() {
            return None;
        }
        if self.contains(x) {
            return Some(x);
        }
        let d = self.longest_prefix(x);
        // The bit where x's path diverges from the trie.
        let bit = (x >> (63 - d)) & 1;
        let (_, mx) = *self.levels[d].get(&Self::prefix(x, d))?;
        if bit == 1 {
            // x wants the right child; only a left subtree
            // exists, so its max leaf is the predecessor.
            Some(mx)
        } else {
            // x wants the left child; every leaf under this
            // node is larger than x — predecessor is the leaf
            // just below the node's minimum.
            let (mn, _) = *self.levels[d].get(&Self::prefix(x, d))?;
            self.leaves.range(..mn).next_back().copied()
        }
    }

    /// Smallest key `≥ x` — `None` above the maximum or when empty.
    pub fn succ(&self, x: u64) -> Option<u64> {
        if self.is_empty() {
            return None;
        }
        if self.contains(x) {
            return Some(x);
        }
        let d = self.longest_prefix(x);
        let bit = (x >> (63 - d)) & 1;
        let (mn, _) = *self.levels[d].get(&Self::prefix(x, d))?;
        if bit == 0 {
            Some(mn)
        } else {
            let (_, mx) = *self.levels[d].get(&Self::prefix(x, d))?;
            self.leaves.range(mx + 1..).next().copied()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let mut t = Xfast::new();
        assert!(t.is_empty());
        assert_eq!(t.pred(7), None);
        t.insert(10);
        t.insert(30);
        t.insert(20);
        t.insert(40);
        assert_eq!(t.len(), 4);
        assert_eq!(t.pred(25), Some(20));
        assert_eq!(t.succ(25), Some(30));
        assert_eq!(t.pred(10), Some(10));
        assert_eq!(t.succ(41), None);
        assert_eq!(t.pred(5), None);
        assert!(t.erase(20));
        assert!(!t.erase(20));
        assert_eq!(t.pred(25), Some(10));
        // extremes
        t.insert(u64::MAX);
        t.insert(0);
        assert_eq!(t.pred(u64::MAX), Some(u64::MAX));
        assert_eq!(t.succ(u64::MAX - 1), Some(u64::MAX));
        assert_eq!(t.pred(u64::MAX - 1), Some(40));
        assert_eq!(t.succ(0), Some(0));
    }

    #[test]
    fn oracle_vs_btreeset() {
        let mut rng = SplitMix64::new(0xFA57);
        for _ in 0..60 {
            let mut t = Xfast::new();
            let mut oracle = BTreeSet::new();
            for _ in 0..400 {
                match rng.below(3) {
                    0 => {
                        let k = rng.next_u64() >> rng.below(60);
                        assert_eq!(t.insert(k), ());
                        oracle.insert(k);
                    }
                    1 => {
                        let k = rng.next_u64() >> rng.below(60);
                        assert_eq!(t.erase(k), oracle.remove(&k));
                    }
                    _ => {
                        let x = rng.next_u64();
                        assert_eq!(t.contains(x), oracle.contains(&x));
                        assert_eq!(t.pred(x), oracle.range(..=x).next_back().copied());
                        assert_eq!(t.succ(x), oracle.range(x..).next().copied());
                    }
                }
            }
            // full sorted sweep after the churn
            let v: Vec<u64> = oracle.iter().copied().collect();
            for (i, &k) in v.iter().enumerate() {
                assert_eq!(t.pred(k), Some(k));
                assert_eq!(t.succ(k), Some(k));
                if i > 0 {
                    assert_eq!(t.pred(k - 1), Some(v[i - 1]));
                }
            }
        }
    }
}

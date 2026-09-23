//! y-fast–style clustered predecessor set over the `u32` universe —
//! Willard's two-tier layout: a top level of *representative* keys
//! that each own a bounded bucket covering a key interval
//! `(prev_rep, rep]`, so `predecessor`/`successor` answer by finding
//! the right bucket then scanning its ordered contents — no hashing
//! at all, so the structure is a pure function of the key sequence
//! applied to it and replays bit-identically across peers.
//!
//! Where `veb` splits the universe by fixed high bits, `yfast`
//! splits by *content*: buckets split at their median whenever they
//! outgrow `BUCKET`, keeping every probe `O(log n + BUCKET)` and the
//! rep layer small (`≤ n/BUCKET` entries).
//!
//! ```
//! use izanagi_kit::yfast::YFast;
//!
//! let mut y = YFast::new();
//! for k in [10u32, 3, 70_000, 5] {
//!     y.insert(k);
//! }
//! assert_eq!(y.predecessor(9), Some(5)); // strict < x
//! assert_eq!(y.successor(10), Some(10)); // inclusive ≥ x
//! assert_eq!(y.min(), Some(3));
//! assert_eq!(y.max(), Some(70_000));
//! ```
//!
//! References: Willard, "Log-logarithmic worst-case range queries
//! are possible in space Θ(n)" (1983).

use std::collections::{BTreeMap, BTreeSet};

/// Members per bucket before a median split — sized like log₂(U).
const BUCKET: usize = 32;

/// Sorted `u32` set as a y-fast–style rep-ordered clustered trie.
pub struct YFast {
    /// Rep keys in order; `reps[i]`'s bucket covers `(reps[i−1], reps[i]]`
    /// (the first bucket covers `(0..=reps[0]]`).
    reps: Vec<u32>,
    /// `rep → ordered member bucket` (rep itself is a member).
    buckets: BTreeMap<u32, BTreeSet<u32>>,
    len: usize,
}

impl Default for YFast {
    fn default() -> Self {
        Self::new()
    }
}

impl YFast {
    /// Empty set.
    pub fn new() -> Self {
        Self {
            reps: Vec::new(),
            buckets: BTreeMap::new(),
            len: 0,
        }
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Bucket index whose interval may contain `x`: the first rep
    /// `≥ x`, or `None` past the last rep.
    fn home(&self, x: u32) -> Option<usize> {
        let i = self.reps.partition_point(|&r| r < x);
        if i < self.reps.len() {
            Some(i)
        } else {
            None
        }
    }

    /// Insert `x`; returns `false` when already present.
    pub fn insert(&mut self, x: u32) -> bool {
        if self.reps.is_empty() {
            self.reps.push(x);
            self.buckets.insert(x, BTreeSet::from([x]));
            self.len += 1;
            return true;
        }
        match self.home(x) {
            None => {
                // Past the last rep: append a fresh bucket.
                self.reps.push(x);
                self.buckets.insert(x, BTreeSet::from([x]));
            }
            Some(i) => {
                let rep = self.reps[i];
                let bucket = match self.buckets.get_mut(&rep) {
                    Some(b) => b,
                    None => return false,
                };
                if !bucket.insert(x) {
                    return false;
                }
                self.len += 1;
                if bucket.len() > 2 * BUCKET {
                    self.split_bucket(i);
                }
                return true;
            }
        }
        self.len += 1;
        true
    }

    /// Split bucket `i` at its median: the larger half keeps `rep`,
    /// the smaller half gets the median as a new rep covering
    /// `(prev_rep, median]`.
    fn split_bucket(&mut self, i: usize) {
        let rep = self.reps[i];
        let (Some(bucket), _) = (self.buckets.remove(&rep), ()) else {
            return;
        };
        let median = bucket.iter().nth(bucket.len() / 2).copied().unwrap_or(rep);
        let (mut low, mut high) = (BTreeSet::new(), BTreeSet::new());
        for v in bucket {
            if v <= median {
                low.insert(v);
            } else {
                high.insert(v);
            }
        }
        self.buckets.insert(median, low);
        self.buckets.insert(rep, high);
        self.reps.insert(i, median);
    }

    /// Whether `x` is present.
    pub fn contains(&self, x: u32) -> bool {
        match self.home(x) {
            Some(i) => self
                .buckets
                .get(&self.reps[i])
                .is_some_and(|b| b.contains(&x)),
            None => false,
        }
    }

    /// Remove `x`; returns whether it was present. A bucket emptied
    /// of its last member drops its rep; when the removed key *was*
    /// the rep, the bucket's max takes over as rep.
    pub fn remove(&mut self, x: u32) -> bool {
        let Some(i) = self.home(x) else {
            return false;
        };
        let rep = self.reps[i];
        let Some(bucket) = self.buckets.get_mut(&rep) else {
            return false;
        };
        if !bucket.remove(&x) {
            return false;
        }
        self.len -= 1;
        if bucket.is_empty() {
            self.buckets.remove(&rep);
            self.reps.remove(i);
        } else if rep == x {
            // Rep removed: promote the bucket's max to rep —
            // the interval boundary `(prev, rep]` stays exact.
            let Some(&new_rep) = bucket.iter().next_back() else {
                return true;
            };
            let Some(mut b) = self.buckets.remove(&rep) else {
                return true;
            };
            b.insert(new_rep);
            self.buckets.insert(new_rep, b);
            self.reps[i] = new_rep;
        }
        true
    }

    /// Largest key `< x` (strict).
    pub fn predecessor(&self, x: u32) -> Option<u32> {
        if self.reps.is_empty() {
            return None;
        }
        // Try the bucket of the first rep ≥ x, then walk left.
        let mut i = self.reps.partition_point(|&r| r < x);
        loop {
            if i < self.reps.len() {
                let rep = self.reps[i];
                if let Some(b) = self.buckets.get(&rep) {
                    if let Some(&v) = b.range(..x).next_back() {
                        return Some(v);
                    }
                }
            }
            if i == 0 {
                return None;
            }
            i -= 1;
        }
    }

    /// Smallest key `≥ x` (inclusive — matches `veb` semantics).
    pub fn successor(&self, x: u32) -> Option<u32> {
        if self.reps.is_empty() {
            return None;
        }
        let mut i = self.reps.partition_point(|&r| r < x);
        loop {
            if i >= self.reps.len() {
                return None;
            }
            let rep = self.reps[i];
            if let Some(b) = self.buckets.get(&rep) {
                if let Some(&v) = b.range(x..).next() {
                    return Some(v);
                }
            }
            i += 1;
        }
    }

    /// Smallest key.
    pub fn min(&self) -> Option<u32> {
        self.reps
            .first()
            .and_then(|r| self.buckets.get(r))
            .and_then(|b| b.iter().next().copied())
    }

    /// Largest key.
    pub fn max(&self) -> Option<u32> {
        self.reps
            .last()
            .and_then(|r| self.buckets.get(r))
            .and_then(|b| b.iter().next_back().copied())
    }

    /// Keys in ascending order.
    pub fn iter(&self) -> Vec<u32> {
        let mut v = Vec::with_capacity(self.len);
        for r in &self.reps {
            if let Some(b) = self.buckets.get(r) {
                v.extend(b.iter().copied());
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basic_ops() {
        let mut y = YFast::new();
        assert!(y.is_empty());
        assert_eq!(y.min(), None);
        assert_eq!(y.max(), None);
        for k in [10u32, 3, 70_000, 5] {
            assert!(y.insert(k));
        }
        assert!(!y.insert(10));
        assert!(y.contains(5));
        assert!(!y.contains(4));
        assert_eq!(y.len(), 4);
        assert_eq!(y.min(), Some(3));
        assert_eq!(y.max(), Some(70_000));
        assert_eq!(y.iter(), vec![3, 5, 10, 70_000]);
        assert!(y.remove(5));
        assert!(!y.remove(5));
        assert_eq!(y.iter(), vec![3, 10, 70_000]);
    }

    #[test]
    fn pred_succ_semantics() {
        let mut y = YFast::new();
        for k in [10u32, 20, 30] {
            y.insert(k);
        }
        assert_eq!(y.predecessor(10), None); // strict
        assert_eq!(y.predecessor(11), Some(10));
        assert_eq!(y.predecessor(31), Some(30));
        assert_eq!(y.successor(10), Some(10)); // inclusive
        assert_eq!(y.successor(11), Some(20));
        assert_eq!(y.successor(31), None);
        assert_eq!(y.successor(0), Some(10));
    }

    #[test]
    fn rep_promotion() {
        // Force a rep removal: single-key buckets rep == key.
        let mut y = YFast::new();
        for k in [1u32, 5, 9, 100, 250] {
            y.insert(k);
        }
        assert!(y.remove(5));
        assert_eq!(y.iter(), vec![1, 9, 100, 250]);
        assert_eq!(y.successor(5), Some(9));
        assert_eq!(y.predecessor(9), Some(1));
    }

    #[test]
    fn oracle_random_ops() {
        let mut rng = SplitMix64::new(0x9FA5_7FA5);
        for _ in 0..60 {
            let mut y = YFast::new();
            let mut oracle = BTreeSet::new();
            for _ in 0..4000 {
                let x = (rng.next_u64() % 300) as u32;
                match rng.next_u64() % 4 {
                    0 => assert_eq!(y.insert(x), oracle.insert(x)),
                    1 => assert_eq!(y.remove(x), oracle.remove(&x)),
                    2 => assert_eq!(y.contains(x), oracle.contains(&x)),
                    _ => {
                        assert_eq!(y.predecessor(x), oracle.range(..x).next_back().copied());
                        assert_eq!(y.successor(x), oracle.range(x..).next().copied());
                    }
                }
                assert_eq!(y.len(), oracle.len());
            }
            assert_eq!(y.iter(), oracle.iter().copied().collect::<Vec<u32>>());
            assert_eq!(y.min(), oracle.iter().next().copied());
            assert_eq!(y.max(), oracle.iter().next_back().copied());
        }
    }

    #[test]
    fn bucket_splits_stay_correct() {
        // Sequential inserts concentrate one bucket → many splits.
        let mut y = YFast::new();
        for k in 0..500u32 {
            assert!(y.insert(k));
        }
        for k in 0..500u32 {
            assert!(y.contains(k));
            assert_eq!(y.successor(k), Some(k));
        }
        assert_eq!(y.predecessor(499), Some(498));
        assert_eq!(y.iter().len(), 500);
    }
}

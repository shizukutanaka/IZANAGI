//! Min-max heap — a double-ended priority queue (Atkinson, Sack,
//! Santoro & Strothotte 1986) where levels alternate *min* and
//! *max*: even levels store values no larger than every descendant,
//! odd levels no smaller. Both `peek_min` and `peek_max` are O(1),
//! and both pops sift down `O(log n)`.
//!
//! The maximum lives at the root's largest child (index 1 or 2),
//! never deeper — that is the whole trick: one array answers both
//! ends, which a plain binary heap cannot do.
//!
//! Deterministic by shape: the structure is a pure function of the
//! operation sequence — swaps follow fixed comparisons, no ties
//! broken arbitrarily.
//!
//! ```
//! use izanagi_kit::mmheap::MmHeap;
//! let mut h = MmHeap::new();
//! for k in [5, 1, 9, 3, 7] {
//!     h.push(k);
//! }
//! assert_eq!(h.peek_min(), Some(&1));
//! assert_eq!(h.peek_max(), Some(&9));
//! assert_eq!(h.pop_min(), Some(1));
//! assert_eq!(h.pop_max(), Some(9));
//! ```

/// Min-max double-ended heap over `u64` (multiset — duplicates kept).
#[derive(Clone, Debug, Default)]
pub struct MmHeap {
    a: Vec<u64>,
}

fn is_min_level(i: usize) -> bool {
    // Level of index i: index 0 is level 0 (a min level).
    (usize::BITS - (i + 1).leading_zeros() - 1) % 2 == 0
}

fn parent(i: usize) -> usize {
    (i - 1) / 2
}

impl MmHeap {
    /// New empty heap.
    pub fn new() -> MmHeap {
        MmHeap { a: Vec::new() }
    }

    /// Heapify — `O(n)` bottom-up sift-down (beats `n` pushes).
    pub fn from_slice(keys: &[u64]) -> MmHeap {
        let mut h = MmHeap { a: keys.to_vec() };
        for i in (0..h.a.len() / 2).rev() {
            h.sift_down(i);
        }
        h
    }

    /// Number of stored keys.
    pub fn len(&self) -> usize {
        self.a.len()
    }

    /// Empty check.
    pub fn is_empty(&self) -> bool {
        self.a.is_empty()
    }

    /// Smallest key — the root.
    pub fn peek_min(&self) -> Option<&u64> {
        self.a.first()
    }

    /// Largest key — the larger of the root's children.
    pub fn peek_max(&self) -> Option<&u64> {
        match self.a.len() {
            0 => None,
            1 => self.a.first(),
            2 => self.a.get(1),
            _ => Some(if self.a[1] >= self.a[2] {
                &self.a[1]
            } else {
                &self.a[2]
            }),
        }
    }

    /// Insert; `O(log n)` sift-up.
    pub fn push(&mut self, k: u64) {
        self.a.push(k);
        let i = self.a.len() - 1;
        if i == 0 {
            return;
        }
        let p = parent(i);
        if is_min_level(i) {
            if self.a[i] > self.a[p] {
                self.a.swap(i, p);
                self.sift_up_max(p);
            } else {
                self.sift_up_min(i);
            }
        } else if self.a[i] < self.a[p] {
            self.a.swap(i, p);
            self.sift_up_min(p);
        } else {
            self.sift_up_max(i);
        }
    }

    /// Remove and return the minimum.
    pub fn pop_min(&mut self) -> Option<u64> {
        if self.a.is_empty() {
            return None;
        }
        let last = self.a.pop()?;
        if self.a.is_empty() {
            return Some(last);
        }
        let out = self.a[0];
        self.a[0] = last;
        self.sift_down(0);
        Some(out)
    }

    /// Remove and return the maximum.
    pub fn pop_max(&mut self) -> Option<u64> {
        match self.a.len() {
            0 => None,
            1 | 2 => self.a.pop(),
            _ => {
                let m = if self.a[1] >= self.a[2] { 1 } else { 2 };
                let out = self.a[m];
                let last = self.a.pop()?;
                // When m was the tail slot the popped element IS the
                // max — nothing to move.
                if m < self.a.len() {
                    self.a[m] = last;
                    self.sift_down(m);
                }
                Some(out)
            }
        }
    }

    /// Debug invariant check — every min-level node ≤ descendants,
    /// every max-level node ≥ descendants. `O(n²)`; tests only.
    pub fn check(&self) -> bool {
        for i in 0..self.a.len() {
            for j in (i + 1)..self.a.len() {
                // j is a descendant of i iff i is an ancestor of j.
                let mut p = j;
                let mut desc = false;
                while p > 0 {
                    p = parent(p);
                    if p == i {
                        desc = true;
                        break;
                    }
                }
                if desc {
                    if is_min_level(i) && self.a[i] > self.a[j] {
                        return false;
                    }
                    if !is_min_level(i) && self.a[i] < self.a[j] {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Sorted-by-min drain (destructive convenience for tests).
    pub fn drain_sorted(mut self) -> Vec<u64> {
        let mut out = Vec::with_capacity(self.a.len());
        while let Some(v) = self.pop_min() {
            out.push(v);
        }
        out
    }

    fn sift_up_min(&mut self, mut i: usize) {
        while i >= 3 {
            let g = parent(parent(i));
            if self.a[i] < self.a[g] {
                self.a.swap(i, g);
                i = g;
            } else {
                break;
            }
        }
    }

    fn sift_up_max(&mut self, mut i: usize) {
        while i >= 3 {
            let g = parent(parent(i));
            if self.a[i] > self.a[g] {
                self.a.swap(i, g);
                i = g;
            } else {
                break;
            }
        }
    }

    fn sift_down(&mut self, i: usize) {
        if is_min_level(i) {
            self.sift_down_side(i, true);
        } else {
            self.sift_down_side(i, false);
        }
    }

    /// `min_side`: pull toward min levels (smallest wins);
    /// otherwise toward max levels.
    fn sift_down_side(&mut self, mut i: usize, min_side: bool) {
        loop {
            // Best candidate among children and grandchildren.
            let mut best = i;
            let mut best_val = self.a[i];
            let mut best_is_grandchild = false;
            for c in [2 * i + 1, 2 * i + 2] {
                if c < self.a.len() {
                    let better = if min_side {
                        self.a[c] < best_val
                    } else {
                        self.a[c] > best_val
                    };
                    if better {
                        best = c;
                        best_val = self.a[c];
                        best_is_grandchild = false;
                    }
                    for g in [2 * c + 1, 2 * c + 2] {
                        if g < self.a.len() {
                            let gb = if min_side {
                                self.a[g] < best_val
                            } else {
                                self.a[g] > best_val
                            };
                            if gb {
                                best = g;
                                best_val = self.a[g];
                                best_is_grandchild = true;
                            }
                        }
                    }
                }
            }
            if best == i {
                return;
            }
            self.a.swap(i, best);
            if best_is_grandchild {
                let p = parent(best);
                let fix = if min_side {
                    self.a[best] > self.a[p]
                } else {
                    self.a[best] < self.a[p]
                };
                if fix {
                    self.a.swap(best, p);
                }
                i = best;
            } else {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    fn shadow_pop_min(s: &mut BTreeMap<u64, u64>) -> Option<u64> {
        let (&k, _) = s.iter().next()?;
        let c = s.get_mut(&k)?;
        *c -= 1;
        if *c == 0 {
            s.remove(&k);
        }
        Some(k)
    }

    fn shadow_pop_max(s: &mut BTreeMap<u64, u64>) -> Option<u64> {
        let (&k, _) = s.iter().next_back()?;
        let c = s.get_mut(&k)?;
        *c -= 1;
        if *c == 0 {
            s.remove(&k);
        }
        Some(k)
    }

    #[test]
    fn basics() {
        let mut h = MmHeap::new();
        assert!(h.is_empty());
        assert_eq!(h.peek_min(), None);
        assert_eq!(h.pop_max(), None);
        for k in [5, 1, 9, 3, 7, 1] {
            h.push(k);
        }
        assert_eq!(h.len(), 6);
        assert_eq!(h.peek_min(), Some(&1));
        assert_eq!(h.peek_max(), Some(&9));
        assert!(h.check());
        assert_eq!(h.pop_min(), Some(1));
        assert_eq!(h.pop_max(), Some(9));
        assert!(h.check());
        assert_eq!(h.drain_sorted(), vec![1, 3, 5, 7]);
    }

    #[test]
    fn heapify() {
        let h = MmHeap::from_slice(&[9, 1, 8, 2, 7, 3, 6, 4, 5]);
        assert!(h.check());
        assert_eq!(h.peek_min(), Some(&1));
        assert_eq!(h.peek_max(), Some(&9));
        assert_eq!(h.drain_sorted(), vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn oracle_interleaved() {
        let mut rng = SplitMix64::new(0x1234_5678_9abc_def0);
        for _case in 0..20 {
            let mut h = MmHeap::new();
            let mut shadow: BTreeMap<u64, u64> = BTreeMap::new();
            for _op in 0..2000 {
                match rng.below(10) {
                    0..=4 => {
                        let k = rng.next_u64() % 200;
                        h.push(k);
                        *shadow.entry(k).or_insert(0) += 1;
                    }
                    5..=7 => {
                        let expect = shadow.iter().next().map(|(k, _)| *k);
                        assert_eq!(h.pop_min(), expect);
                        let _ = shadow_pop_min(&mut shadow);
                    }
                    _ => {
                        let expect = shadow.iter().next_back().map(|(k, _)| *k);
                        assert_eq!(h.pop_max(), expect);
                        let _ = shadow_pop_max(&mut shadow);
                    }
                }
                assert_eq!(h.len() as u64, shadow.values().sum::<u64>());
            }
            assert!(h.check());
        }
    }

    #[test]
    fn edge_cases() {
        let mut h = MmHeap::new();
        h.push(42);
        assert_eq!(h.peek_max(), Some(&42));
        h.push(7);
        assert_eq!(h.peek_max(), Some(&42));
        assert_eq!(h.pop_max(), Some(42));
        assert_eq!(h.pop_max(), Some(7));
        assert_eq!(h.pop_max(), None);
        // Adversarial shapes: strictly ascending / descending inserts.
        let asc = MmHeap::from_slice(&(0..64).collect::<Vec<u64>>());
        assert!(asc.check());
        let desc = MmHeap::from_slice(&(0..64).rev().collect::<Vec<u64>>());
        assert!(desc.check());
    }
}

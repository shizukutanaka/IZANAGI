//! Indexed binary heap — a priority queue where entries are
//! *named* by a fixed key space `0..cap`, so `set` upserts and
//! `decrease`/`increase` adjust in place (the Dijkstra pattern).
//! Priority is `i64`; ties break on the smaller key, so pop order
//! is a pure function of the operation sequence — canonical, never
//! address- or insertion-order-dependent.
//!
//! [`crate::pathfinding`] hard-codes its own open set; this is the
//! reusable version for any seeded best-first procedure.
//!
//! ```
//! use izanagi_kit::iheap::IHeap;
//! let mut h = IHeap::new(4).unwrap();
//! h.set(0, 10);
//! h.set(1, 5);
//! h.decrease(0, 3); // 0 now beats 1
//! assert_eq!(h.pop(), Some((0, 3)));
//! assert_eq!(h.peek(), Some((1, 5)));
//! ```

/// An indexed min-heap over keys `0..cap` with `i64` priorities.
#[derive(Clone, Debug)]
pub struct IHeap {
    /// heap slot order — heap[i] is a key.
    heap: Vec<u32>,
    /// priority per key — u64::MAX slot = absent.
    prio: Vec<Option<i64>>,
    /// key -> heap index.
    pos: Vec<u32>,
}

impl IHeap {
    /// Heap over key space `0..cap` — `None` when `cap == 0`.
    /// `cap` exceeding `u32::MAX` is also rejected.
    pub fn new(cap: usize) -> Option<Self> {
        if cap == 0 || cap > u32::MAX as usize {
            return None;
        }
        Some(IHeap {
            heap: Vec::new(),
            prio: vec![None; cap],
            pos: vec![0; cap],
        })
    }

    /// Entries currently held.
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// `len == 0`.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Whether `key` is in the heap.
    pub fn contains(&self, key: u32) -> bool {
        self.prio.get(key as usize).is_some_and(|p| p.is_some())
    }

    /// Current priority of `key` — `None` when absent or OOB.
    pub fn get(&self, key: u32) -> Option<i64> {
        self.prio.get(key as usize).copied().flatten()
    }

    /// Upsert `key` at `priority` — `false` when `key >= cap`.
    pub fn set(&mut self, key: u32, priority: i64) -> bool {
        let k = key as usize;
        if k >= self.prio.len() {
            return false;
        }
        match self.prio[k] {
            None => {
                self.prio[k] = Some(priority);
                self.pos[k] = self.heap.len() as u32;
                self.heap.push(key);
                self.sift_up(self.heap.len() - 1);
            }
            Some(old) => {
                self.prio[k] = Some(priority);
                let i = self.pos[k] as usize;
                if priority < old {
                    self.sift_up(i);
                } else if priority > old {
                    self.sift_down(i);
                }
            }
        }
        true
    }

    /// Lower `key`'s priority — `false` when absent, OOB, or the new
    /// priority is not actually lower.
    pub fn decrease(&mut self, key: u32, priority: i64) -> bool {
        match self.get(key) {
            Some(old) if priority < old => {
                self.set(key, priority);
                true
            }
            _ => false,
        }
    }

    /// Raise `key`'s priority — `false` when absent, OOB, or the new
    /// priority is not actually higher.
    pub fn increase(&mut self, key: u32, priority: i64) -> bool {
        match self.get(key) {
            Some(old) if priority > old => {
                self.set(key, priority);
                true
            }
            _ => false,
        }
    }

    /// Remove `key` — `false` when absent or OOB.
    pub fn remove(&mut self, key: u32) -> bool {
        let k = key as usize;
        if k >= self.prio.len() || self.prio[k].is_none() {
            return false;
        }
        let i = self.pos[k] as usize;
        let last = self.heap.len() - 1;
        self.swap_slots(i, last);
        self.heap.pop();
        self.prio[k] = None;
        if i < self.heap.len() {
            // the swapped-down element may need to rise or sink
            self.sift_up(i);
            self.sift_down(i);
        }
        true
    }

    /// Minimum `(key, priority)` — `None` when empty.
    pub fn peek(&self) -> Option<(u32, i64)> {
        self.heap
            .first()
            .and_then(|&k| self.prio[k as usize].map(|p| (k, p)))
    }

    /// Pop the minimum `(key, priority)`.
    pub fn pop(&mut self) -> Option<(u32, i64)> {
        let top = *self.heap.first()?;
        let p = self.prio[top as usize]?;
        let last = self.heap.len() - 1;
        self.swap_slots(0, last);
        self.heap.pop();
        self.prio[top as usize] = None;
        if !self.heap.is_empty() {
            self.sift_down(0);
        }
        Some((top, p))
    }

    fn less(&self, a: u32, b: u32) -> bool {
        // (priority, key) lexicographic — key tie-break is canonical
        (self.prio[a as usize], a) < (self.prio[b as usize], b)
    }

    fn swap_slots(&mut self, i: usize, j: usize) {
        self.heap.swap(i, j);
        self.pos[self.heap[i] as usize] = i as u32;
        self.pos[self.heap[j] as usize] = j as u32;
    }

    fn sift_up(&mut self, mut i: usize) {
        while i > 0 {
            let p = (i - 1) / 2;
            if self.less(self.heap[i], self.heap[p]) {
                self.swap_slots(i, p);
                i = p;
            } else {
                break;
            }
        }
    }

    fn sift_down(&mut self, mut i: usize) {
        loop {
            let l = 2 * i + 1;
            let r = l + 1;
            let mut m = i;
            if l < self.heap.len() && self.less(self.heap[l], self.heap[m]) {
                m = l;
            }
            if r < self.heap.len() && self.less(self.heap[r], self.heap[m]) {
                m = r;
            }
            if m == i {
                return;
            }
            self.swap_slots(i, m);
            i = m;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn pop_order_matches_sorted_oracle() {
        let mut rng = SplitMix64::new(0x1DEA);
        for _ in 0..200 {
            let cap = 1 + rng.below(24) as usize;
            let mut h = IHeap::new(cap).unwrap();
            let mut oracle: BTreeMap<u32, i64> = BTreeMap::new();
            for _ in 0..300 {
                match rng.below(6) {
                    0 => {
                        let k = rng.below(cap as u32);
                        let p = rng.below(60) as i64 - 30;
                        assert!(h.set(k, p));
                        oracle.insert(k, p);
                    }
                    1 => {
                        let k = rng.below(cap as u32);
                        let p = rng.below(60) as i64 - 30;
                        let ok = h.decrease(k, p);
                        let want = oracle.get(&k).is_some_and(|&o| p < o);
                        assert_eq!(ok, want);
                        if ok {
                            oracle.insert(k, p);
                        }
                    }
                    2 => {
                        let k = rng.below(cap as u32);
                        let p = rng.below(60) as i64 - 30;
                        let ok = h.increase(k, p);
                        let want = oracle.get(&k).is_some_and(|&o| p > o);
                        assert_eq!(ok, want);
                        if ok {
                            oracle.insert(k, p);
                        }
                    }
                    3 => {
                        let k = rng.below(cap as u32);
                        let ok = h.remove(k);
                        assert_eq!(ok, oracle.remove(&k).is_some());
                    }
                    4 => {
                        let got = h.pop();
                        let want = oracle
                            .iter()
                            .min_by_key(|(&k, &p)| (p, k))
                            .map(|(&k, &p)| (k, p));
                        assert_eq!(got, want);
                        if let Some((k, _)) = want {
                            oracle.remove(&k);
                        }
                    }
                    _ => {
                        let k = rng.below(cap as u32);
                        assert_eq!(h.get(k), oracle.get(&k).copied());
                        assert_eq!(h.contains(k), oracle.contains_key(&k));
                    }
                }
                assert_eq!(h.len(), oracle.len());
                assert_eq!(h.is_empty(), oracle.is_empty());
                let want = oracle
                    .iter()
                    .min_by_key(|(&k, &p)| (p, k))
                    .map(|(&k, &p)| (k, p));
                assert_eq!(h.peek(), want);
            }
        }
    }

    #[test]
    fn sorted_drain_and_validation() {
        let mut h = IHeap::new(8).unwrap();
        for (k, p) in [(3u32, 9i64), (0, 9), (7, 1), (2, 5)] {
            h.set(k, p);
        }
        // equal priorities drain in key order
        assert_eq!(h.pop(), Some((7, 1)));
        assert_eq!(h.pop(), Some((2, 5)));
        assert_eq!(h.pop(), Some((0, 9)));
        assert_eq!(h.pop(), Some((3, 9)));
        assert!(h.pop().is_none());
        // OOB rejected everywhere
        assert!(!h.set(8, 0));
        assert!(!h.decrease(8, 0));
        assert!(!h.increase(8, 0));
        assert!(!h.remove(8));
        assert!(h.get(8).is_none());
        assert!(IHeap::new(0).is_none());
        // set twice on same key = update, not duplicate
        h.set(1, 10);
        h.set(1, 2);
        assert_eq!(h.len(), 1);
        assert_eq!(h.get(1), Some(2));
    }
}

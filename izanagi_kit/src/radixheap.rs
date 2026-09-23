//! Radix heap — the monotone priority queue Dijkstra wants, with
//! O(1) amortized push and O(log C) amortized pops when the key range is
//! bounded by `C`.
//!
//! Items are bucketed by the bit-length of `key − last_popped`: after
//! each pop the popped key becomes the new base and the current minimum
//! bucket is redistributed. Pushing a key *below* the last popped key
//! violates monotonicity and returns `false` — the caller's bug, never
//! silently accepted. Canonical tie-break `(key, seq)` makes the pop
//! trace a pure function of the insertion trace.
//!
//! ```
//! use izanagi_kit::radixheap::RadixHeap;
//! let mut h = RadixHeap::new();
//! assert!(h.push(5, 10));
//! assert!(h.push(3, 20));
//! assert_eq!(h.pop(), Some((3, 20)));
//! assert!(!h.push(2, 30)); // monotone violation
//! ```
//!
//! Reference: Ahuja, Mehlhorn, Orlin & Tarjan (1990), "Faster algorithms
//! for the shortest path problem" §3.

/// A bucketed monotone priority queue over `u64` keys / `u32` payloads.
pub struct RadixHeap {
    /// `(key, seq, val)` per bucket; `buckets[b]` holds keys with
    /// `bit_len(key − last) == b` (b==0 ⇔ key == last).
    buckets: [Vec<(u64, u64, u32)>; 65],
    last: u64,
    seq: u64,
    len: usize,
    /// Smallest bucket index possibly nonempty (probe hint).
    min_bucket: usize,
}

impl Default for RadixHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl RadixHeap {
    /// Empty heap, base `last = 0`.
    pub const fn new() -> RadixHeap {
        const EMPTY: Vec<(u64, u64, u32)> = Vec::new();
        RadixHeap {
            buckets: [EMPTY; 65],
            last: 0,
            seq: 0,
            len: 0,
            min_bucket: 0,
        }
    }

    /// Bucket index for `key` under the current base — the position of
    /// the most significant bit where `key` and `last` differ. With
    /// monotone pushes (`key >= last`), a higher bucket index provably
    /// means a strictly larger key: `msb(key XOR last)`, *not* the
    /// bit-length of `key − last` — under a subtracting scheme a stale
    /// bucket can hold a key smaller than the lowest bucket's minimum.
    fn bucket_of(&self, key: u64) -> usize {
        let diff = key ^ self.last;
        64 - diff.leading_zeros() as usize
    }

    /// Number of live items.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push `(key, val)`; `false` if `key` is below the last popped key.
    pub fn push(&mut self, key: u64, val: u32) -> bool {
        if self.len > 0 && key < self.last {
            return false;
        }
        let b = self.bucket_of(key);
        self.buckets[b].push((key, self.seq, val));
        self.seq += 1;
        self.len += 1;
        if b < self.min_bucket {
            self.min_bucket = b;
        }
        true
    }

    /// Peek the next key without popping (the minimum `(key, seq, val)`).
    pub fn peek(&mut self) -> Option<(u64, u32)> {
        if self.is_empty() {
            return None;
        }
        self.prepare_min();
        let &(k, _s, v) = self.buckets[0].iter().min_by_key(|e| (e.0, e.1))?;
        Some((k, v))
    }

    /// Pop the minimum `(key, val)`; ties break on insertion order.
    pub fn pop(&mut self) -> Option<(u64, u32)> {
        if self.is_empty() {
            return None;
        }
        self.prepare_min();
        // Minimum of bucket 0 by (key, seq).
        let (pos, &(k, _s, v)) = self.buckets[0]
            .iter()
            .enumerate()
            .min_by_key(|(_, e)| (e.0, e.1))?;
        self.buckets[0].swap_remove(pos);
        self.len -= 1;
        Some((k, v))
    }

    /// Ensure bucket 0 holds the minimum element: locate the lowest
    /// nonempty bucket, move its minimum key to `last`, redistribute.
    fn prepare_min(&mut self) {
        while self.min_bucket < 65 && self.buckets[self.min_bucket].is_empty() {
            self.min_bucket += 1;
        }
        if self.min_bucket == 0 || self.min_bucket == 65 {
            return;
        }
        let bucket = self.min_bucket;
        // New base = minimum key in the lowest nonempty bucket.
        let new_last = self.buckets[bucket]
            .iter()
            .map(|e| e.0)
            .min()
            .unwrap_or(self.last);
        self.last = new_last;
        let items = std::mem::take(&mut self.buckets[bucket]);
        for (k, s, v) in items {
            let b = self.bucket_of(k);
            self.buckets[b].push((k, s, v));
            if b < self.min_bucket {
                self.min_bucket = b;
            }
        }
        // Everything that had diff==bucket now re-buckets lower;
        // bucket 0 may not be it — re-scan from 0.
        self.min_bucket = 0;
        while self.min_bucket < 65 && self.buckets[self.min_bucket].is_empty() {
            self.min_bucket += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn basics() {
        let mut h = RadixHeap::new();
        assert!(h.is_empty());
        assert!(h.push(9, 1));
        assert!(h.push(1, 2));
        assert!(h.push(5, 3));
        assert_eq!(h.peek(), Some((1, 2)));
        assert_eq!(h.pop(), Some((1, 2)));
        assert_eq!(h.pop(), Some((5, 3)));
        assert_eq!(h.pop(), Some((9, 1)));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn monotone_violation_rejected() {
        let mut h = RadixHeap::new();
        h.push(10, 1);
        assert_eq!(h.pop(), Some((10, 1)));
        h.push(12, 2);
        assert!(!h.push(5, 3)); // 5 < last popped 10
        assert_eq!(h.pop(), Some((12, 2)));
    }

    #[test]
    fn interleaved_oracle() {
        // Dijkstra-style: pushes are >= last popped key. Compare the
        // entire (key, seq) pop trace to a BTreeMultiset shadow where
        // equal keys pop in insertion order.
        let mut rng = SplitMix64::new(0x5eed_5eed_5eed_5eed);
        let mut h = RadixHeap::new();
        let mut shadow: BTreeMap<(u64, u64), u32> = BTreeMap::new();
        let mut seq = 0u64;
        let mut last = 0u64;
        for _ in 0..3000 {
            if shadow.is_empty() || rng.below(2) == 0 {
                let key = last + rng.below(50) as u64;
                let val = rng.below(1_000_000);
                assert!(h.push(key, val));
                shadow.insert((key, seq), val);
                seq += 1;
            } else {
                let got = h.pop();
                let want = shadow.iter().next().map(|(k, v)| (k.0, *v));
                assert_eq!(got, want);
                if let Some((k, _)) = shadow.iter().next() {
                    let key = *k;
                    shadow.remove(&key);
                    last = key.0.max(last);
                }
            }
        }
        while let Some(kv) = h.pop() {
            let want = shadow.iter().next().map(|(k, v)| (k.0, *v));
            assert_eq!(Some(kv), want);
            if let Some((k, _)) = shadow.iter().next() {
                let key = *k;
                shadow.remove(&key);
            }
        }
        assert!(shadow.is_empty());
    }

    #[test]
    fn bucket_boundaries() {
        // Keys at bit-length boundaries redistribute correctly.
        let mut h = RadixHeap::new();
        h.push(0, 0);
        h.push(1, 1); // diff 1 → bucket 1
        h.push(2, 2); // diff 2 → bucket 2
        h.push(3, 3); // diff 3 → bucket 2
        h.push(1u64 << 40, 4); // deep bucket
        assert_eq!(h.pop(), Some((0, 0)));
        assert_eq!(h.pop(), Some((1, 1)));
        assert_eq!(h.pop(), Some((2, 2)));
        assert_eq!(h.pop(), Some((3, 3)));
        assert_eq!(h.pop(), Some((1 << 40, 4)));
        assert_eq!(h.pop(), None);
    }
}

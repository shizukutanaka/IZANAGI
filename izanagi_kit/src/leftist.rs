//! Leftist heap — a mergeable priority queue where every node
//! keeps its *rank* (null-path length: distance to the nearest
//! empty slot) and the left spine is always the heavy side:
//! `rank(left) ≥ rank(right)`. All work happens on the right
//! spine, which is `O(log n)` long by construction — `merge` is
//! the only real operation; `push`/`pop`/`heapify` reduce to it.
//!
//! Deterministic by shape: the structure is a pure function of
//! the merge sequence — no tie-break randomness needed since
//! equal keys merge left-first.
//!
//! ```
//! use izanagi_kit::leftist::LeftistHeap;
//! let mut h = LeftistHeap::new();
//! for k in [9, 3, 7, 1] {
//!     h.push(k);
//! }
//! assert_eq!(h.peek(), Some(&1));
//! assert_eq!(h.pop(), Some(1));
//! assert_eq!(h.pop(), Some(3));
//! ```

const NIL: usize = !0;

#[derive(Clone, Debug)]
struct Node {
    key: u64,
    left: usize,
    right: usize,
    rank: u32,
}

/// Min-first leftist heap over `u64` (multiset — duplicates kept).
#[derive(Clone, Debug)]
pub struct LeftistHeap {
    nodes: Vec<Node>,
    root: usize,
    len: usize,
}

impl Default for LeftistHeap {
    fn default() -> Self {
        LeftistHeap::new()
    }
}

impl LeftistHeap {
    /// New empty heap.
    pub fn new() -> LeftistHeap {
        LeftistHeap {
            nodes: Vec::new(),
            root: NIL,
            len: 0,
        }
    }

    /// Heapify `keys` — `O(n)` pairwise-meld bottom-up (better
    /// than `n` sequential pushes' `O(n log n)`).
    pub fn from_slice(keys: &[u64]) -> LeftistHeap {
        let mut h = LeftistHeap::new();
        let mut queue: Vec<usize> = Vec::with_capacity(keys.len());
        for &k in keys {
            h.nodes.push(Node {
                key: k,
                left: NIL,
                right: NIL,
                rank: 1,
            });
            queue.push(h.nodes.len() - 1);
        }
        // meld adjacent pairs until one heap remains
        while queue.len() > 1 {
            let mut next = Vec::with_capacity(queue.len().div_ceil(2));
            let mut i = 0;
            while i < queue.len() {
                if i + 1 < queue.len() {
                    next.push(h.merge(queue[i], queue[i + 1]));
                } else {
                    next.push(queue[i]);
                }
                i += 2;
            }
            queue = next;
        }
        h.root = if queue.is_empty() { NIL } else { queue[0] };
        h.len = keys.len();
        h
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn rank(&self, x: usize) -> u32 {
        if x == NIL {
            0
        } else {
            self.nodes[x].rank
        }
    }

    /// Merge two sub-heaps — the core operation. Result keeps
    /// the smaller key at the root; recursion on the right spine
    /// keeps `O(log n)` depth, then rank fix-up restores the
    /// left-leaning shape.
    fn merge(&mut self, a: usize, b: usize) -> usize {
        if a == NIL {
            return b;
        }
        if b == NIL {
            return a;
        }
        let (a, b) = if self.nodes[b].key < self.nodes[a].key {
            (b, a)
        } else {
            (a, b)
        };
        let r = self.merge(self.nodes[a].right, b);
        self.nodes[a].right = r;
        // leftist invariant: left rank ≥ right rank
        if self.rank(self.nodes[a].left) < self.rank(self.nodes[a].right) {
            let (l, r) = (self.nodes[a].left, self.nodes[a].right);
            self.nodes[a].left = r;
            self.nodes[a].right = l;
        }
        self.nodes[a].rank = self.rank(self.nodes[a].right) + 1;
        a
    }

    /// Meld `other` into `self` (drains `other`).
    pub fn meld(&mut self, other: &mut LeftistHeap) {
        if other.root == NIL {
            return;
        }
        // foreign arena — copy keys into our arena then merge
        let mut keys = Vec::new();
        other.drain_keys(other.root, &mut keys);
        for k in keys {
            self.nodes.push(Node {
                key: k,
                left: NIL,
                right: NIL,
                rank: 1,
            });
            let leaf = self.nodes.len() - 1;
            self.root = self.merge(self.root, leaf);
        }
        self.len += other.len;
        other.root = NIL;
        other.len = 0;
    }

    fn drain_keys(&self, x: usize, out: &mut Vec<u64>) {
        if x == NIL {
            return;
        }
        out.push(self.nodes[x].key);
        self.drain_keys(self.nodes[x].left, out);
        self.drain_keys(self.nodes[x].right, out);
    }

    /// Push `key`.
    pub fn push(&mut self, key: u64) {
        self.nodes.push(Node {
            key,
            left: NIL,
            right: NIL,
            rank: 1,
        });
        let leaf = self.nodes.len() - 1;
        self.root = self.merge(self.root, leaf);
        self.len += 1;
    }

    /// Smallest key.
    pub fn peek(&self) -> Option<&u64> {
        if self.root == NIL {
            None
        } else {
            Some(&self.nodes[self.root].key)
        }
    }

    /// Remove and return the smallest key.
    pub fn pop(&mut self) -> Option<u64> {
        if self.root == NIL {
            return None;
        }
        let k = self.nodes[self.root].key;
        let (l, r) = (self.nodes[self.root].left, self.nodes[self.root].right);
        self.root = self.merge(l, r);
        self.len -= 1;
        Some(k)
    }

    /// Keys in sorted order — drains a clone, `O(n log n)`.
    pub fn sorted(&self) -> Vec<u64> {
        let mut h = self.clone();
        let mut out = Vec::with_capacity(self.len);
        while let Some(k) = h.pop() {
            out.push(k);
        }
        out
    }

    /// Root rank — the right-spine length bound (`≤ ⌊log₂(n+1)⌋`).
    pub fn right_spine_len(&self) -> u32 {
        self.rank(self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn basics() {
        let mut h = LeftistHeap::new();
        assert!(h.is_empty());
        for k in [9, 3, 7, 1, 1] {
            h.push(k);
        }
        assert_eq!(h.len(), 5);
        assert_eq!(h.peek(), Some(&1));
        assert_eq!(h.pop(), Some(1));
        assert_eq!(h.pop(), Some(1)); // duplicates kept
        assert_eq!(h.pop(), Some(3));
        // sorted drains everything in order
        let h2 = LeftistHeap::from_slice(&[5, 2, 8, 2, 6]);
        assert_eq!(h2.sorted(), vec![2, 2, 5, 6, 8]);
        assert_eq!(h2.len(), 5); // sorted() leaves the heap intact
        assert_eq!(LeftistHeap::from_slice(&[]).pop(), None);
        assert_eq!(LeftistHeap::default().len(), 0);
        // meld
        let mut a = LeftistHeap::from_slice(&[1, 4]);
        let mut b = LeftistHeap::from_slice(&[2, 3]);
        a.meld(&mut b);
        assert_eq!(a.sorted(), vec![1, 2, 3, 4]);
        assert_eq!(b.len(), 0);
        a.meld(&mut b); // empty meld is a no-op
        assert_eq!(a.len(), 4);
    }

    /// Multiset shadow oracle: every op sequence is replayed on
    /// a `BTreeMap<u64,u32>` — plus the leftist structural
    /// invariant `rank(left) ≥ rank(right)` checked every step.
    #[test]
    fn oracle_multiset_shadow() {
        fn check_shape(h: &LeftistHeap, x: usize) {
            if x == !0usize {
                return;
            }
            let (l, r) = (h.nodes[x].left, h.nodes[x].right);
            assert!(h.rank(l) >= h.rank(r), "right spine heavy at {x}");
            assert_eq!(h.nodes[x].rank, h.rank(r) + 1);
            if l != !0usize {
                assert!(h.nodes[l].key >= h.nodes[x].key, "heap order broken");
            }
            if r != !0usize {
                assert!(h.nodes[r].key >= h.nodes[x].key, "heap order broken");
            }
            check_shape(h, l);
            check_shape(h, r);
        }
        let mut rng = SplitMix64::new(17);
        for _round in 0..80 {
            let mut h = if rng.below(2) == 0 {
                LeftistHeap::new()
            } else {
                LeftistHeap::from_slice(
                    &(0..rng.below(30))
                        .map(|_| rng.below(64) as u64)
                        .collect::<Vec<u64>>(),
                )
            };
            let mut want: BTreeMap<u64, u32> = BTreeMap::new();
            // replay construction
            for k in h.sorted() {
                *want.entry(k).or_insert(0) += 1;
            }
            for _ in 0..150 {
                match rng.below(3) {
                    0 | 1 => {
                        let k = rng.below(128) as u64;
                        h.push(k);
                        *want.entry(k).or_insert(0) += 1;
                    }
                    _ => {
                        let got = h.pop();
                        let expected = want.iter().next().map(|(&k, &c)| (k, c));
                        match expected {
                            None => assert_eq!(got, None),
                            Some((k, _)) => {
                                assert_eq!(got, Some(k));
                                let e = want.get_mut(&k).unwrap();
                                *e -= 1;
                                if *e == 0 {
                                    want.remove(&k);
                                }
                            }
                        }
                    }
                }
                assert_eq!(h.len(), want.values().map(|&c| c as usize).sum::<usize>());
                check_shape(&h, h.root);
            }
            // final drain agrees with the multiset
            let mut rest = Vec::new();
            while let Some(k) = h.pop() {
                rest.push(k);
            }
            let want_vec: Vec<u64> = want
                .iter()
                .flat_map(|(&k, &c)| std::iter::repeat(k).take(c as usize))
                .collect();
            assert_eq!(rest, want_vec);
        }
    }

    /// The leftist property's payoff: after any workload the
    /// right spine (the only path merge ever walks) stays
    /// `O(log n)`.
    #[test]
    fn right_spine_is_log() {
        let mut h = LeftistHeap::new();
        let mut rng = SplitMix64::new(7);
        for _ in 0..4096 {
            h.push(rng.next_u64());
        }
        // rank(root) ≤ ⌊log₂(n+1)⌋ = 12 for n = 4096
        assert!(h.right_spine_len() <= 13, "spine {}", h.right_spine_len());
    }
}

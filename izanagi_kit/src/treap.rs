//! Deterministic balanced set — a treap (BST + min-heap over hashed
//! priorities) stored in a flat arena. Each key's priority is
//! `splitmix64(key ^ seed)`, so the tree's shape is a pure function of
//! the key set: inserting the same elements in any order yields the
//! same tree. That makes it the ordered-set backbone for simulation
//! state that must agree across peers — iterating a `HashSet` would
//! leak platform entropy; iterating this yields sorted keys, always.
//!
//! ```
//! use izanagi_kit::treap::Treap;
//! let mut t = Treap::new();
//! for k in [50, 10, 70, 30] { t.insert(k); }
//! assert_eq!(t.iter().collect::<Vec<_>>(), vec![10, 30, 50, 70]);
//! assert_eq!(t.select(2), Some(50));
//! assert_eq!(t.rank(40), 2);
//! ```

use crate::rng::SplitMix64;

#[derive(Clone)]
struct Node {
    key: u64,
    prio: u64,
    left: u32,
    right: u32,
    size: u32,
}

const NIL: u32 = u32::MAX;

/// An ordered `u64` set with O(log n) expected insert/remove/rank.
/// Priorities are hashed, so shape never depends on insertion order.
#[derive(Clone, Default)]
pub struct Treap {
    nodes: Vec<Node>,
    free: Vec<u32>,
    root: u32,
    seed: u64,
}

fn prio_of(key: u64, seed: u64) -> u64 {
    SplitMix64::new(key ^ seed).next_u64()
}

impl Treap {
    /// Empty set with the default seed.
    pub fn new() -> Self {
        Self::with_seed(0)
    }

    /// Empty set with domain-separating `seed` — two seeds hash the
    /// same keys to different priorities, hence different shapes.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            root: NIL,
            seed,
        }
    }

    /// Number of keys stored.
    pub fn len(&self) -> usize {
        self.nodes.len() - self.free.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn size(&self, i: u32) -> u32 {
        if i == NIL {
            0
        } else {
            self.nodes[i as usize].size
        }
    }

    fn pull(&mut self, i: u32) {
        let (l, r) = (self.nodes[i as usize].left, self.nodes[i as usize].right);
        self.nodes[i as usize].size = 1 + self.size(l) + self.size(r);
    }

    fn alloc(&mut self, key: u64) -> u32 {
        let n = Node {
            key,
            prio: prio_of(key, self.seed),
            left: NIL,
            right: NIL,
            size: 1,
        };
        if let Some(i) = self.free.pop() {
            self.nodes[i as usize] = n;
            i
        } else {
            self.nodes.push(n);
            self.nodes.len() as u32 - 1
        }
    }

    /// `merge(l, r)` assumes every key in `l` < every key in `r` and
    /// returns the combined subtree root. Heap ties break on key, so
    /// identical priority pairs still yield one canonical shape.
    fn merge(&mut self, l: u32, r: u32) -> u32 {
        if l == NIL {
            return r;
        }
        if r == NIL {
            return l;
        }
        let (ln, rn) = (&self.nodes[l as usize], &self.nodes[r as usize]);
        // (prio, key) lexicographic — the smaller tuple parents.
        if (ln.prio, ln.key) < (rn.prio, rn.key) {
            let new_r = self.merge(self.nodes[l as usize].right, r);
            self.nodes[l as usize].right = new_r;
            self.pull(l);
            l
        } else {
            let new_l = self.merge(l, self.nodes[r as usize].left);
            self.nodes[r as usize].left = new_l;
            self.pull(r);
            r
        }
    }

    /// Split `t` into `(l, r)` with `l` holding keys `< key`.
    fn split(&mut self, t: u32, key: u64) -> (u32, u32) {
        if t == NIL {
            return (NIL, NIL);
        }
        let tk = self.nodes[t as usize].key;
        if tk < key {
            let (a, b) = self.split(self.nodes[t as usize].right, key);
            self.nodes[t as usize].right = a;
            self.pull(t);
            (t, b)
        } else {
            let (a, b) = self.split(self.nodes[t as usize].left, key);
            self.nodes[t as usize].left = b;
            self.pull(t);
            (a, t)
        }
    }

    /// Insert `key`; returns `false` when already present.
    pub fn insert(&mut self, key: u64) -> bool {
        if self.contains(key) {
            return false;
        }
        let node = self.alloc(key);
        let (l, r) = self.split(self.root, key);
        let joined = self.merge(l, node);
        self.root = self.merge(joined, r);
        true
    }

    /// Remove `key`; returns `false` when absent.
    pub fn remove(&mut self, key: u64) -> bool {
        let (l, rest) = self.split(self.root, key);
        // `mid` = keys in [key, key+1) = exactly `key`, so it is a
        // single node. key == u64::MAX means there is no upper bound.
        let (mid, r) = match key.checked_add(1) {
            Some(hi) => self.split(rest, hi),
            None => (rest, NIL),
        };
        if mid == NIL {
            self.root = self.merge(l, r);
            return false;
        }
        self.free.push(mid);
        self.root = self.merge(l, r);
        true
    }

    /// Whether `key` is present.
    pub fn contains(&self, key: u64) -> bool {
        let mut cur = self.root;
        while cur != NIL {
            let n = &self.nodes[cur as usize];
            if key < n.key {
                cur = n.left;
            } else if key > n.key {
                cur = n.right;
            } else {
                return true;
            }
        }
        false
    }

    /// Smallest key, or `None` when empty.
    pub fn min(&self) -> Option<u64> {
        let mut cur = self.root;
        let mut best = None;
        while cur != NIL {
            let n = &self.nodes[cur as usize];
            best = Some(n.key);
            cur = n.left;
        }
        best
    }

    /// Largest key, or `None` when empty.
    pub fn max(&self) -> Option<u64> {
        let mut cur = self.root;
        let mut best = None;
        while cur != NIL {
            let n = &self.nodes[cur as usize];
            best = Some(n.key);
            cur = n.right;
        }
        best
    }

    /// Number of keys strictly less than `key`.
    pub fn rank(&self, key: u64) -> usize {
        let mut cur = self.root;
        let mut acc = 0u32;
        while cur != NIL {
            let n = &self.nodes[cur as usize];
            if key <= n.key {
                cur = n.left;
            } else {
                acc += 1 + self.size(n.left);
                cur = n.right;
            }
        }
        acc as usize
    }

    /// The `k`-th smallest key (0-based), or `None` when out of range.
    pub fn select(&self, k: usize) -> Option<u64> {
        let mut cur = self.root;
        let mut k = k as u32;
        while cur != NIL {
            let n = &self.nodes[cur as usize];
            let ls = self.size(n.left);
            if k < ls {
                cur = n.left;
            } else if k == ls {
                return Some(n.key);
            } else {
                k -= ls + 1;
                cur = n.right;
            }
        }
        None
    }

    /// In-order iterator — sorted keys.
    pub fn iter(&self) -> Iter<'_> {
        let mut it = Iter {
            nodes: &self.nodes,
            stack: Vec::new(),
            cur: self.root,
        };
        it.descend();
        it
    }
}

/// In-order iterator over a [`Treap`].
pub struct Iter<'a> {
    nodes: &'a [Node],
    stack: Vec<u32>,
    cur: u32,
}

impl Iter<'_> {
    fn descend(&mut self) {
        while self.cur != NIL {
            self.stack.push(self.cur);
            self.cur = self.nodes[self.cur as usize].left;
        }
    }
}

impl Iterator for Iter<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        let i = self.stack.pop()?;
        let key = self.nodes[i as usize].key;
        self.cur = self.nodes[i as usize].right;
        self.descend();
        Some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn insert_remove_match_btree_set() {
        let mut rng = SplitMix64::new(0x7E4F);
        let mut t = Treap::new();
        let mut oracle = BTreeSet::new();
        for _ in 0..4_000 {
            let key = rng.below(500) as u64;
            if rng.below(3) == 0 {
                assert_eq!(t.remove(key), oracle.remove(&key));
            } else {
                assert_eq!(t.insert(key), oracle.insert(key));
            }
            assert_eq!(t.len(), oracle.len());
        }
        assert_eq!(
            t.iter().collect::<Vec<_>>(),
            oracle.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(t.min(), oracle.iter().next().copied());
        assert_eq!(t.max(), oracle.iter().next_back().copied());
        assert!(t.is_empty() == oracle.is_empty());
    }

    #[test]
    fn rank_and_select_match_positions() {
        let mut rng = SplitMix64::new(0x9E3779B9);
        let mut t = Treap::new();
        let mut oracle = BTreeSet::new();
        for _ in 0..1_500 {
            t.insert(rng.next_u64() % 1_000_000);
        }
        oracle.extend(t.iter());
        let sorted: Vec<u64> = oracle.iter().copied().collect();
        for (k, &v) in sorted.iter().enumerate() {
            assert_eq!(t.select(k), Some(v));
            assert_eq!(t.rank(v), k);
            assert_eq!(t.rank(v + 1), k + 1);
        }
        assert_eq!(t.select(sorted.len()), None);
        assert_eq!(t.rank(sorted[0]), 0);
        assert_eq!(t.rank(u64::MAX), sorted.len());
    }

    #[test]
    fn shape_is_independent_of_insertion_order() {
        // Same key set, shuffled insertions → identical (key, depth)
        // in-order sequence, which uniquely reconstructs the tree.
        let keys: Vec<u64> = {
            let mut r = SplitMix64::new(7);
            (0..300).map(|_| r.next_u64() % 10_000).collect()
        };
        let keys: BTreeSet<u64> = keys.into_iter().collect();
        let mut a = Treap::new();
        for &k in &keys {
            a.insert(k);
        }
        let mut b = Treap::new();
        for &k in keys.iter().rev() {
            b.insert(k);
        }
        // in-order (key, depth) fully determines a BST's shape.
        fn depth_sig(t: &Treap) -> Vec<(u64, u32)> {
            fn go(t: &Treap, n: u32, d: u32, out: &mut Vec<(u64, u32)>) {
                if n == NIL {
                    return;
                }
                go(t, t.nodes[n as usize].left, d + 1, out);
                out.push((t.nodes[n as usize].key, d));
                go(t, t.nodes[n as usize].right, d + 1, out);
            }
            let mut out = Vec::new();
            go(t, t.root, 0, &mut out);
            out
        }
        assert_eq!(depth_sig(&a), depth_sig(&b));
        // Heap property: each child's (prio, key) exceeds its parent's.
        fn heap_ok(t: &Treap, n: u32) -> bool {
            if n == NIL {
                return true;
            }
            let node = &t.nodes[n as usize];
            [node.left, node.right].iter().all(|&c| {
                c == NIL
                    || (t.nodes[c as usize].prio, t.nodes[c as usize].key) > (node.prio, node.key)
            }) && heap_ok(t, node.left)
                && heap_ok(t, node.right)
        }
        assert!(heap_ok(&a, a.root));
    }
}

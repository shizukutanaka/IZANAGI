//! PATRICIA / crit-bit tree over `u64` keys — a radix tree where
//! each internal node stores the index of the first bit on which
//! its two subtrees disagree (Bernstein's crit-bit formulation).
//! Depth is bounded by 64 regardless of `n`, every operation is
//! a pure function of the *set* of keys inserted, and in-order
//! traversal yields keys in ascending numeric order — so two
//! peers holding the same keyset walk identical paths.
//!
//! ```
//! use izanagi_kit::patricia::Patricia;
//!
//! let mut t = Patricia::new();
//! assert!(t.insert(10));
//! assert!(t.insert(3));
//! assert!(!t.insert(10));
//! assert!(t.contains(3));
//! assert_eq!(t.iter().copied().collect::<Vec<u64>>(), vec![3, 10]);
//! assert_eq!(t.floor(7), Some(3));
//! assert_eq!(t.ceil(7), Some(10));
//! ```
//!
//! References: Morrison, "PATRICIA" (1968); Bernstein, crit-bit
//! trees (2006); Okasaki & Gill, "Fast mergeable integer maps".

enum Node {
    /// Internal: branch on bit `bit`; `child[0]`/​`child[1]` are
    /// arena indices (0-bit branch first).
    Internal { bit: u32, child: [usize; 2] },
    /// External: a stored key.
    Leaf(u64),
}

/// Ordered set of `u64` keys as a crit-bit radix tree.
pub struct Patricia {
    nodes: Vec<Node>,
    root: Option<usize>,
    len: usize,
}

impl Patricia {
    /// Empty tree.
    pub fn new() -> Patricia {
        Patricia {
            nodes: Vec::new(),
            root: None,
            len: 0,
        }
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Highest set-bit position where `a` and `b` differ, i.e.
    /// `63 - (a^b).leading_zeros()`. Callers ensure `a != b`.
    fn crit_bit(a: u64, b: u64) -> u32 {
        63 - (a ^ b).leading_zeros()
    }

    /// Walks from `start` to the leaf reached by following `key`'s
    /// bits; returns its key.
    fn leaf_for(&self, start: usize, key: u64) -> u64 {
        let mut cur = start;
        loop {
            match self.nodes[cur] {
                Node::Leaf(k) => return k,
                Node::Internal { bit, child } => {
                    cur = child[((key >> bit) & 1) as usize];
                }
            }
        }
    }

    /// `min`/`max` of a subtree: descend to the 0-leaf/1-leaf.
    fn extreme(&self, start: usize, high: bool) -> u64 {
        let mut cur = start;
        loop {
            match self.nodes[cur] {
                Node::Leaf(k) => return k,
                Node::Internal { child, .. } => {
                    cur = child[high as usize];
                }
            }
        }
    }

    /// Inserts `key`; returns `false` when already present.
    pub fn insert(&mut self, key: u64) -> bool {
        let leaf_idx = self.nodes.len();
        self.nodes.push(Node::Leaf(key));
        match self.root {
            None => {
                self.root = Some(leaf_idx);
                self.len += 1;
                true
            }
            Some(root) => {
                let other = self.leaf_for(root, key);
                if other == key {
                    self.nodes.pop();
                    return false;
                }
                let crit = Self::crit_bit(key, other);
                // The new internal node sits above every internal
                // node whose crit bit is *less* significant than
                // `crit`. Descend by `key`'s bits, tracking the
                // parent link, until the split subtree.
                let new_bit_key = ((key >> crit) & 1) as usize;
                let int_idx = self.nodes.len();
                self.nodes.push(Node::Internal {
                    bit: crit,
                    child: [0, 0],
                });
                let mut slot = root;
                let mut parent: Option<(usize, usize)> = None;
                loop {
                    match self.nodes[slot] {
                        Node::Internal { bit, child } if bit > crit => {
                            let dir = ((key >> bit) & 1) as usize;
                            parent = Some((slot, dir));
                            slot = child[dir];
                        }
                        _ => break,
                    }
                }
                // `slot` is the subtree being split: fill the new
                // internal node, then redirect the parent link.
                if let Node::Internal { child, .. } = &mut self.nodes[int_idx] {
                    child[new_bit_key] = leaf_idx;
                    child[1 - new_bit_key] = slot;
                }
                match parent {
                    None => self.root = Some(int_idx),
                    Some((p, dir)) => {
                        if let Node::Internal { child, .. } = &mut self.nodes[p] {
                            child[dir] = int_idx;
                        }
                    }
                }
                self.len += 1;
                true
            }
        }
    }

    /// Membership test.
    pub fn contains(&self, key: u64) -> bool {
        match self.root {
            None => false,
            Some(r) => self.leaf_for(r, key) == key,
        }
    }

    /// Smallest and largest keys.
    pub fn min(&self) -> Option<u64> {
        self.root.map(|r| self.extreme(r, false))
    }

    /// Largest key.
    pub fn max(&self) -> Option<u64> {
        self.root.map(|r| self.extreme(r, true))
    }

    /// Largest key `<= key`. Crit-bit subtrees only constrain the
    /// *tested* bits, so keys in an off-branch can still dip below
    /// the bound via untested higher positions — both children may
    /// hold candidates and each is explored iff its subtree minimum
    /// can satisfy the bound. `strict` makes it `< key`.
    fn max_le(&self, cur: usize, key: u64, strict: bool) -> Option<u64> {
        match &self.nodes[cur] {
            Node::Leaf(k) => {
                let ok = if strict { *k < key } else { *k <= key };
                if ok {
                    Some(*k)
                } else {
                    None
                }
            }
            Node::Internal { bit, child } => {
                let dir = ((key >> bit) & 1) as usize;
                let mut best: Option<u64> = None;
                for &ch in &[child[dir], child[1 - dir]] {
                    let lo = self.extreme(ch, false);
                    if if strict { lo < key } else { lo <= key } {
                        if let Some(v) = self.max_le(ch, key, strict) {
                            best = Some(best.map_or(v, |b| b.max(v)));
                        }
                    }
                }
                best
            }
        }
    }

    /// Smallest key `>= key` (`> key` when `strict`).
    fn min_ge(&self, cur: usize, key: u64, strict: bool) -> Option<u64> {
        match &self.nodes[cur] {
            Node::Leaf(k) => {
                let ok = if strict { *k > key } else { *k >= key };
                if ok {
                    Some(*k)
                } else {
                    None
                }
            }
            Node::Internal { bit, child } => {
                let dir = ((key >> bit) & 1) as usize;
                let mut best: Option<u64> = None;
                for &ch in &[child[dir], child[1 - dir]] {
                    let hi = self.extreme(ch, true);
                    if if strict { hi > key } else { hi >= key } {
                        if let Some(v) = self.min_ge(ch, key, strict) {
                            best = Some(best.map_or(v, |b| b.min(v)));
                        }
                    }
                }
                best
            }
        }
    }

    /// Largest key `<= key`.
    pub fn floor(&self, key: u64) -> Option<u64> {
        self.max_le(self.root?, key, false)
    }

    /// Smallest key `>= key`.
    pub fn ceil(&self, key: u64) -> Option<u64> {
        self.min_ge(self.root?, key, false)
    }

    /// Largest key strictly `< key` (`None` below the minimum).
    pub fn predecessor(&self, key: u64) -> Option<u64> {
        self.max_le(self.root?, key, true)
    }

    /// Smallest key strictly `> key` (`None` above the maximum).
    pub fn successor(&self, key: u64) -> Option<u64> {
        self.min_ge(self.root?, key, true)
    }

    /// Ascending-order iterator over all keys (in-order walk —
    /// 0-subtree then 1-subtree is globally sorted because bits
    /// get less significant with depth).
    pub fn iter(&self) -> Iter<'_> {
        let mut it = Iter {
            t: self,
            stack: Vec::new(),
        };
        if let Some(r) = self.root {
            it.stack.push(r);
        }
        it
    }
}

impl Default for Patricia {
    fn default() -> Patricia {
        Patricia::new()
    }
}

/// In-order iterator over [`Patricia`].
pub struct Iter<'a> {
    t: &'a Patricia,
    stack: Vec<usize>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a u64;

    fn next(&mut self) -> Option<&'a u64> {
        while let Some(cur) = self.stack.pop() {
            match &self.t.nodes[cur] {
                Node::Leaf(k) => return Some(k),
                Node::Internal { child, .. } => {
                    // Push 1-subtree first so 0 pops first.
                    self.stack.push(child[1]);
                    self.stack.push(child[0]);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn basic_ops() {
        let mut t = Patricia::new();
        assert!(t.is_empty());
        assert_eq!(t.floor(5), None);
        assert!(t.insert(5));
        assert!(t.insert(1));
        assert!(t.insert(9));
        assert!(!t.insert(5));
        assert_eq!(t.len(), 3);
        assert_eq!(t.min(), Some(1));
        assert_eq!(t.max(), Some(9));
        assert!(t.contains(9));
        assert!(!t.contains(8));
        assert_eq!(t.iter().copied().collect::<Vec<_>>(), vec![1, 5, 9]);
    }

    #[test]
    fn floor_ceil_pred_succ_edges() {
        let mut t = Patricia::new();
        for &k in &[3u64, 7, 11, 11, u64::MAX, 0] {
            t.insert(k);
        }
        assert_eq!(t.floor(0), Some(0));
        assert_eq!(t.floor(2), Some(0));
        assert_eq!(t.floor(u64::MAX), Some(u64::MAX));
        assert_eq!(t.ceil(0), Some(0));
        assert_eq!(t.ceil(12), Some(u64::MAX));
        assert_eq!(t.predecessor(0), None);
        assert_eq!(t.predecessor(4), Some(3));
        assert_eq!(t.successor(11), Some(u64::MAX));
        assert_eq!(t.successor(u64::MAX), None);
    }

    #[test]
    fn matches_btreeset_oracle() {
        let mut rng = SplitMix64::new(0x9A17);
        for round in 0..150 {
            let mut t = Patricia::new();
            let mut oracle = BTreeSet::new();
            let n = 1 + (rng.next_u64() % 80) as usize;
            for _ in 0..n {
                let k = match rng.next_u64() % 4 {
                    0 => rng.next_u64() % 16,
                    1 => rng.next_u64() % 256,
                    2 => rng.next_u64() >> 32,
                    _ => rng.next_u64(),
                };
                assert_eq!(t.insert(k), oracle.insert(k), "insert {k}");
            }
            assert_eq!(t.len(), oracle.len());
            assert_eq!(t.min(), oracle.iter().next().copied());
            assert_eq!(t.max(), oracle.iter().next_back().copied());
            let got: Vec<u64> = t.iter().copied().collect();
            let want: Vec<u64> = oracle.iter().copied().collect();
            assert_eq!(got, want, "round {round} order");
            for _ in 0..30 {
                let q = match rng.next_u64() % 3 {
                    0 => rng.next_u64() % 300,
                    _ => rng.next_u64(),
                };
                assert_eq!(t.contains(q), oracle.contains(&q), "contains {q}");
                assert_eq!(
                    t.floor(q),
                    oracle.range(..=q).next_back().copied(),
                    "floor {q}"
                );
                assert_eq!(t.ceil(q), oracle.range(q..).next().copied(), "ceil {q}");
            }
        }
    }

    #[test]
    fn insertion_order_shapes_structure_but_not_set() {
        // Different insertion orders may shape internal nodes
        // differently, but the *set operations* agree everywhere.
        let orders: [Vec<u64>; 3] = [
            vec![1, 50, 200, 9, 77],
            vec![200, 77, 9, 50, 1],
            vec![50, 1, 9, 77, 200],
        ];
        let mut floors: Vec<Vec<Option<u64>>> = Vec::new();
        for o in &orders {
            let mut t = Patricia::new();
            for &k in o {
                t.insert(k);
            }
            assert_eq!(
                t.iter().copied().collect::<Vec<_>>(),
                vec![1, 9, 50, 77, 200]
            );
            floors.push((0..=255u64).map(|q| t.floor(q)).collect());
        }
        assert_eq!(floors[0], floors[1]);
        assert_eq!(floors[1], floors[2]);
    }
}

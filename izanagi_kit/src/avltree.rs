//! AVL self-balancing binary search tree over `u64` keys.
//!
//! Each node tracks its subtree height; every insertion and
//! deletion restores `|height(left) - height(right)| <= 1` at
//! every ancestor by LL / RR / LR / RL rotations. Balance is a
//! pure function of the tree shape — no seed, no randomness —
//! so equal key sets in different orders can give different
//! shapes but always within the AVL invariant.
//!
//! The arena is append-only: removed nodes stay in `nodes` but
//! become unreachable (`left`/`right`/`height` freed). `len`
//! counts live keys.
//!
//! ```
//! use izanagi_kit::avltree::AvlTree;
//! let mut t = AvlTree::new();
//! for k in [5u64, 2, 8, 1, 3] { t.insert(k); }
//! assert_eq!(t.values(), vec![1, 2, 3, 5, 8]);
//! assert!(t.remove(2));
//! assert_eq!(t.values(), vec![1, 3, 5, 8]);
//! assert!(t.height() <= 3);
//! ```

/// Arena index of an absent child.
const NIL: usize = !0;

#[derive(Clone)]
struct Node {
    key: u64,
    left: usize,
    right: usize,
    /// Height of this subtree: `max(h(l), h(r)) + 1`, a leaf is 1.
    height: u32,
}

/// An AVL tree over `u64` keys — a set (duplicate inserts are
/// ignored). See module docs for the balance contract.
#[derive(Clone)]
pub struct AvlTree {
    nodes: Vec<Node>,
    root: usize,
    len: usize,
}

impl AvlTree {
    /// Empty tree.
    pub fn new() -> Self {
        AvlTree {
            nodes: Vec::new(),
            root: NIL,
            len: 0,
        }
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Height of the whole tree (empty = 0, leaf = 1).
    /// AVL guarantees `height <= ~1.44 log2(len + 2)`.
    pub fn height(&self) -> u32 {
        self.h(self.root)
    }

    /// `key` present?
    pub fn contains(&self, key: &u64) -> bool {
        let mut x = self.root;
        while x != NIL {
            let k = self.nodes[x].key;
            if *key == k {
                return true;
            }
            x = if *key < k {
                self.nodes[x].left
            } else {
                self.nodes[x].right
            };
        }
        false
    }

    /// In-order key sequence — always ascending.
    pub fn values(&self) -> Vec<u64> {
        let mut out = Vec::with_capacity(self.len);
        self.inorder(self.root, &mut out);
        out
    }

    /// Smallest key.
    pub fn min(&self) -> Option<u64> {
        let mut x = self.root;
        if x == NIL {
            return None;
        }
        while self.nodes[x].left != NIL {
            x = self.nodes[x].left;
        }
        Some(self.nodes[x].key)
    }

    /// Largest key.
    pub fn max(&self) -> Option<u64> {
        let mut x = self.root;
        if x == NIL {
            return None;
        }
        while self.nodes[x].right != NIL {
            x = self.nodes[x].right;
        }
        Some(self.nodes[x].key)
    }

    /// Insert `key`; `true` if it was absent.
    pub fn insert(&mut self, key: u64) -> bool {
        let mut inserted = false;
        self.root = self.insert_at(self.root, key, &mut inserted);
        if inserted {
            self.len += 1;
        }
        inserted
    }

    /// Remove `key`; `true` if it was present.
    pub fn remove(&mut self, key: u64) -> bool {
        let mut removed = false;
        self.root = self.remove_at(self.root, key, &mut removed);
        if removed {
            self.len -= 1;
        }
        removed
    }

    // ---- internals --------------------------------------------------

    fn h(&self, x: usize) -> u32 {
        if x == NIL {
            0
        } else {
            self.nodes[x].height
        }
    }

    fn fix_height(&mut self, x: usize) {
        let lh = self.h(self.nodes[x].left);
        let rh = self.h(self.nodes[x].right);
        self.nodes[x].height = lh.max(rh) + 1;
    }

    /// Balance factor `h(right) - h(left)` at `x` (signed).
    fn bf(&self, x: usize) -> i32 {
        self.h(self.nodes[x].right) as i32 - self.h(self.nodes[x].left) as i32
    }

    /// Right rotation around `x` — left child rises.
    fn rotate_right(&mut self, x: usize) -> usize {
        let l = self.nodes[x].left;
        self.nodes[x].left = self.nodes[l].right;
        self.nodes[l].right = x;
        self.fix_height(x);
        self.fix_height(l);
        l
    }

    /// Left rotation around `x` — right child rises.
    fn rotate_left(&mut self, x: usize) -> usize {
        let r = self.nodes[x].right;
        self.nodes[x].right = self.nodes[r].left;
        self.nodes[r].left = x;
        self.fix_height(x);
        self.fix_height(r);
        r
    }

    /// Restore the AVL invariant at `x`; returns the subtree root.
    fn rebalance(&mut self, x: usize) -> usize {
        self.fix_height(x);
        let bf = self.bf(x);
        if bf == 2 {
            // right heavy
            let r = self.nodes[x].right;
            if self.bf(r) < 0 {
                self.nodes[x].right = self.rotate_right(r);
            }
            return self.rotate_left(x);
        }
        if bf == -2 {
            // left heavy
            let l = self.nodes[x].left;
            if self.bf(l) > 0 {
                self.nodes[x].left = self.rotate_left(l);
            }
            return self.rotate_right(x);
        }
        x
    }

    fn insert_at(&mut self, x: usize, key: u64, inserted: &mut bool) -> usize {
        if x == NIL {
            self.nodes.push(Node {
                key,
                left: NIL,
                right: NIL,
                height: 1,
            });
            *inserted = true;
            return self.nodes.len() - 1;
        }
        if key == self.nodes[x].key {
            return x;
        }
        if key < self.nodes[x].key {
            let l = self.insert_at(self.nodes[x].left, key, inserted);
            self.nodes[x].left = l;
        } else {
            let r = self.insert_at(self.nodes[x].right, key, inserted);
            self.nodes[x].right = r;
        }
        self.rebalance(x)
    }

    fn remove_at(&mut self, x: usize, key: u64, removed: &mut bool) -> usize {
        if x == NIL {
            return NIL;
        }
        if key < self.nodes[x].key {
            let l = self.remove_at(self.nodes[x].left, key, removed);
            self.nodes[x].left = l;
        } else if key > self.nodes[x].key {
            let r = self.remove_at(self.nodes[x].right, key, removed);
            self.nodes[x].right = r;
        } else {
            *removed = true;
            let l = self.nodes[x].left;
            let r = self.nodes[x].right;
            if l == NIL {
                return r;
            }
            if r == NIL {
                return l;
            }
            // two children: splice the in-order successor's key
            let mut s = r;
            while self.nodes[s].left != NIL {
                s = self.nodes[s].left;
            }
            let skey = self.nodes[s].key;
            self.nodes[x].key = skey;
            let mut dummy = false;
            let nr = self.remove_at(r, skey, &mut dummy);
            self.nodes[x].right = nr;
        }
        if *removed {
            self.rebalance(x)
        } else {
            x
        }
    }

    fn inorder(&self, x: usize, out: &mut Vec<u64>) {
        if x == NIL {
            return;
        }
        self.inorder(self.nodes[x].left, out);
        out.push(self.nodes[x].key);
        self.inorder(self.nodes[x].right, out);
    }

    /// Post-order balance/height audit used by tests.
    #[cfg(test)]
    fn check_node(&self, x: usize, lo: Option<u64>, hi: Option<u64>) -> u32 {
        if x == NIL {
            return 0;
        }
        let n = &self.nodes[x];
        if let Some(l) = lo {
            assert!(n.key > l, "key {} <= lo {}", n.key, l);
        }
        if let Some(h) = hi {
            assert!(n.key < h, "key {} >= hi {}", n.key, h);
        }
        let lh = self.check_node(n.left, lo, Some(n.key));
        let rh = self.check_node(n.right, Some(n.key), hi);
        assert!(
            (lh as i32 - rh as i32).abs() <= 1,
            "unbalanced at {}: {} vs {}",
            n.key,
            lh,
            rh
        );
        assert_eq!(n.height, lh.max(rh) + 1, "stale height at {}", n.key);
        n.height
    }

    /// Full structural audit — panics on any invariant break.
    #[cfg(test)]
    fn check(&self) {
        self.check_node(self.root, None, None);
    }
}

impl Default for AvlTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn basics() {
        let mut t = AvlTree::new();
        assert!(t.is_empty());
        for k in [10u64, 20, 30, 40, 50, 25] {
            assert!(t.insert(k));
        }
        assert!(!t.insert(25)); // duplicate ignored
        assert_eq!(t.len(), 6);
        assert!(t.contains(&30));
        assert!(!t.contains(&31));
        assert_eq!(t.min(), Some(10));
        assert_eq!(t.max(), Some(50));
        assert_eq!(t.values(), vec![10, 20, 25, 30, 40, 50]);
        assert!(t.height() <= 3);
        assert!(!t.remove(31));
        assert!(t.remove(30));
        assert_eq!(t.values(), vec![10, 20, 25, 40, 50]);
    }

    /// Sorted insertion is the classic AVL worst case for a
    /// plain BST — height must stay ~log n.
    #[test]
    fn sorted_insert_stays_short() {
        let mut t = AvlTree::new();
        for k in 0..1000u64 {
            t.insert(k);
        }
        t.check();
        assert!(t.height() <= 11, "height {}", t.height()); // ~1.44·log2(1002) ≈ 14.3; rotations keep it ~10-11
        assert_eq!(t.values().len(), 1000);
    }

    /// Shadow a `BTreeSet` through random op sequences; audit
    /// the balance invariant after every op.
    #[test]
    fn oracle_btreeset_shadow() {
        let mut rng = SplitMix64::new(0xA71_7EE);
        for _round in 0..60 {
            let mut t = AvlTree::new();
            let mut oracle = BTreeSet::new();
            for _ in 0..400 {
                let k = rng.below(500) as u64;
                match rng.below(3) {
                    0 | 1 => {
                        assert_eq!(t.insert(k), oracle.insert(k));
                    }
                    _ => {
                        assert_eq!(t.remove(k), oracle.remove(&k));
                    }
                }
                t.check();
                assert_eq!(t.len(), oracle.len());
            }
            assert_eq!(t.values(), oracle.iter().copied().collect::<Vec<u64>>());
        }
    }
}

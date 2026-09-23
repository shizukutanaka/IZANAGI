//! AA tree — Arne Andersson's simplified red-black tree.
//!
//! A red-black tree tracks two booleans per node (a color);
//! an AA tree tracks one integer `level`. The red-black
//! constraint "no two consecutive reds" becomes "only a
//! *right* child may share its parent's level" — every
//! invariant drops to two local rotations:
//!
//! - **skew**: if `level(left) == level(t)`, rotate right
//!   (fixes a left horizontal edge).
//! - **split**: if `level(right(right)) == level(t)`,
//!   rotate left (fixes a pseudo-2-node of three levels).
//!
//! That's the whole rebalancing toolbox — insert is
//! `insert; skew; split`, delete is `delete;` then
//! `decrease_level` + `skew×3` + `split×2` on the unwind.
//!
//! ```text
//! level(node) = 1                     for leaves
//! level(node) = level(child) + 1      if either child is lower
//! invariant: level(left) < level(t) ≤ level(right)
//!            level(right(right)) < level(t)
//! ```
//!
//! Determinism: the tree shape is a pure function of the
//! insertion order — no priorities, no seeds.
//!
//! ```
//! use izanagi_kit::aastree::AaTree;
//! let mut t = AaTree::new();
//! t.insert(5);
//! t.insert(3);
//! t.insert(7);
//! assert!(t.contains(3));
//! assert!(!t.contains(4));
//! assert_eq!(t.iter().collect::<Vec<u64>>(), vec![3, 5, 7]);
//! assert!(t.check().is_empty());
//! ```

/// Arena index for "no node".
const NIL: usize = !0;

#[derive(Clone, Default)]
struct Node {
    key: u64,
    level: u32,
    left: usize,
    right: usize,
}

/// An AA tree over `u64` keys. See module docs.
#[derive(Clone, Default)]
pub struct AaTree {
    nodes: Vec<Node>,
    root: usize,
    len: usize,
    free: Vec<usize>,
}

impl AaTree {
    /// An empty tree.
    pub fn new() -> Self {
        AaTree {
            nodes: Vec::new(),
            root: NIL,
            len: 0,
            free: Vec::new(),
        }
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Allocate a node (reusing removed slots first).
    fn alloc(&mut self, key: u64) -> usize {
        let n = Node {
            key,
            level: 1,
            left: NIL,
            right: NIL,
        };
        if let Some(i) = self.free.pop() {
            self.nodes[i] = n;
            return i;
        }
        self.nodes.push(n);
        self.nodes.len() - 1
    }

    /// Contains-key lookup.
    pub fn contains(&self, key: u64) -> bool {
        let mut x = self.root;
        while x != NIL {
            if key < self.nodes[x].key {
                x = self.nodes[x].left;
            } else if key > self.nodes[x].key {
                x = self.nodes[x].right;
            } else {
                return true;
            }
        }
        false
    }

    /// Smallest key, if any.
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

    /// Largest key, if any.
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

    /// Height of the root — `O(log n)` guaranteed by the
    /// level invariants (`height ≤ 2·log2(len + 1)`).
    pub fn height(&self) -> usize {
        fn h(nodes: &[Node], x: usize) -> usize {
            if x == NIL {
                0
            } else {
                1 + h(nodes, nodes[x].left).max(h(nodes, nodes[x].right))
            }
        }
        h(&self.nodes, self.root)
    }

    /// In-order traversal iterator (sorted order).
    pub fn iter(&self) -> Iter<'_> {
        let mut it = Iter {
            nodes: &self.nodes,
            stack: Vec::new(),
        };
        it.push_left(self.root);
        it
    }

    /// Keys in sorted order.
    pub fn values(&self) -> Vec<u64> {
        self.iter().collect()
    }

    /// Right rotation at `t` — fixes `level(left) == level(t)`
    /// (a left horizontal edge).
    fn skew(&mut self, t: usize) -> usize {
        if t == NIL {
            return NIL;
        }
        let l = self.nodes[t].left;
        if l != NIL && self.nodes[l].level == self.nodes[t].level {
            self.nodes[t].left = self.nodes[l].right;
            self.nodes[l].right = t;
            l
        } else {
            t
        }
    }

    /// Left rotation + level raise at `t` — fixes
    /// `level(right(right)) == level(t)` (a pseudo 2-node).
    fn split(&mut self, t: usize) -> usize {
        if t == NIL {
            return NIL;
        }
        let r = self.nodes[t].right;
        if r != NIL {
            let rr = self.nodes[r].right;
            if rr != NIL && self.nodes[rr].level == self.nodes[t].level {
                self.nodes[t].right = self.nodes[r].left;
                self.nodes[r].left = t;
                self.nodes[r].level += 1;
                return r;
            }
        }
        t
    }

    /// Recursive insert — unwind applies skew then split.
    fn ins(&mut self, t: usize, key: u64) -> usize {
        if t == NIL {
            self.len += 1;
            return self.alloc(key);
        }
        if key < self.nodes[t].key {
            let l = self.ins(self.nodes[t].left, key);
            self.nodes[t].left = l;
        } else if key > self.nodes[t].key {
            let r = self.ins(self.nodes[t].right, key);
            self.nodes[t].right = r;
        } else {
            return t; // already present
        }
        let t = self.skew(t);
        self.split(t)
    }

    /// Insert `key`; returns true if newly added.
    pub fn insert(&mut self, key: u64) -> bool {
        let before = self.len;
        self.root = self.ins(self.root, key);
        self.len > before
    }

    /// Recompute `level(t)` after a delete — when a child
    /// dropped a level, `t` may have to follow. Then the
    /// standard unwind: skew×3, split×2.
    fn fix_after_delete(&mut self, t: usize) -> usize {
        if t == NIL {
            return NIL;
        }
        // decrease_level: if either child's level < level(t)-1,
        // lower t (and possibly its right child)
        let left = self.nodes[t].left;
        let right = self.nodes[t].right;
        let low = [left, right]
            .iter()
            .map(|&c| if c == NIL { 0 } else { self.nodes[c].level })
            .min()
            .unwrap_or(0);
        if low + 1 < self.nodes[t].level {
            self.nodes[t].level = low + 1;
            let r = self.nodes[t].right;
            if r != NIL && self.nodes[r].level > low + 1 {
                self.nodes[r].level = low + 1;
            }
        }
        let t = self.skew(t);
        let r = self.nodes[t].right;
        if r != NIL {
            let nr = self.skew(r);
            self.nodes[t].right = nr;
            let rr = self.nodes[nr].right;
            if rr != NIL {
                let nrr = self.skew(rr);
                self.nodes[nr].right = nrr;
            }
        }
        let t = self.split(t);
        let r = self.nodes[t].right;
        if r != NIL {
            let nr = self.split(r);
            self.nodes[t].right = nr;
        }
        t
    }

    /// Recursive delete — standard BST delete then the
    /// AA unwind on the way back up. Returns
    /// `(new_subtree_root, vacated_slot)`.
    fn rem(&mut self, t: usize, key: u64) -> (usize, usize) {
        if t == NIL {
            return (NIL, NIL);
        }
        let vacated = if key < self.nodes[t].key {
            let (l, v) = self.rem(self.nodes[t].left, key);
            self.nodes[t].left = l;
            v
        } else if key > self.nodes[t].key {
            let (r, v) = self.rem(self.nodes[t].right, key);
            self.nodes[t].right = r;
            v
        } else {
            self.len -= 1;
            if self.nodes[t].left == NIL || self.nodes[t].right == NIL {
                // 0/1 child: the node itself is vacated
                let heir = if self.nodes[t].left == NIL {
                    self.nodes[t].right
                } else {
                    self.nodes[t].left
                };
                return (heir, t);
            }
            // two children: steal the successor's key, then
            // delete that successor node from the right subtree
            let mut s = self.nodes[t].right;
            while self.nodes[s].left != NIL {
                s = self.nodes[s].left;
            }
            self.nodes[t].key = self.nodes[s].key;
            let (r, v) = self.rem(self.nodes[t].right, self.nodes[s].key);
            self.nodes[t].right = r;
            // the recursive rem counted the successor key as
            // deleted — but it moved into `t`, so undo
            self.len += 1;
            v
        };
        (self.fix_after_delete(t), vacated)
    }

    /// Remove `key`; returns true if it was present.
    pub fn remove(&mut self, key: u64) -> bool {
        let (root, vacated) = self.rem(self.root, key);
        self.root = root;
        if vacated != NIL {
            self.free.push(vacated);
            return true;
        }
        false
    }

    /// Audit the AA invariants; a report line is emitted for
    /// every violation found (empty = healthy).
    /// Errors: `bst order`, `left child level >= node`,
    /// `right child level > node`, `right-right level >= node`,
    /// `level != 1 + min child level` (when either child low).
    pub fn check(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let mut prev: Option<u64> = None;
        self.audit(
            self.root,
            -1,
            i128::from(u64::MAX) + 1,
            &mut prev,
            &mut errs,
        );
        let counted = self.iter().count();
        if counted != self.len {
            errs.push(format!("len {}/counted {}", self.len, counted));
        }
        errs
    }

    fn audit(
        &self,
        t: usize,
        lo: i128,
        hi: i128,
        prev: &mut Option<u64>,
        errs: &mut Vec<String>,
    ) -> u32 {
        if t == NIL {
            return 0;
        }
        let n = &self.nodes[t];
        let k = i128::from(n.key);
        if k <= lo || k >= hi {
            errs.push(format!("bst order violated at {}", n.key));
        }
        // in-order: audit left subtree, then this node, then right
        let ll = self.audit(n.left, lo, k, prev, errs);
        if let Some(p) = *prev {
            if p >= n.key {
                errs.push(format!("order: {p} >= {}", n.key));
            }
        }
        *prev = Some(n.key);
        let rl = self.audit(n.right, k, hi, prev, errs);
        // AA invariants (NIL children count as level 0):
        if n.left != NIL && ll >= n.level {
            errs.push(format!("left level {} >= node level {}", ll, n.level));
        }
        if n.right != NIL && rl > n.level {
            errs.push(format!("right level {} > node level {}", rl, n.level));
        }
        if n.right != NIL {
            let rr = self.nodes[n.right].right;
            if rr != NIL && self.nodes[rr].level >= n.level {
                errs.push("right-right level >= node level".to_string());
            }
        }
        // the defining identity: level = 1 + left child's
        // level (NIL counts as 0) — a "red" node is one that
        // shares its level with its parent via a RIGHT edge
        if n.level != ll + 1 {
            errs.push(format!("level {} != left level {} + 1", n.level, ll));
        }
        n.level
    }
}

/// In-order iterator — sorted ascending.
pub struct Iter<'a> {
    nodes: &'a [Node],
    stack: Vec<usize>,
}

impl<'a> Iter<'a> {
    fn push_left(&mut self, mut x: usize) {
        while x != NIL {
            self.stack.push(x);
            x = self.nodes[x].left;
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = u64;
    fn next(&mut self) -> Option<u64> {
        let x = self.stack.pop()?;
        let key = self.nodes[x].key;
        self.push_left(self.nodes[x].right);
        Some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Basics: insert/contains/iter/min/max/remove.
    #[test]
    fn basics() {
        let mut t = AaTree::new();
        assert!(t.is_empty());
        assert_eq!(t.min(), None);
        for k in [5u64, 3, 7, 1, 4, 6, 8] {
            assert!(t.insert(k));
        }
        assert_eq!(t.len(), 7);
        assert!(!t.insert(5));
        assert_eq!(t.len(), 7);
        assert_eq!(t.iter().collect::<Vec<u64>>(), vec![1, 3, 4, 5, 6, 7, 8]);
        assert_eq!(t.min(), Some(1));
        assert_eq!(t.max(), Some(8));
        assert!(t.contains(4));
        assert!(!t.contains(2));
        assert!(t.remove(5));
        assert!(!t.remove(5));
        assert_eq!(t.iter().collect::<Vec<u64>>(), vec![1, 3, 4, 6, 7, 8]);
        assert!(t.check().is_empty(), "{:?}", t.check());
    }

    /// Sorted insertion — the AA unwind must keep the tree
    /// balanced where a plain BST would degenerate.
    #[test]
    fn sorted_stays_balanced() {
        let mut t = AaTree::new();
        for i in 0u64..1000 {
            t.insert(i);
        }
        assert!(t.check().is_empty(), "{:?}", t.check());
        // AA height bound: h <= 2·log2(n+1) → <= 20 here
        assert!(t.height() <= 20, "height {}", t.height());
    }

    /// Shadow oracle: every op's invariants audited against
    /// BTreeSet ground truth — this is what catches
    /// skew/split/decrease-level errors.
    #[test]
    fn oracle_btreeset_shadow() {
        let mut rng = SplitMix64::new(0xAA11);
        for _ in 0..60 {
            let mut t = AaTree::new();
            let mut s = BTreeSet::new();
            for _ in 0..400 {
                let k = rng.below(150) as u64;
                if rng.below(3) == 0 && !s.is_empty() {
                    assert_eq!(t.remove(k), s.remove(&k), "remove {k}");
                } else {
                    assert_eq!(t.insert(k), s.insert(k), "insert {k}");
                }
            }
            for k in 0..150u64 {
                assert_eq!(t.contains(k), s.contains(&k), "contains {k}");
            }
            assert_eq!(t.len(), s.len());
            assert_eq!(
                t.iter().collect::<Vec<u64>>(),
                s.iter().copied().collect::<Vec<u64>>()
            );
            assert_eq!(t.min(), s.first().copied());
            assert_eq!(t.max(), s.last().copied());
            assert!(t.check().is_empty(), "{:?}", t.check());
        }
    }

    /// Adversarial delete orders — sorted desc, then evens
    /// then odds; successor path exercised hard.
    #[test]
    fn adversarial_delete_orders() {
        let mut t = AaTree::new();
        for i in 0u64..200 {
            t.insert(i);
        }
        // evens descending, then odds descending
        for i in (0u64..=198).rev().step_by(2) {
            assert!(t.remove(i), "del {i}");
            assert!(t.check().is_empty(), "after del {i}: {:?}", t.check());
        }
        for i in (1u64..=199).rev().step_by(2) {
            assert!(t.remove(i), "del {i}");
            assert!(t.check().is_empty(), "after del {i}: {:?}", t.check());
        }
        assert!(t.is_empty());
    }

    /// The level invariants of `check` must actually fire on
    /// a corrupted tree — meta-oracle.
    #[test]
    fn check_catches_violations() {
        let mut t = AaTree::new();
        for i in 0u64..32 {
            t.insert(i);
        }
        assert!(t.check().is_empty());
        // corrupt a level manually
        t.nodes[t.root].level = 99;
        assert!(!t.check().is_empty());
    }
}

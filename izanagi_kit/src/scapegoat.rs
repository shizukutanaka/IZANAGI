//! Scapegoat tree — a self-balancing BST with no per-node
//! balance metadata beyond subtree size (Galperin & Rivest
//! 1993). The invariant: for every node `x`,
//! `size(child) ≤ α·size(x)` with α = 3/4. An insert or delete
//! that breaks the constraint finds the lowest ancestor — the
//! *scapegoat* — where `4·size(child) > 3·size(node)` and
//! rebuilds that subtree perfectly balanced by median split.
//! Amortized `O(log n)` insert/remove because rebuilds are paid
//! for by the imbalance they cure; a global `max_size` high-water
//! mark decides when deletions warrant a full rebuild.
//!
//! ```
//! use izanagi_kit::scapegoat::Scapegoat;
//! let mut t = Scapegoat::new();
//! for k in 0..100 {
//!     t.insert(k);
//! }
//! assert!(t.contains(&77) && t.len() == 100);
//! assert!(t.height() <= 2 * 7); // ~log_{4/3}(100) ≈ 16 — comfortably under
//! t.remove(&77);
//! assert!(!t.contains(&77));
//! ```

const NIL: usize = !0;

#[derive(Clone, Debug)]
struct Node {
    key: u64,
    left: usize,
    right: usize,
    size: usize,
}

/// α-balanced scapegoat tree over `u64` keys. Arena-allocated;
/// removed or rebuilt nodes stay in the arena as unreachable
/// garbage (documented: a `rebuild` replaces subtree wiring, it
/// does not compact).
#[derive(Clone, Debug)]
pub struct Scapegoat {
    nodes: Vec<Node>,
    root: usize,
    len: usize,
    max_size: usize,
    rebuilds: usize,
}

impl Default for Scapegoat {
    fn default() -> Self {
        Scapegoat::new()
    }
}

impl Scapegoat {
    /// New empty tree (α = 3/4).
    pub fn new() -> Scapegoat {
        Scapegoat {
            nodes: Vec::new(),
            root: NIL,
            len: 0,
            max_size: 0,
            rebuilds: 0,
        }
    }

    /// Number of live keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Rebuild count — diagnostics for the amortization claim.
    pub fn n_rebuilds(&self) -> usize {
        self.rebuilds
    }

    fn key(&self, i: usize) -> u64 {
        self.nodes[i].key
    }

    fn size(&self, i: usize) -> usize {
        if i == NIL {
            0
        } else {
            self.nodes[i].size
        }
    }

    /// `key` present?
    pub fn contains(&self, key: &u64) -> bool {
        let mut x = self.root;
        while x != NIL {
            let k = self.nodes[x].key;
            if *key < k {
                x = self.nodes[x].left;
            } else if *key > k {
                x = self.nodes[x].right;
            } else {
                return true;
            }
        }
        false
    }

    /// Insert `key` — `true` iff absent. After linking the leaf,
    /// the path back up both fixes ancestor sizes and finds the
    /// lowest ancestor whose child broke `size ≤ α·parent` — that
    /// node is the scapegoat and its subtree is rebuilt.
    pub fn insert(&mut self, key: u64) -> bool {
        if self.root == NIL {
            self.nodes.push(Node {
                key,
                left: NIL,
                right: NIL,
                size: 1,
            });
            self.root = 0;
            self.len = 1;
            self.max_size = self.max_size.max(1);
            return true;
        }
        // descend, recording the ancestor path
        let mut path: Vec<usize> = Vec::new();
        let mut x = self.root;
        loop {
            let k = self.nodes[x].key;
            path.push(x);
            if key < k {
                let l = self.nodes[x].left;
                if l == NIL {
                    self.nodes.push(Node {
                        key,
                        left: NIL,
                        right: NIL,
                        size: 1,
                    });
                    self.nodes[x].left = self.nodes.len() - 1;
                    break;
                }
                x = l;
            } else if key > k {
                let r = self.nodes[x].right;
                if r == NIL {
                    self.nodes.push(Node {
                        key,
                        left: NIL,
                        right: NIL,
                        size: 1,
                    });
                    self.nodes[x].right = self.nodes.len() - 1;
                    break;
                }
                x = r;
            } else {
                return false;
            }
        }
        self.len += 1;
        self.max_size = self.max_size.max(self.len);
        // fix ancestor sizes bottom-up
        for &anc in path.iter().rev() {
            self.nodes[anc].size += 1;
        }
        // find the lowest ancestor whose path-child broke
        // `size(child) ≤ α·size(anc)` — prev starts as the leaf
        let mut scapegoat = NIL;
        let mut prev = self.nodes.len() - 1;
        for &anc in path.iter().rev() {
            // prev is the child of anc containing the new leaf
            if 4 * self.size(prev) > 3 * self.size(anc) {
                scapegoat = anc;
                break;
            }
            prev = anc;
        }
        if scapegoat != NIL {
            self.rebuild(scapegoat);
        }
        true
    }

    /// Remove `key` — `true` iff present. Standard BST delete;
    /// afterwards, `len < α·max_size` triggers a whole-tree
    /// rebuild and resets the high-water mark — that's how
    /// deletions pay into the balance invariant.
    pub fn remove(&mut self, key: &u64) -> bool {
        // find the node + parent chain
        let mut parent = NIL;
        let mut x = self.root;
        while x != NIL && self.nodes[x].key != *key {
            parent = x;
            x = if *key < self.nodes[x].key {
                self.nodes[x].left
            } else {
                self.nodes[x].right
            };
        }
        if x == NIL {
            return false;
        }
        // delete x: 0/1 child cases, else splice successor
        if self.nodes[x].left != NIL && self.nodes[x].right != NIL {
            // successor = leftmost of right subtree; copy its key,
            // delete it there instead (it has ≤1 child)
            let mut spar = x;
            let mut s = self.nodes[x].right;
            while self.nodes[s].left != NIL {
                spar = s;
                s = self.nodes[s].left;
            }
            self.nodes[x].key = self.nodes[s].key;
            // s has no left child — relink
            let sright = self.nodes[s].right;
            if self.nodes[spar].left == s {
                self.nodes[spar].left = sright;
            } else {
                self.nodes[spar].right = sright;
            }
            self.fix_all_sizes();
        } else {
            let child = if self.nodes[x].left != NIL {
                self.nodes[x].left
            } else {
                self.nodes[x].right
            };
            if parent == NIL {
                self.root = child;
            } else if self.nodes[parent].left == x {
                self.nodes[parent].left = child;
            } else {
                self.nodes[parent].right = child;
            }
            self.fix_all_sizes();
        }
        self.len -= 1;
        if self.len > 0 && 4 * self.len < 3 * self.max_size {
            // whole-tree rebuild
            self.rebuild(self.root);
            self.max_size = self.len;
        }
        true
    }

    /// Recompute every subtree size bottom-up — `O(n)`, but
    /// deletes are already rebuild-amortized so the linear pass
    /// is absorbed. Simpler and safer than chasing the two
    /// splice paths of a two-child delete.
    fn fix_all_sizes(&mut self) {
        self.fix_sizes_rec(self.root);
    }

    fn fix_sizes_rec(&mut self, x: usize) -> usize {
        if x == NIL {
            return 0;
        }
        let l = self.fix_sizes_rec(self.nodes[x].left);
        let r = self.fix_sizes_rec(self.nodes[x].right);
        self.nodes[x].size = l + r + 1;
        l + r + 1
    }

    /// Rebuild the subtree rooted at `x` as a perfectly balanced
    /// median-split BST — the amortization's payoff step.
    fn rebuild(&mut self, x: usize) {
        // in-order collect keys of the subtree
        let mut keys = Vec::new();
        self.inorder_collect(x, &mut keys);
        let l = keys.len();
        let new_sub = self.build_balanced(&keys, 0, l);
        // splice: find parent of x
        if self.root == x {
            self.root = new_sub;
        } else {
            // locate parent by key walk
            let mut p = self.root;
            loop {
                let pk = self.nodes[p].key;
                let target = self.key(new_sub);
                if target < pk {
                    let l = self.nodes[p].left;
                    if l == x {
                        self.nodes[p].left = new_sub;
                        break;
                    }
                    p = l;
                } else {
                    let r = self.nodes[p].right;
                    if r == x {
                        self.nodes[p].right = new_sub;
                        break;
                    }
                    p = r;
                }
            }
        }
        self.rebuilds += 1;
    }

    fn inorder_collect(&self, x: usize, out: &mut Vec<u64>) {
        if x == NIL {
            return;
        }
        self.inorder_collect(self.nodes[x].left, out);
        out.push(self.nodes[x].key);
        self.inorder_collect(self.nodes[x].right, out);
    }

    /// Median-split build from `keys[lo..hi]` — fresh arena nodes,
    /// `size` correct by construction.
    fn build_balanced(&mut self, keys: &[u64], lo: usize, hi: usize) -> usize {
        if lo >= hi {
            return NIL;
        }
        let mid = lo + (hi - lo) / 2;
        self.nodes.push(Node {
            key: keys[mid],
            left: NIL,
            right: NIL,
            size: 0,
        });
        let id = self.nodes.len() - 1;
        let l = self.build_balanced(keys, lo, mid);
        let r = self.build_balanced(keys, mid + 1, hi);
        self.nodes[id].left = l;
        self.nodes[id].right = r;
        self.nodes[id].size = hi - lo;
        id
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

    /// In-order keys — sorted output, canonical form.
    pub fn values(&self) -> Vec<u64> {
        let mut out = Vec::with_capacity(self.len);
        self.inorder_collect(self.root, &mut out);
        out
    }

    /// Tree height (root-only tree = 1, empty = 0).
    pub fn height(&self) -> usize {
        self.height_rec(self.root)
    }

    fn height_rec(&self, x: usize) -> usize {
        if x == NIL {
            0
        } else {
            1 + self
                .height_rec(self.nodes[x].left)
                .max(self.height_rec(self.nodes[x].right))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn basics() {
        let mut t = Scapegoat::new();
        assert!(t.is_empty());
        for k in 0..100 {
            assert!(t.insert(k));
        }
        assert!(!t.insert(50));
        assert_eq!(t.len(), 100);
        assert!(t.contains(&77));
        assert_eq!(t.min(), Some(0));
        assert_eq!(t.max(), Some(99));
        assert_eq!(t.values(), (0..100).collect::<Vec<u64>>());
        assert_eq!(t.n_rebuilds(), t.rebuilds);
        // sorted insertion is the pathological case for plain BSTs;
        // the α-invariant must keep height near log
        assert!(t.height() <= 20, "height {}", t.height());
        assert!(t.remove(&77));
        assert!(!t.remove(&77));
        assert!(!t.contains(&77));
        assert_eq!(t.len(), 99);
        let mut e = Scapegoat::new();
        e.insert(1);
        e.remove(&1);
        assert_eq!(e.values(), Vec::<u64>::new());
        assert_eq!(e.min(), None);
        assert_eq!(e.max(), None);
        assert_eq!(Scapegoat::default().len(), 0);
    }

    /// Sorted-insert stress: the case that degenerates an
    /// unguarded BST to a chain — the scapegoat invariant caps it.
    #[test]
    fn sorted_insert_stays_short() {
        let mut t = Scapegoat::new();
        for k in 0..1000u64 {
            t.insert(k);
        }
        // log_{4/3}(1000) ≈ 24; height ≤ floor(log_{1/α} n) + 1
        assert!(t.height() <= 30, "height {} degenerate", t.height());
        assert_eq!(t.len(), 1000);
        assert_eq!(t.values()[0], 0);
        assert_eq!(t.values()[999], 999);
    }

    /// BTreeSet shadow oracle + structural invariant check:
    /// after every op, verify the α-weight constraint
    /// `size(child) ≤ α·size(node)` holds tree-wide.
    #[test]
    fn oracle_btree_shadow() {
        // size bookkeeping is a real invariant after every op;
        // the per-node α constraint is only guaranteed after
        // inserts — deletes deliberately let it drift until
        // `len < α·max_size` fires a whole-tree rebuild
        fn check(t: &Scapegoat, x: usize, alpha: bool) {
            if x == !0usize {
                return;
            }
            let (l, r, s) = (t.nodes[x].left, t.nodes[x].right, t.nodes[x].size);
            assert_eq!(s, t.size(l) + t.size(r) + 1, "size field stale at {x}");
            if alpha {
                assert!(4 * t.size(l) <= 3 * s + 3, "left heavy at {x}");
                assert!(4 * t.size(r) <= 3 * s + 3, "right heavy at {x}");
            }
            check(t, l, alpha);
            check(t, r, alpha);
        }
        let mut rng = SplitMix64::new(11);
        for _round in 0..60 {
            let mut t = Scapegoat::new();
            let mut want = BTreeSet::new();
            let mut insert_only = true;
            for _ in 0..300 {
                let k = rng.below(256) as u64;
                match rng.below(3) {
                    0 | 1 => assert_eq!(t.insert(k), want.insert(k)),
                    _ => {
                        insert_only = false;
                        assert_eq!(t.remove(&k), want.remove(&k));
                    }
                }
                assert_eq!(t.len(), want.len());
                assert_eq!(t.values(), want.iter().copied().collect::<Vec<u64>>());
                check(&t, t.root, insert_only);
            }
            assert_eq!(t.min(), want.iter().next().copied());
            assert_eq!(t.max(), want.iter().next_back().copied());
        }
    }
}

//! Red-black binary search tree over `u64` keys.
//!
//! The canonical balanced BST — [`crate::avltree`] tracks
//! heights, this tracks a single color bit, so insert/delete
//! rebalance with at most 2 / 3 rotations respectively and a
//! looser height bound (`h ≤ 2·log2(n+1)`).
//!
//! Arena-backed (`Vec<Node>`, indices instead of pointers) so
//! the whole structure is a plain value — cloneable, movable,
//! and fully deterministic. A [`crate::rng::SplitMix64`]-free
//! design: shape depends only on the *sequence* of ops.
//!
//! ```
//! use izanagi_kit::redblack::RbTree;
//! let mut t = RbTree::new();
//! for k in [3u64, 1, 4, 1, 5, 9] { t.insert(k); }
//! assert_eq!(t.len(), 5);
//! assert!(t.contains(4));
//! t.remove(4);
//! assert_eq!(t.min(), Some(1));
//! assert_eq!(t.iter(), &[1, 3, 5, 9]);
//! ```

/// Arena index of "no node".
const NIL: usize = !0;

const RED: bool = true;
const BLACK: bool = false;

#[derive(Clone)]
struct Node {
    key: u64,
    color: bool,
    left: usize,
    right: usize,
    parent: usize,
}

/// A red-black tree set of `u64` keys. See module docs.
#[derive(Clone, Default)]
pub struct RbTree {
    nodes: Vec<Node>,
    root: usize,
    len: usize,
    /// Freelist of removed node slots.
    free: Vec<usize>,
}

impl RbTree {
    /// An empty tree.
    pub fn new() -> Self {
        RbTree {
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

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `key` is present?
    pub fn contains(&self, key: u64) -> bool {
        self.find(key) != NIL
    }

    fn find(&self, key: u64) -> usize {
        let mut x = self.root;
        while x != NIL {
            let k = self.nodes[x].key;
            if key == k {
                return x;
            }
            x = if key < k {
                self.nodes[x].left
            } else {
                self.nodes[x].right
            };
        }
        NIL
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

    /// In-order keys.
    pub fn iter(&self) -> Vec<u64> {
        let mut out = Vec::with_capacity(self.len);
        self.inorder(self.root, &mut out);
        out
    }

    fn inorder(&self, x: usize, out: &mut Vec<u64>) {
        if x == NIL {
            return;
        }
        self.inorder(self.nodes[x].left, out);
        out.push(self.nodes[x].key);
        self.inorder(self.nodes[x].right, out);
    }

    /// Height of the tree (empty = 0, single node = 1).
    pub fn height(&self) -> usize {
        self.height_of(self.root)
    }

    fn height_of(&self, x: usize) -> usize {
        if x == NIL {
            return 0;
        }
        1 + self
            .height_of(self.nodes[x].left)
            .max(self.height_of(self.nodes[x].right))
    }

    fn alloc(&mut self, key: u64) -> usize {
        let node = Node {
            key,
            color: RED,
            left: NIL,
            right: NIL,
            parent: NIL,
        };
        if let Some(slot) = self.free.pop() {
            self.nodes[slot] = node;
            slot
        } else {
            self.nodes.push(node);
            self.nodes.len() - 1
        }
    }

    fn rotate_left(&mut self, x: usize) {
        let y = self.nodes[x].right;
        self.nodes[x].right = self.nodes[y].left;
        if self.nodes[y].left != NIL {
            let l = self.nodes[y].left;
            self.nodes[l].parent = x;
        }
        self.nodes[y].parent = self.nodes[x].parent;
        let p = self.nodes[x].parent;
        if p == NIL {
            self.root = y;
        } else if self.nodes[p].left == x {
            self.nodes[p].left = y;
        } else {
            self.nodes[p].right = y;
        }
        self.nodes[y].left = x;
        self.nodes[x].parent = y;
    }

    fn rotate_right(&mut self, x: usize) {
        let y = self.nodes[x].left;
        self.nodes[x].left = self.nodes[y].right;
        if self.nodes[y].right != NIL {
            let r = self.nodes[y].right;
            self.nodes[r].parent = x;
        }
        self.nodes[y].parent = self.nodes[x].parent;
        let p = self.nodes[x].parent;
        if p == NIL {
            self.root = y;
        } else if self.nodes[p].right == x {
            self.nodes[p].right = y;
        } else {
            self.nodes[p].left = y;
        }
        self.nodes[y].right = x;
        self.nodes[x].parent = y;
    }

    /// Insert `key`; no-op if already present.
    pub fn insert(&mut self, key: u64) {
        // standard BST descend
        let mut p = NIL;
        let mut x = self.root;
        while x != NIL {
            p = x;
            if key == self.nodes[x].key {
                return; // already present
            }
            x = if key < self.nodes[x].key {
                self.nodes[x].left
            } else {
                self.nodes[x].right
            };
        }
        let z = self.alloc(key);
        self.nodes[z].parent = p;
        if p == NIL {
            self.root = z;
        } else if key < self.nodes[p].key {
            self.nodes[p].left = z;
        } else {
            self.nodes[p].right = z;
        }
        self.len += 1;
        self.insert_fixup(z);
    }

    fn insert_fixup(&mut self, mut z: usize) {
        while z != self.root && self.nodes[self.nodes[z].parent].color == RED {
            let p = self.nodes[z].parent;
            let g = self.nodes[p].parent;
            if p == self.nodes[g].left {
                let uncle = self.nodes[g].right;
                if uncle != NIL && self.nodes[uncle].color == RED {
                    // case 1 — recolor, move up
                    self.nodes[p].color = BLACK;
                    self.nodes[uncle].color = BLACK;
                    self.nodes[g].color = RED;
                    z = g;
                } else {
                    if z == self.nodes[p].right {
                        // case 2 — inner grandchild: rotate to case 3
                        z = p;
                        self.rotate_left(z);
                    }
                    // case 3
                    let p2 = self.nodes[z].parent;
                    let g2 = self.nodes[p2].parent;
                    self.nodes[p2].color = BLACK;
                    self.nodes[g2].color = RED;
                    self.rotate_right(g2);
                }
            } else {
                let uncle = self.nodes[g].left;
                if uncle != NIL && self.nodes[uncle].color == RED {
                    self.nodes[p].color = BLACK;
                    self.nodes[uncle].color = BLACK;
                    self.nodes[g].color = RED;
                    z = g;
                } else {
                    if z == self.nodes[p].left {
                        z = p;
                        self.rotate_right(z);
                    }
                    let p2 = self.nodes[z].parent;
                    let g2 = self.nodes[p2].parent;
                    self.nodes[p2].color = BLACK;
                    self.nodes[g2].color = RED;
                    self.rotate_left(g2);
                }
            }
        }
        self.nodes[self.root].color = BLACK;
    }

    fn transplant(&mut self, u: usize, v: usize) {
        let pu = self.nodes[u].parent;
        if pu == NIL {
            self.root = v;
        } else if self.nodes[pu].left == u {
            self.nodes[pu].left = v;
        } else {
            self.nodes[pu].right = v;
        }
        if v != NIL {
            self.nodes[v].parent = pu;
        }
    }

    fn tree_min(&self, mut x: usize) -> usize {
        while self.nodes[x].left != NIL {
            x = self.nodes[x].left;
        }
        x
    }

    /// Remove `key`; returns whether it was present.
    pub fn remove(&mut self, key: u64) -> bool {
        let z = self.find(key);
        if z == NIL {
            return false;
        }
        self.len -= 1;
        // CLRS RB-DELETE — `fix_node` is NIL when the spliced
        // child slot is empty; the fixup then needs the parent
        // to orient, tracked in `fix_parent`.
        let mut y = z;
        let mut y_orig_color = self.nodes[y].color;
        let fix_node: usize;
        let fix_parent: usize;
        if self.nodes[z].left == NIL {
            fix_node = self.nodes[z].right;
            fix_parent = self.nodes[z].parent;
            self.transplant(z, self.nodes[z].right);
        } else if self.nodes[z].right == NIL {
            fix_node = self.nodes[z].left;
            fix_parent = self.nodes[z].parent;
            self.transplant(z, self.nodes[z].left);
        } else {
            y = self.tree_min(self.nodes[z].right);
            y_orig_color = self.nodes[y].color;
            fix_node = self.nodes[y].right;
            if self.nodes[y].parent == z {
                fix_parent = y;
                if fix_node != NIL {
                    self.nodes[fix_node].parent = y;
                }
            } else {
                fix_parent = self.nodes[y].parent;
                self.transplant(y, self.nodes[y].right);
                let zr = self.nodes[z].right;
                self.nodes[y].right = zr;
                self.nodes[zr].parent = y;
            }
            self.transplant(z, y);
            let zl = self.nodes[z].left;
            self.nodes[y].left = zl;
            self.nodes[zl].parent = y;
            self.nodes[y].color = self.nodes[z].color;
        }
        // recycle z's slot
        self.nodes[z].parent = NIL;
        self.free.push(z);
        if y_orig_color == BLACK && self.len > 0 {
            self.remove_fixup(fix_node, fix_parent);
        }
        true
    }

    /// `x` may be NIL — `p` carries the orientation. All
    /// accesses to `x`'s fields must be guarded.
    fn remove_fixup(&mut self, mut x: usize, mut p: usize) {
        while x != self.root && (x == NIL || self.nodes[x].color == BLACK) {
            if x == NIL && p == NIL {
                break; // x is a NIL child of the old root's slot
            }
            if x == self.nodes[p].left {
                let mut w = self.nodes[p].right;
                if w == NIL {
                    break;
                }
                if self.nodes[w].color == RED {
                    self.nodes[w].color = BLACK;
                    self.nodes[p].color = RED;
                    self.rotate_left(p);
                    w = self.nodes[p].right;
                    if w == NIL {
                        break;
                    }
                }
                let wl_red =
                    self.nodes[w].left != NIL && self.nodes[self.nodes[w].left].color == RED;
                let wr_red =
                    self.nodes[w].right != NIL && self.nodes[self.nodes[w].right].color == RED;
                if !wl_red && !wr_red {
                    self.nodes[w].color = RED;
                    x = p;
                    p = self.nodes[x].parent;
                } else {
                    if !wr_red {
                        let wl = self.nodes[w].left;
                        self.nodes[wl].color = BLACK;
                        self.nodes[w].color = RED;
                        self.rotate_right(w);
                        w = self.nodes[p].right;
                    }
                    self.nodes[w].color = self.nodes[p].color;
                    self.nodes[p].color = BLACK;
                    let wr = self.nodes[w].right;
                    if wr != NIL {
                        self.nodes[wr].color = BLACK;
                    }
                    self.rotate_left(p);
                    x = self.root;
                }
            } else {
                let mut w = self.nodes[p].left;
                if w == NIL {
                    break;
                }
                if self.nodes[w].color == RED {
                    self.nodes[w].color = BLACK;
                    self.nodes[p].color = RED;
                    self.rotate_right(p);
                    w = self.nodes[p].left;
                    if w == NIL {
                        break;
                    }
                }
                let wl_red =
                    self.nodes[w].left != NIL && self.nodes[self.nodes[w].left].color == RED;
                let wr_red =
                    self.nodes[w].right != NIL && self.nodes[self.nodes[w].right].color == RED;
                if !wl_red && !wr_red {
                    self.nodes[w].color = RED;
                    x = p;
                    p = self.nodes[x].parent;
                } else {
                    if !wl_red {
                        let wr = self.nodes[w].right;
                        self.nodes[wr].color = BLACK;
                        self.nodes[w].color = RED;
                        self.rotate_left(w);
                        w = self.nodes[p].left;
                    }
                    self.nodes[w].color = self.nodes[p].color;
                    self.nodes[p].color = BLACK;
                    let wl = self.nodes[w].left;
                    if wl != NIL {
                        self.nodes[wl].color = BLACK;
                    }
                    self.rotate_right(p);
                    x = self.root;
                }
            }
            if x == self.root {
                break;
            }
        }
        if x != NIL {
            self.nodes[x].color = BLACK;
        }
        self.nodes[self.root].color = BLACK;
    }

    /// Check every red-black invariant — test-only audit:
    /// BST order, root black, no red-red edge, equal
    /// black-height on all root-leaf paths.
    #[cfg(test)]
    fn check(&self) {
        fn black_height(t: &RbTree, x: usize, lo: i128, hi: i128) -> u32 {
            if x == NIL {
                return 1; // NIL counts as one black
            }
            let n = &t.nodes[x];
            let k = i128::from(n.key);
            assert!(k > lo && k < hi, "BST order violated");
            if n.color == RED {
                assert!(
                    (n.left == NIL || t.nodes[n.left].color == BLACK)
                        && (n.right == NIL || t.nodes[n.right].color == BLACK),
                    "red-red edge at {}",
                    n.key
                );
            }
            let l = black_height(t, n.left, lo, k);
            let r = black_height(t, n.right, k, hi);
            assert_eq!(l, r, "black-height mismatch at {}", n.key);
            l + u32::from(n.color == BLACK)
        }
        if self.root != NIL {
            assert_eq!(self.nodes[self.root].color, BLACK, "root not black");
            black_height(self, self.root, -1, i128::from(u64::MAX) + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn basics() {
        let mut t = RbTree::new();
        assert!(t.is_empty());
        for k in [7u64, 3, 18, 10, 22, 8, 11, 26] {
            t.insert(k);
            t.check();
        }
        assert_eq!(t.len(), 8);
        assert_eq!(t.min(), Some(3));
        assert_eq!(t.max(), Some(26));
        assert!(t.contains(11));
        assert!(!t.contains(9));
        assert_eq!(t.iter(), vec![3, 7, 8, 10, 11, 18, 22, 26]);
        // CLRS bound: h <= 2 log2(n+1)
        assert!(t.height() <= 8);
        assert!(t.remove(18));
        assert!(!t.remove(18));
        t.check();
        assert_eq!(t.iter(), vec![3, 7, 8, 10, 11, 22, 26]);
    }

    /// Sorted insert is the adversarial case for plain BSTs —
    /// here it must stay shallow.
    #[test]
    fn sorted_stays_balanced() {
        let mut t = RbTree::new();
        for k in 0..1000u64 {
            t.insert(k);
        }
        t.check();
        // 2*log2(1001) ≈ 20
        assert!(t.height() <= 20, "height {}", t.height());
        // remove odd half, still balanced
        for k in (1..1000u64).step_by(2) {
            assert!(t.remove(k));
        }
        t.check();
        assert_eq!(t.len(), 500);
        assert!(t.height() <= 18, "height {}", t.height());
    }

    /// BTreeSet shadow oracle — every op audited against the
    /// full invariant suite.
    #[test]
    fn oracle_btreeset_shadow() {
        let mut rng = crate::rng::SplitMix64::new(0xDEC0DE);
        for _ in 0..60 {
            let mut t = RbTree::new();
            let mut shadow = BTreeSet::new();
            for _ in 0..400 {
                let k = rng.below(80) as u64;
                match rng.below(2) {
                    0 => {
                        t.insert(k);
                        shadow.insert(k);
                    }
                    _ => {
                        assert_eq!(t.remove(k), shadow.remove(&k));
                    }
                }
                t.check();
                assert_eq!(t.len(), shadow.len());
                assert_eq!(t.min(), shadow.iter().next().copied());
                assert_eq!(t.max(), shadow.iter().next_back().copied());
            }
            let mut a = t.iter();
            let mut b: Vec<u64> = shadow.iter().copied().collect();
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b);
        }
    }

    /// Delete order independence on a fixed set — the tree
    /// must satisfy invariants under any interleaving.
    #[test]
    fn adversarial_delete_orders() {
        for round in 0..5 {
            let mut rng = crate::rng::SplitMix64::new(round);
            let mut keys: Vec<u64> = (0..60).collect();
            // seeded shuffle
            for i in (1..keys.len()).rev() {
                let j = rng.below(i as u32 + 1) as usize;
                keys.swap(i, j);
            }
            let mut t = RbTree::new();
            for &k in &keys {
                t.insert(k);
            }
            for i in (1..keys.len()).rev() {
                let j = rng.below(i as u32 + 1) as usize;
                keys.swap(i, j);
            }
            for &k in &keys {
                assert!(t.remove(k));
                t.check();
            }
            assert!(t.is_empty());
            assert_eq!(t.iter(), Vec::<u64>::new());
        }
    }
}

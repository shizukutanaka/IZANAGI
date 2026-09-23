//! Splay trees — Sleator & Tarjan's self-adjusting BST, where
//! every access rotates the found node to the root. Recently
//! touched keys stay near the top, so skewed workloads get
//! `O(log n)` amortized without any balancing bookkeeping — no
//! colors, heights, or weights, just the splay itself. `treap`
//! keeps expected balance via seeded priorities; a splay keeps
//! amortized balance via locality, and its shape is a pure
//! function of the operation sequence (no RNG at all).
//!
//! ```
//! use izanagi_kit::splay::Splay;
//! let mut t = Splay::new();
//! t.insert(5);
//! t.insert(3);
//! t.insert(8);
//! assert!(t.contains(3));
//! assert_eq!(t.to_vec(), vec![3, 5, 8]);
//! assert!(t.remove(5));
//! assert_eq!(t.to_vec(), vec![3, 8]);
//! ```
//!
//! References: Sleator & Tarjan (1985) "Self-adjusting binary
//! search trees"; the bottom-up `insert`-then-splay form used by
//! Sedgewick.

const NIL: u32 = !0;

/// A bottom-up splay tree over `u64` keys.
#[derive(Clone, Debug)]
pub struct Splay {
    root: u32,
    /// `(left, right, parent)` per node slot.
    ch: Vec<[u32; 2]>,
    p: Vec<u32>,
    key: Vec<u64>,
    /// Recycled slots keep the arena compact deterministically.
    free: Vec<u32>,
    len: usize,
}

impl Splay {
    /// Empty tree. (`Default` is *not* derived: `0` is a live
    /// slot index, not `NIL`, so `root` must be initialized
    /// explicitly.)
    pub fn new() -> Self {
        Self {
            root: NIL,
            ch: Vec::new(),
            p: Vec::new(),
            key: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Is the tree empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn alloc(&mut self, k: u64) -> u32 {
        if let Some(i) = self.free.pop() {
            self.ch[i as usize] = [NIL, NIL];
            self.p[i as usize] = NIL;
            self.key[i as usize] = k;
            i
        } else {
            self.ch.push([NIL, NIL]);
            self.p.push(NIL);
            self.key.push(k);
            (self.key.len() - 1) as u32
        }
    }

    fn child(&self, x: u32, dir: usize) -> u32 {
        if x == NIL {
            NIL
        } else {
            self.ch[x as usize][dir]
        }
    }

    fn set_child(&mut self, x: u32, dir: usize, c: u32) {
        if x != NIL {
            self.ch[x as usize][dir] = c;
        }
        if c != NIL {
            self.p[c as usize] = x;
        }
    }

    /// Rotate `x` up over its parent.
    fn rotate(&mut self, x: u32) {
        let p = self.p[x as usize];
        let g = self.p[p as usize];
        let dir = (self.ch[p as usize][1] == x) as usize;
        let b = self.child(x, dir ^ 1);
        self.set_child(x, dir ^ 1, p);
        self.set_child(p, dir, b);
        self.p[x as usize] = g;
        if g != NIL {
            if self.ch[g as usize][0] == p {
                self.ch[g as usize][0] = x;
            } else {
                self.ch[g as usize][1] = x;
            }
        }
    }

    /// Splay `x` to the root.
    fn splay(&mut self, x: u32) {
        if x == NIL {
            return;
        }
        while self.p[x as usize] != NIL {
            let p = self.p[x as usize];
            let g = self.p[p as usize];
            if g == NIL {
                self.rotate(x); // zig
            } else {
                let same = (self.ch[p as usize][1] == x) == (self.ch[g as usize][1] == p);
                if same {
                    self.rotate(p); // zig-zig: parent first
                } else {
                    self.rotate(x); // zig-zag: x twice
                }
                self.rotate(x);
            }
        }
        self.root = x;
    }

    /// Find `k`; splays the found node — or its would-be parent —
    /// to the root. Returns the node slot or `NIL`.
    fn find(&mut self, k: u64) -> Option<u32> {
        let mut x = self.root;
        let mut last = NIL;
        while x != NIL {
            last = x;
            let key = self.key[x as usize];
            if k == key {
                self.splay(x);
                return Some(x);
            }
            x = self.child(x, (k > key) as usize);
        }
        if last != NIL {
            self.splay(last);
        }
        None
    }

    /// Membership test (splays the answer's neighborhood).
    pub fn contains(&mut self, k: u64) -> bool {
        self.find(k).is_some()
    }

    /// Insert `k`; false when already present.
    pub fn insert(&mut self, k: u64) -> bool {
        if self.root == NIL {
            self.root = self.alloc(k);
            self.len = 1;
            return true;
        }
        let mut x = self.root;
        loop {
            let key = self.key[x as usize];
            if k == key {
                self.splay(x);
                return false;
            }
            let dir = (k > key) as usize;
            let c = self.child(x, dir);
            if c == NIL {
                let n = self.alloc(k);
                self.set_child(x, dir, n);
                self.splay(n);
                self.len += 1;
                return true;
            }
            x = c;
        }
    }

    /// Remove `k`; false when absent.
    pub fn remove(&mut self, k: u64) -> bool {
        let Some(x) = self.find(k) else {
            return false;
        };
        // x is at the root; join its two subtrees
        let (l, r) = (self.child(x, 0), self.child(x, 1));
        if l != NIL {
            self.p[l as usize] = NIL;
        }
        if r != NIL {
            self.p[r as usize] = NIL;
        }
        self.free.push(x);
        self.len -= 1;
        self.root = if l == NIL {
            r
        } else {
            // splay l's max to the subtree's root, then graft r
            // under it (m's right child is empty by maximality)
            let m = self.subtree_min_max(l, true);
            self.root = l;
            self.splay(m);
            self.set_child(m, 1, r);
            m
        };
        true
    }

    fn subtree_min_max(&self, mut x: u32, max: bool) -> u32 {
        let dir = max as usize;
        while self.child(x, dir) != NIL {
            x = self.child(x, dir);
        }
        x
    }

    /// Smallest key.
    pub fn min(&mut self) -> Option<u64> {
        let m = self.subtree_min_max(self.root, false);
        if m == NIL {
            None
        } else {
            self.splay(m);
            Some(self.key[m as usize])
        }
    }

    /// Largest key.
    pub fn max(&mut self) -> Option<u64> {
        let m = self.subtree_min_max(self.root, true);
        if m == NIL {
            None
        } else {
            self.splay(m);
            Some(self.key[m as usize])
        }
    }

    /// Keys in sorted order (the BST invariant's canonical form).
    pub fn to_vec(&self) -> Vec<u64> {
        let mut out = Vec::with_capacity(self.len);
        let mut stack = Vec::new();
        let mut x = self.root;
        while x != NIL || !stack.is_empty() {
            while x != NIL {
                stack.push(x);
                x = self.child(x, 0);
            }
            x = stack.pop().unwrap_or(NIL);
            if x == NIL {
                break;
            }
            out.push(self.key[x as usize]);
            x = self.child(x, 1);
        }
        out
    }
}

impl Default for Splay {
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
        let mut t = Splay::new();
        assert!(t.is_empty());
        assert!(t.insert(5));
        assert!(t.insert(3));
        assert!(!t.insert(3));
        assert!(t.insert(8));
        assert_eq!(t.to_vec(), vec![3, 5, 8]);
        assert_eq!(t.min(), Some(3));
        assert_eq!(t.max(), Some(8));
        assert!(t.remove(5));
        assert!(!t.remove(5));
        assert_eq!(t.to_vec(), vec![3, 8]);
        assert!(t.contains(8));
        assert!(!t.contains(5));
    }

    #[test]
    fn splaying_brings_accessed_key_to_root() {
        let mut t = Splay::new();
        for k in [10, 20, 30, 40] {
            t.insert(k);
        }
        assert!(t.contains(10));
        assert_eq!(t.key[t.root as usize], 10);
    }

    #[test]
    fn oracle_random_ops() {
        let mut r = SplitMix64::new(0x9A17);
        for _round in 0..30 {
            let mut t = Splay::new();
            let mut oracle = BTreeSet::new();
            for _ in 0..3000 {
                let k = r.next_u64() % 200;
                match r.next_u64() % 4 {
                    0 => assert_eq!(t.insert(k), oracle.insert(k)),
                    1 => assert_eq!(t.remove(k), oracle.remove(&k)),
                    2 => assert_eq!(t.contains(k), oracle.contains(&k)),
                    _ => {
                        assert_eq!(
                            t.min().or(t.max()),
                            oracle
                                .iter()
                                .next()
                                .copied()
                                .or(oracle.iter().next_back().copied())
                        );
                        assert_eq!(t.min(), oracle.iter().next().copied());
                        assert_eq!(t.max(), oracle.iter().next_back().copied());
                    }
                }
            }
            assert_eq!(t.to_vec(), oracle.iter().copied().collect::<Vec<_>>());
        }
    }

    #[test]
    fn worst_case_insert_order_stays_correct() {
        // sorted inserts build a chain; splays must keep it valid
        let mut t = Splay::new();
        for k in 0..500u64 {
            assert!(t.insert(k));
        }
        for k in 0..500u64 {
            assert!(t.contains(k), "missing {k}");
        }
        for k in (0..500u64).rev() {
            assert!(t.remove(k), "couldn't remove {k}");
        }
        assert!(t.is_empty());
    }

    #[test]
    fn deterministic_shape() {
        let ops = [5u64, 3, 8, 1, 4, 7];
        let build = |ops: &[u64]| {
            let mut t = Splay::new();
            for &k in ops {
                t.insert(k);
            }
            for &k in &ops[0..3] {
                t.contains(k);
            }
            (t.root, t.key.clone(), t.ch.clone(), t.p.clone())
        };
        assert_eq!(build(&ops), build(&ops));
    }
}

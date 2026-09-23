//! Order-statistic tree — a treap with subtree sizes answering
//! `select(k)` (the k-th smallest key) and `rank(k)` (count of keys
//! below `k`) in `O(log n)` expected time alongside plain
//! `insert`/`erase`/`contains`.
//!
//! Determinism: node priority is `splitmix64(key ^ SEED)` — a pure
//! function of the key, so the tree shape depends only on the *set*
//! of keys inserted, never on order, timing, or an RNG. Same set in
//! any order yields the same tree; `rank`/`select` are pure lookups
//! off the deterministic shape.
//!
//! ```
//! use izanagi_kit::ost::Ost;
//! let mut t = Ost::new(0xBEEF);
//! for k in [40u64, 10, 30, 20] {
//!     t.insert(k);
//! }
//! assert_eq!(t.select(0), Some(10));
//! assert_eq!(t.select(3), Some(40));
//! assert_eq!(t.rank(25), 2); // {10, 20} below 25
//! assert_eq!(t.len(), 4);
//! ```
use crate::rng::SplitMix64;

const ROOT_SEED: u64 = 0x0517_5EED_9E77;

/// A deterministic order-statistic tree over `u64` keys (set
/// semantics — inserting an existing key is a no-op returning `false`).
#[derive(Default)]
pub struct Ost {
    root: Option<Box<Node>>,
    seed: u64,
    len: usize,
}

struct Node {
    key: u64,
    prio: u64,
    size: usize,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

fn size(n: &Option<Box<Node>>) -> usize {
    n.as_ref().map_or(0, |n| n.size)
}

fn pull(n: &mut Box<Node>) {
    n.size = 1 + size(&n.left) + size(&n.right);
}

fn rot_right(mut p: Box<Node>) -> Box<Node> {
    let Some(mut c) = p.left.take() else {
        return p;
    };
    p.left = c.right.take();
    pull(&mut p);
    c.right = Some(p);
    pull(&mut c);
    c
}

fn rot_left(mut p: Box<Node>) -> Box<Node> {
    let Some(mut c) = p.right.take() else {
        return p;
    };
    p.right = c.left.take();
    pull(&mut p);
    c.left = Some(p);
    pull(&mut c);
    c
}

impl Ost {
    /// Empty set; `seed` perturbs the key→priority hash so different
    /// logical sets can carry different shapes (still deterministic
    /// per `(seed, key set)`).
    pub fn new(seed: u64) -> Ost {
        Ost {
            root: None,
            seed,
            len: 0,
        }
    }

    fn prio(&self, key: u64) -> u64 {
        let mut r =
            SplitMix64::new(ROOT_SEED ^ self.seed ^ key.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        r.next_u64()
    }

    /// `true` if `key` was absent (now inserted).
    pub fn insert(&mut self, key: u64) -> bool {
        let prio = self.prio(key);
        let (n, added) = ins(self.root.take(), key, prio);
        self.root = n;
        self.len += usize::from(added);
        added
    }

    /// `true` if `key` was present (now removed).
    pub fn erase(&mut self, key: u64) -> bool {
        let (n, gone) = del(self.root.take(), key);
        self.root = n;
        self.len -= usize::from(gone);
        gone
    }

    /// Membership test.
    pub fn contains(&self, key: u64) -> bool {
        self.rank(key + 1) != self.rank(key)
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` iff empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The k-th smallest key, `None` when `k >= len`.
    pub fn select(&self, k: usize) -> Option<u64> {
        if k >= self.len {
            return None;
        }
        let mut n = self.root.as_deref()?;
        let mut k = k;
        loop {
            let ls = n.left.as_ref().map_or(0, |l| l.size);
            if k < ls {
                n = n.left.as_deref()?;
            } else if k == ls {
                return Some(n.key);
            } else {
                k -= ls + 1;
                n = n.right.as_deref()?;
            }
        }
    }

    /// Count of keys `< key`.
    pub fn rank(&self, key: u64) -> usize {
        let mut r = 0usize;
        let mut n = self.root.as_deref();
        while let Some(node) = n {
            if key <= node.key {
                n = node.left.as_deref();
            } else {
                r += 1 + node.left.as_ref().map_or(0, |l| l.size);
                n = node.right.as_deref();
            }
        }
        r
    }

    /// Smallest key `>= key` (successor-or-self), `None` past the max.
    pub fn lower_bound(&self, key: u64) -> Option<u64> {
        let mut n = self.root.as_deref();
        let mut best = None;
        while let Some(node) = n {
            if node.key >= key {
                best = Some(node.key);
                n = node.left.as_deref();
            } else {
                n = node.right.as_deref();
            }
        }
        best
    }
}

fn ins(n: Option<Box<Node>>, key: u64, prio: u64) -> (Option<Box<Node>>, bool) {
    match n {
        None => (
            Some(Box::new(Node {
                key,
                prio,
                size: 1,
                left: None,
                right: None,
            })),
            true,
        ),
        Some(mut node) => {
            if key < node.key {
                let (l, a) = ins(node.left.take(), key, prio);
                node.left = l;
                pull(&mut node);
                if a && node.left.as_ref().is_some_and(|l| l.prio < node.prio) {
                    node = rot_right(node);
                }
                (Some(node), a)
            } else if key > node.key {
                let (r, a) = ins(node.right.take(), key, prio);
                node.right = r;
                pull(&mut node);
                if a && node.right.as_ref().is_some_and(|r| r.prio < node.prio) {
                    node = rot_left(node);
                }
                (Some(node), a)
            } else {
                (Some(node), false)
            }
        }
    }
}

fn del(n: Option<Box<Node>>, key: u64) -> (Option<Box<Node>>, bool) {
    match n {
        None => (None, false),
        Some(mut node) => {
            if key < node.key {
                let (l, g) = del(node.left.take(), key);
                node.left = l;
                pull(&mut node);
                (Some(node), g)
            } else if key > node.key {
                let (r, g) = del(node.right.take(), key);
                node.right = r;
                pull(&mut node);
                (Some(node), g)
            } else {
                // Two children → rotate the higher-priority child up,
                // recur the node down the other side.
                let lp = node.left.as_ref().map_or(u64::MAX, |l| l.prio);
                let rp = node.right.as_ref().map_or(u64::MAX, |r| r.prio);
                if lp == u64::MAX && rp == u64::MAX {
                    return (None, true);
                }
                if lp < rp {
                    node = rot_right(node);
                    let (r, g) = del(node.right.take(), key);
                    node.right = r;
                    pull(&mut node);
                    (Some(node), g)
                } else {
                    node = rot_left(node);
                    let (l, g) = del(node.left.take(), key);
                    node.left = l;
                    pull(&mut node);
                    (Some(node), g)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn check_invariant(n: &Option<Box<Node>>) -> (usize, Vec<u64>) {
        match n {
            None => (0, Vec::new()),
            Some(node) => {
                let (ls, mut lv) = check_invariant(&node.left);
                let (rs, rv) = check_invariant(&node.right);
                assert_eq!(node.size, 1 + ls + rs);
                // Heap invariant: parent priority strictly smaller (min-heap).
                if let Some(l) = &node.left {
                    assert!(l.prio > node.prio);
                }
                if let Some(r) = &node.right {
                    assert!(r.prio > node.prio);
                }
                lv.push(node.key);
                lv.extend(rv);
                (node.size, lv)
            }
        }
    }

    #[test]
    fn basics() {
        let mut t = Ost::new(1);
        assert!(t.is_empty());
        assert!(t.insert(5));
        assert!(!t.insert(5));
        assert!(t.contains(5));
        assert!(!t.contains(6));
        assert_eq!(t.select(0), Some(5));
        assert_eq!(t.select(1), None);
        assert!(t.erase(5));
        assert!(!t.erase(5));
        assert!(t.is_empty());
        assert_eq!(t.lower_bound(0), None);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0x0517_ca55_ba9e);
        for round in 0..60 {
            let mut t = Ost::new(round);
            let mut want = BTreeSet::<u64>::new();
            for _ in 0..300 {
                let k = u64::from(rng.below(80));
                match rng.below(5) {
                    0..=2 => {
                        assert_eq!(t.insert(k), want.insert(k));
                    }
                    _ => {
                        assert_eq!(t.erase(k), want.remove(&k));
                    }
                }
                assert_eq!(t.len(), want.len());
            }
            // Full rank/select consistency against the sorted set.
            let v: Vec<u64> = want.iter().copied().collect();
            let (sz, order) = check_invariant(&t.root);
            assert_eq!(sz, want.len());
            assert_eq!(order, v);
            for (i, &key) in v.iter().enumerate() {
                assert_eq!(t.select(i), Some(key));
                assert_eq!(t.rank(key), i);
                assert_eq!(t.lower_bound(key), Some(key));
            }
            assert_eq!(t.select(v.len()), None);
            for k in 0..=80u64 {
                let expect = want.range(..k).count();
                assert_eq!(t.rank(k), expect);
                let lb = want.range(k..).next().copied();
                assert_eq!(t.lower_bound(k), lb);
            }
        }
    }
}

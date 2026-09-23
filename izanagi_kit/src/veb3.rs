//! Van Emde Boas tree — a recursively-split predecessor
//! structure over `u64` keys. Unlike [`crate::veb`]'s two-level
//! proto form, this is the true van Emde Boas layout: each node
//! caches `min`/`max` outside its clusters, so `min`, `max`,
//! `successor`, `predecessor`, `insert`, `remove` all descend
//! `O(log log U)` levels — for 64-bit keys the tree is 12
//! levels deep and each operation touches O(1) nodes per level.
//!
//! Determinism: clusters live in a `BTreeMap`, so the whole
//! structure is a pure function of the operation sequence —
//! no hashing, no addresses. Same conventions as `veb`/`yfast`:
//! `successor`/`predecessor` are strict, `min`/`max` inclusive.
//!
//! ```
//! use izanagi_kit::veb3::VebTree;
//! let mut t = VebTree::new(16); // 16-bit universe
//! t.insert(5);
//! t.insert(100);
//! t.insert(50);
//! assert_eq!(t.min(), Some(5));
//! assert_eq!(t.successor(5), Some(50));
//! assert_eq!(t.predecessor(100), Some(50));
//! assert!(t.member(50));
//! t.remove(50);
//! assert_eq!(t.successor(5), Some(100));
//! ```
//!
//! References: van Emde Boas (1977); CLRS ch. 20.3 for the
//! `x > max` swap-and-insert and the summary-delete bookkeeping.

use std::collections::BTreeMap;

/// Keys split into `high` (cluster index) and `low` (index
/// inside a cluster); clusters of at most `2^LEAF_BITS`
/// elements bottom out as plain bit-masks.
const LEAF_BITS: u32 = 6;

enum Repr {
    /// Presence mask over the `2^bits` universe (bits ≤ 6).
    Leaf(u64),
    /// `summary` over cluster indices; `clusters` only for
    /// nonempty clusters. Node `min`/`max` hold the smallest
    /// and largest keys and are NOT duplicated into clusters.
    Rec {
        summary: Box<VebTree>,
        clusters: BTreeMap<u64, VebTree>,
    },
}

/// Recursive van Emde Boas tree over `{0 .. 2^bits}`.
pub struct VebTree {
    bits: u32,
    /// Low-part width for `Repr::Rec` (`bits/2`); the high
    /// part — cluster indices — gets `bits - lo_bits` bits so
    /// the split is `⌈bits/2⌉ / ⌊bits/2⌋` (CLRS convention).
    lo_bits: u32,
    repr: Repr,
    min: u64,
    max: u64,
    has: bool,
}

impl VebTree {
    /// A tree over keys `< 2^bits` (`bits` 0..=64; practical
    /// bound ~40 given recursion overhead).
    pub fn new(bits: u32) -> Self {
        let bits = bits.min(64);
        let (lo_bits, repr) = if bits <= LEAF_BITS {
            (bits, Repr::Leaf(0))
        } else {
            let lo = bits / 2;
            let hi = bits - lo;
            (
                lo,
                Repr::Rec {
                    summary: Box::new(VebTree::new(hi)),
                    clusters: BTreeMap::new(),
                },
            )
        };
        VebTree {
            bits,
            lo_bits,
            repr,
            min: 0,
            max: 0,
            has: false,
        }
    }

    fn mask(&self) -> u64 {
        (1u64 << self.lo_bits) - 1
    }

    fn high(&self, x: u64) -> u64 {
        x >> self.lo_bits
    }

    fn low(&self, x: u64) -> u64 {
        x & self.mask()
    }

    fn index(&self, hi: u64, lo: u64) -> u64 {
        (hi << self.lo_bits) | lo
    }

    /// Number of elements.
    pub fn is_empty(&self) -> bool {
        !self.has
    }

    /// Smallest element (`None` when empty).
    pub fn min(&self) -> Option<u64> {
        if self.has {
            Some(self.min)
        } else {
            None
        }
    }

    /// Largest element (`None` when empty).
    pub fn max(&self) -> Option<u64> {
        if self.has {
            Some(self.max)
        } else {
            None
        }
    }

    /// Membership test, `O(log log U)` levels.
    pub fn member(&self, x: u64) -> bool {
        if !self.has || (self.bits < 64 && x >= 1u64 << self.bits) {
            return false;
        }
        if x == self.min || x == self.max {
            return true;
        }
        match &self.repr {
            Repr::Leaf(m) => m >> x & 1 == 1,
            Repr::Rec { clusters, .. } => match clusters.get(&self.high(x)) {
                Some(c) => c.member(self.low(x)),
                None => false,
            },
        }
    }

    /// Insert `x` (no-op when already present or out of range).
    pub fn insert(&mut self, x: u64) {
        if self.bits < 64 && x >= 1u64 << self.bits {
            return;
        }
        if let Repr::Leaf(m) = &mut self.repr {
            // leaf mask holds the *whole* set; min/max are a cache
            *m |= 1 << x;
            if !self.has {
                self.min = x;
                self.max = x;
                self.has = true;
            } else {
                self.min = self.min.min(x);
                self.max = self.max.max(x);
            }
            return;
        }
        if !self.has {
            self.min = x;
            self.max = x;
            self.has = true;
            return;
        }
        if x == self.min || x == self.max {
            return;
        }
        let mut x = x;
        if x < self.min {
            std::mem::swap(&mut x, &mut self.min);
        }
        if x > self.max {
            self.max = x;
        }
        let lo_bits = self.lo_bits;
        match &mut self.repr {
            Repr::Leaf(_) => {}
            Repr::Rec { summary, clusters } => {
                let hi = x >> lo_bits;
                let lo = x & ((1u64 << lo_bits) - 1);
                if let Some(c) = clusters.get_mut(&hi) {
                    c.insert(lo);
                } else {
                    summary.insert(hi);
                    let mut c = VebTree::new(lo_bits);
                    c.insert(lo);
                    clusters.insert(hi, c);
                }
            }
        }
    }

    /// Remove `x` (no-op when absent).
    pub fn remove(&mut self, x: u64) {
        if !self.has || self.bits < 64 && x >= 1u64 << self.bits {
            return;
        }
        if self.min == self.max {
            // single element — must be x
            if x == self.min {
                self.has = false;
                if let Repr::Leaf(m) = &mut self.repr {
                    *m = 0;
                }
            }
            return;
        }
        match &mut self.repr {
            Repr::Leaf(m) => {
                *m &= !(1u64 << x);
                if x == self.min {
                    self.min = m.trailing_zeros() as u64;
                }
                if x == self.max {
                    self.max = 63 - m.leading_zeros() as u64;
                }
            }
            Repr::Rec { summary, clusters } => {
                let lo_bits = self.lo_bits;
                let index = |hi: u64, lo: u64| (hi << lo_bits) | lo;
                let mut x = x;
                if x == self.min {
                    // second-smallest becomes the new min
                    let hi = summary.min().unwrap_or(0);
                    x = index(hi, clusters.get(&hi).and_then(|c| c.min()).unwrap_or(0));
                    self.min = x;
                }
                let hi = x >> lo_bits;
                let lo = x & ((1u64 << lo_bits) - 1);
                if let Some(c) = clusters.get_mut(&hi) {
                    c.remove(lo);
                    if c.is_empty() {
                        clusters.remove(&hi);
                        summary.remove(hi);
                    }
                }
                if x == self.max {
                    match summary.max() {
                        Some(smax) => {
                            self.max =
                                index(smax, clusters.get(&smax).and_then(|c| c.max()).unwrap_or(0));
                        }
                        None => self.max = self.min,
                    }
                }
            }
        }
    }

    /// Smallest element `> x` (`None` past the end).
    pub fn successor(&self, x: u64) -> Option<u64> {
        if !self.has {
            return None;
        }
        if x < self.min {
            return Some(self.min);
        }
        if x >= self.max {
            return None;
        }
        match &self.repr {
            Repr::Leaf(m) => {
                let rest = if x + 1 >= 64 {
                    0
                } else {
                    m & !((1u64 << (x + 1)) - 1)
                };
                if rest == 0 {
                    None
                } else {
                    Some(rest.trailing_zeros() as u64)
                }
            }
            Repr::Rec { summary, clusters } => {
                let hi = self.high(x);
                let lo = self.low(x);
                if let Some(c) = clusters.get(&hi) {
                    if let Some(cmax) = c.max() {
                        if lo < cmax {
                            return c.successor(lo).map(|s| self.index(hi, s));
                        }
                    }
                }
                summary
                    .successor(hi)
                    .map(|s| self.index(s, clusters.get(&s).and_then(|c| c.min()).unwrap_or(0)))
            }
        }
    }

    /// Largest element `< x` (`None` before the start).
    pub fn predecessor(&self, x: u64) -> Option<u64> {
        if !self.has {
            return None;
        }
        if x > self.max {
            return Some(self.max);
        }
        if x <= self.min {
            return None;
        }
        match &self.repr {
            Repr::Leaf(m) => {
                let rest = m & ((1u64 << x) - 1);
                if rest == 0 {
                    None
                } else {
                    Some(63 - rest.leading_zeros() as u64)
                }
            }
            Repr::Rec { summary, clusters } => {
                let hi = self.high(x);
                let lo = self.low(x);
                if let Some(c) = clusters.get(&hi) {
                    if let Some(cmin) = c.min() {
                        if lo > cmin {
                            return c.predecessor(lo).map(|s| self.index(hi, s));
                        }
                    }
                }
                match summary.predecessor(hi) {
                    Some(s) => {
                        Some(self.index(s, clusters.get(&s).and_then(|c| c.max()).unwrap_or(0)))
                    }
                    // `min` lives outside clusters, so no earlier
                    // cluster means `min` itself is the answer
                    None => Some(self.min),
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

    #[test]
    fn basic() {
        let mut t = VebTree::new(8);
        for &x in &[10u64, 3, 200, 55, 3] {
            t.insert(x);
        }
        assert_eq!(t.min(), Some(3));
        assert_eq!(t.max(), Some(200));
        assert!(t.member(55));
        assert!(!t.member(56));
        assert_eq!(t.successor(3), Some(10));
        assert_eq!(t.successor(55), Some(200));
        assert_eq!(t.successor(200), None);
        assert_eq!(t.predecessor(200), Some(55));
        t.remove(3);
        assert_eq!(t.min(), Some(10));
        t.remove(200);
        assert_eq!(t.max(), Some(55));
        t.remove(10);
        t.remove(55);
        assert!(t.is_empty());
        assert_eq!(t.min(), None);
    }

    #[test]
    fn oracle_btreeset() {
        let mut rng = SplitMix64::new(21);
        for bits in [4u32, 7, 10, 13] {
            let mut t = VebTree::new(bits);
            let mut oracle = BTreeSet::new();
            let range = 1u64 << bits;
            for _ in 0..700 {
                match rng.below(4) {
                    0 => {
                        let x = rng.below((range * 2) as u32) as u64; // half out-of-range
                        t.insert(x);
                        if x < range {
                            oracle.insert(x);
                        }
                    }
                    1 => {
                        let x = rng.below((range * 2) as u32) as u64;
                        t.remove(x);
                        if x < range {
                            oracle.remove(&x);
                        }
                    }
                    2 => {
                        let x = rng.below(range as u32) as u64;
                        let want = oracle.range((x + 1)..).next().copied();
                        assert_eq!(t.successor(x), want, "succ({}) bits={}", x, bits);
                    }
                    _ => {
                        let x = rng.below(range as u32) as u64;
                        let want = oracle.range(..x).next_back().copied();
                        assert_eq!(t.predecessor(x), want, "pred({}) bits={}", x, bits);
                    }
                }
                assert_eq!(t.min(), oracle.iter().next().copied());
                assert_eq!(t.max(), oracle.iter().next_back().copied());
            }
        }
    }

    #[test]
    fn deep_recursion() {
        let mut t = VebTree::new(64);
        let mut rng = SplitMix64::new(7);
        let mut oracle = BTreeSet::new();
        for _ in 0..400 {
            let x = rng.next_u64();
            t.insert(x);
            oracle.insert(x);
        }
        for _ in 0..400 {
            let x = rng.next_u64();
            let want = oracle.range((x + 1)..).next().copied();
            assert_eq!(t.successor(x), want);
        }
        // delete half
        let vals: Vec<u64> = oracle.iter().copied().collect();
        for (i, v) in vals.iter().enumerate() {
            if i % 2 == 0 {
                t.remove(*v);
                oracle.remove(v);
            }
        }
        assert_eq!(t.min(), oracle.iter().next().copied());
        assert_eq!(t.max(), oracle.iter().next_back().copied());
    }

    #[test]
    fn edge_universe_bounds() {
        let mut t = VebTree::new(4); // universe 0..16
        t.insert(15);
        t.insert(0);
        t.insert(16); // out of range — ignored
        assert_eq!(t.max(), Some(15));
        assert!(!t.member(16));
        t.remove(0);
        t.remove(15);
        assert!(t.is_empty());
    }
}

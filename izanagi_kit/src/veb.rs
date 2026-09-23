//! Predecessor/successor set over the `u32` universe — a proto
//! van Emde Boas layout: the 32-bit key splits as `hi:lo` (16+16),
//! a 1024-word `top` bitset marks nonempty clusters and each cluster
//! is a 1024-word bitset of its `lo` values. Membership, min/max,
//! `predecessor`, `successor` all run in bounded-constant work (one
//! or two bitset scans over ≤128 words), so the structure answers
//! successor queries without any pointer chasing or hashing —
//! neighbours in sorted order are a pure function of the set.
//!
//! ```
//! use izanagi_kit::veb::Veb;
//! let mut v = Veb::new();
//! v.insert(10);
//! v.insert(70_000);
//! v.insert(3);
//! assert_eq!(v.predecessor(9), Some(3));
//! assert_eq!(v.successor(10), Some(10)); // successor is inclusive: ≥ x
//! assert_eq!(v.successor(11), Some(70_000));
//! assert_eq!(v.iter().collect::<Vec<u32>>(), vec![3, 10, 70_000]);
//! ```

use std::collections::BTreeMap;

/// Sorted `u32` set with `O(1)`-class predecessor queries via a
/// two-level (sqrt) van Emde Boas layout.
pub struct Veb {
    /// `top[hi]` marks clusters (high 16 bits) holding ≥1 member.
    top: Box<[u64; 1024]>,
    clusters: BTreeMap<u16, Box<[u64; 1024]>>,
    n: usize,
}

fn word_set(w: u64, bit: u32) -> u64 {
    w | (1u64 << bit)
}

fn word_clear(w: u64, bit: u32) -> u64 {
    w & !(1u64 << bit)
}

fn lowest_set(w: u64) -> Option<u32> {
    if w == 0 {
        None
    } else {
        Some(w.trailing_zeros())
    }
}

fn highest_set(w: u64) -> Option<u32> {
    if w == 0 {
        None
    } else {
        Some(63 - w.leading_zeros())
    }
}

impl Default for Veb {
    fn default() -> Self {
        Self::new()
    }
}

impl Veb {
    /// Empty set over the full `u32` universe.
    pub fn new() -> Self {
        Self {
            top: Box::new([0u64; 1024]),
            clusters: BTreeMap::new(),
            n: 0,
        }
    }

    /// Insert `x`; returns whether it was new.
    pub fn insert(&mut self, x: u32) -> bool {
        let hi = (x >> 16) as usize;
        let lo = x & 0xffff;
        let entry = self
            .clusters
            .entry(hi as u16)
            .or_insert_with(|| Box::new([0u64; 1024]));
        let wi = (lo / 64) as usize;
        let bit = lo % 64;
        if (entry[wi] >> bit) & 1 == 1 {
            return false;
        }
        entry[wi] = word_set(entry[wi], bit);
        self.top[hi / 64] = word_set(self.top[hi / 64], (hi % 64) as u32);
        self.n += 1;
        true
    }

    /// Remove `x`; returns whether it was present.
    pub fn remove(&mut self, x: u32) -> bool {
        let hi = (x >> 16) as usize;
        let lo = x & 0xffff;
        let Some(cluster) = self.clusters.get_mut(&(hi as u16)) else {
            return false;
        };
        let wi = (lo / 64) as usize;
        let bit = lo % 64;
        if (cluster[wi] >> bit) & 1 == 0 {
            return false;
        }
        cluster[wi] = word_clear(cluster[wi], bit);
        self.n -= 1;
        if cluster.iter().all(|&w| w == 0) {
            self.clusters.remove(&(hi as u16));
            self.top[hi / 64] = word_clear(self.top[hi / 64], (hi % 64) as u32);
        }
        true
    }

    /// Membership test.
    pub fn contains(&self, x: u32) -> bool {
        let hi = (x >> 16) as usize;
        let lo = x & 0xffff;
        match self.clusters.get(&(hi as u16)) {
            Some(c) => (c[(lo / 64) as usize] >> (lo % 64)) & 1 == 1,
            None => false,
        }
    }

    /// Element count.
    pub fn len(&self) -> usize {
        self.n
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Lowest member of the cluster `hi` (`lo`-space scan).
    fn cluster_min(&self, hi: u16) -> Option<u32> {
        let c = self.clusters.get(&hi)?;
        for (wi, &w) in c.iter().enumerate() {
            if let Some(b) = lowest_set(w) {
                return Some((wi as u32) * 64 + b);
            }
        }
        None
    }

    /// Highest member of the cluster `hi`.
    fn cluster_max(&self, hi: u16) -> Option<u32> {
        let c = self.clusters.get(&hi)?;
        for wi in (0..1024).rev() {
            if let Some(b) = highest_set(c[wi]) {
                return Some((wi as u32) * 64 + b);
            }
        }
        None
    }

    /// Smallest member (`None` on empty).
    pub fn min(&self) -> Option<u32> {
        let (hi, _) = self.clusters.iter().next()?;
        self.cluster_min(*hi).map(|lo| ((*hi as u32) << 16) | lo)
    }

    /// Largest member.
    pub fn max(&self) -> Option<u32> {
        let (hi, _) = self.clusters.iter().next_back()?;
        self.cluster_max(*hi).map(|lo| ((*hi as u32) << 16) | lo)
    }

    /// Greatest member `≤ x`.
    pub fn predecessor(&self, x: u32) -> Option<u32> {
        let hi = (x >> 16) as usize;
        let lo = x & 0xffff;
        if let Some(c) = self.clusters.get(&(hi as u16)) {
            // Scan within `lo` downward.
            let wi = (lo / 64) as usize;
            let bit = lo % 64;
            let masked = c[wi] & ((!0u64) >> (63 - bit));
            if let Some(b) = highest_set(masked) {
                return Some(((hi as u32) << 16) | ((wi as u32) * 64 + b));
            }
            for wj in (0..wi).rev() {
                if let Some(b) = highest_set(c[wj]) {
                    return Some(((hi as u32) << 16) | ((wj as u32) * 64 + b));
                }
            }
        }
        // Fall to the greatest nonempty cluster below `hi`.
        let mut ci = hi;
        while ci > 0 {
            ci -= 1;
            if (self.top[ci / 64] >> (ci % 64)) & 1 == 1 {
                let c = self.cluster_max(ci as u16)?;
                return Some(((ci as u32) << 16) | c);
            }
        }
        None
    }

    /// Least member `≥ x`.
    pub fn successor(&self, x: u32) -> Option<u32> {
        let hi = (x >> 16) as usize;
        let lo = x & 0xffff;
        if let Some(c) = self.clusters.get(&(hi as u16)) {
            let wi = (lo / 64) as usize;
            let bit = lo % 64;
            let masked = c[wi] & ((!0u64) << bit);
            if let Some(b) = lowest_set(masked) {
                return Some(((hi as u32) << 16) | ((wi as u32) * 64 + b));
            }
            for wj in wi + 1..1024 {
                if let Some(b) = lowest_set(c[wj]) {
                    return Some(((hi as u32) << 16) | ((wj as u32) * 64 + b));
                }
            }
        }
        // Least nonempty cluster above `hi`.
        let mut ci = hi + 1;
        while ci < 65536 {
            if (self.top[ci / 64] >> (ci % 64)) & 1 == 1 {
                let c = self.cluster_min(ci as u16)?;
                return Some(((ci as u32) << 16) | c);
            }
            ci += 1;
        }
        None
    }

    /// Ascending iteration over every member.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        let mut v: Vec<u32> = Vec::with_capacity(self.n);
        for (&hi, c) in &self.clusters {
            for (wi, &w) in c.iter().enumerate() {
                let mut word = w;
                while word != 0 {
                    let b = word.trailing_zeros();
                    v.push(((hi as u32) << 16) | ((wi as u32) * 64 + b));
                    word &= word - 1;
                }
            }
        }
        v.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn basic_ops() {
        let mut v = Veb::new();
        assert!(v.is_empty());
        assert_eq!(v.min(), None);
        assert_eq!(v.max(), None);
        assert_eq!(v.predecessor(7), None);
        assert!(v.insert(7));
        assert!(!v.insert(7));
        assert!(v.contains(7));
        assert_eq!(v.min(), Some(7));
        assert_eq!(v.max(), Some(7));
        assert!(v.remove(7));
        assert!(!v.remove(7));
        assert!(!v.contains(7));
    }

    #[test]
    fn oracle_all_ops() {
        let mut rng = SplitMix64::new(5);
        let mut v = Veb::new();
        let mut s = BTreeSet::new();
        for _ in 0..4000 {
            let x = rng.next_u64() as u32;
            match rng.below(4) {
                0 => {
                    assert_eq!(v.insert(x), s.insert(x));
                }
                1 => {
                    assert_eq!(v.remove(x), s.remove(&x));
                }
                2 => {
                    assert_eq!(v.contains(x), s.contains(&x));
                }
                _ => {
                    assert_eq!(
                        v.predecessor(x),
                        s.range(..=x).next_back().copied(),
                        "pred({x})"
                    );
                    assert_eq!(v.successor(x), s.range(x..).next().copied(), "succ({x})");
                }
            }
        }
        assert_eq!(v.len(), s.len());
        assert_eq!(v.min(), s.iter().next().copied());
        assert_eq!(v.max(), s.iter().next_back().copied());
        let got: Vec<u32> = v.iter().collect();
        let want: Vec<u32> = s.iter().copied().collect();
        assert_eq!(got, want);
    }

    #[test]
    fn edge_values() {
        let mut v = Veb::new();
        v.insert(0);
        v.insert(0xffff_fffeu32);
        v.insert(0xffff_fffd);
        assert_eq!(v.min(), Some(0));
        assert_eq!(v.max(), Some(0xffff_fffeu32));
        assert_eq!(v.predecessor(0xffff_fffeu32), Some(0xffff_fffeu32));
        assert_eq!(v.successor(1), Some(0xffff_fffd));
        assert_eq!(v.predecessor(0), Some(0));
        assert_eq!(v.successor(0xffff_fffeu32), Some(0xffff_fffeu32));
    }

    #[test]
    fn dense_cluster_crossing() {
        // Cluster-boundary predecessor walks.
        let mut v = Veb::new();
        v.insert(0x0000_ffff);
        v.insert(0x0001_0000);
        v.insert(0x0002_0000);
        assert_eq!(v.predecessor(0x0001_0001), Some(0x0001_0000));
        assert_eq!(v.successor(0x0000_fffe), Some(0x0000_ffff));
        assert_eq!(v.predecessor(0x0002_0000), Some(0x0002_0000));
    }
}

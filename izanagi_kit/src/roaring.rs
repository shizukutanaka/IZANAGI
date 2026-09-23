//! Roaring bitmap (Chambi, Lemire et al. 2016): a `u32` set split
//! into 2^16 containers keyed by the high 16 bits; sparse containers
//! hold a sorted `u16` array, dense ones a 1024-word bitset, with a
//! 4096-entry conversion threshold. Iteration is always ascending
//! and container layout is a function of the set content only —
//! `(membership)` is a pure function, so equal sets serialize into
//! identical structures regardless of insertion order. `u32` keys
//! make it a compressed resident set for entity ids, tile flags,
//! or large rank indexes where a raw `[u64; 2^26]` bitset is too fat.
//!
//! ```
//! use izanagi_kit::roaring::Roaring;
//! let mut a = Roaring::new();
//! a.add(3);
//! a.add(0x0002_0001);
//! let mut b = Roaring::new();
//! b.add(0x0002_0001);
//! let u = a.union(&b);
//! assert_eq!(u.len(), 2);
//! assert!(u.contains(3) && u.contains(0x0002_0001));
//! ```

/// Array container capacity before it upgrades to a bitset.
const ARRAY_LIMIT: usize = 4096;

#[derive(Clone)]
enum Container {
    Array(Vec<u16>),
    Bits(Box<[u64; 1024]>),
}

impl Container {
    fn len(&self) -> usize {
        match self {
            Self::Array(v) => v.len(),
            Self::Bits(w) => w.iter().map(|x| x.count_ones() as usize).sum(),
        }
    }

    fn contains(&self, x: u16) -> bool {
        match self {
            Self::Array(v) => v.binary_search(&x).is_ok(),
            Self::Bits(w) => (w[(x / 64) as usize] >> (x % 64)) & 1 == 1,
        }
    }

    fn as_bits(&self) -> Box<[u64; 1024]> {
        let mut w = [0u64; 1024];
        match self {
            Self::Array(v) => {
                for &x in v {
                    w[(x / 64) as usize] |= 1u64 << (x % 64);
                }
            }
            Self::Bits(b) => w.copy_from_slice(&b[..]),
        }
        Box::new(w)
    }

    fn normalize(&self) -> Self {
        if self.len() <= ARRAY_LIMIT {
            Self::Array(self.iter().collect())
        } else {
            Self::Bits(self.as_bits())
        }
    }

    fn add(&mut self, x: u16) {
        match self {
            Self::Array(v) => {
                if let Err(i) = v.binary_search(&x) {
                    v.insert(i, x);
                }
            }
            Self::Bits(w) => w[(x / 64) as usize] |= 1u64 << (x % 64),
        }
        if self.len() > ARRAY_LIMIT {
            *self = Self::Bits(self.as_bits());
        }
    }

    fn remove(&mut self, x: u16) -> bool {
        let had = self.contains(x);
        if !had {
            return false;
        }
        match self {
            Self::Array(v) => {
                if let Ok(i) = v.binary_search(&x) {
                    v.remove(i);
                }
            }
            Self::Bits(w) => w[(x / 64) as usize] &= !(1u64 << (x % 64)),
        }
        if self.len() <= ARRAY_LIMIT {
            *self = self.normalize();
        }
        true
    }

    fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        let mut v: Vec<u16> = Vec::new();
        match self {
            Self::Array(a) => v.extend_from_slice(a),
            Self::Bits(w) => {
                for (wi, &word) in w.iter().enumerate() {
                    let mut x = word;
                    while x != 0 {
                        let b = x.trailing_zeros();
                        v.push((wi as u16) * 64 + b as u16);
                        x &= x - 1;
                    }
                }
            }
        }
        v.into_iter()
    }
}

/// A `u32` set as a roaring bitmap: sorted containers keyed by the
/// high 16 bits.
#[derive(Clone, Default)]
pub struct Roaring {
    containers: Vec<(u16, Container)>,
}

impl Roaring {
    /// Empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from an unsorted slice — output is identical for any
    /// input permutation.
    pub fn from_slice(xs: &[u32]) -> Self {
        let mut r = Self::new();
        for &x in xs {
            r.add(x);
        }
        r
    }

    fn slot(&self, key: u16) -> Result<usize, usize> {
        self.containers.binary_search_by_key(&key, |(k, _)| *k)
    }

    /// Insert `x`; returns whether it was new.
    pub fn add(&mut self, x: u32) -> bool {
        let key = (x >> 16) as u16;
        let low = x as u16;
        if self.contains(x) {
            return false;
        }
        match self.slot(key) {
            Ok(i) => self.containers[i].1.add(low),
            Err(i) => {
                let mut c = Container::Array(Vec::new());
                c.add(low);
                self.containers.insert(i, (key, c));
            }
        }
        true
    }

    /// Remove `x`; returns whether it was present.
    pub fn remove(&mut self, x: u32) -> bool {
        let key = (x >> 16) as u16;
        match self.slot(key) {
            Ok(i) => {
                let removed = self.containers[i].1.remove(x as u16);
                if self.containers[i].1.len() == 0 {
                    self.containers.remove(i);
                }
                removed
            }
            Err(_) => false,
        }
    }

    /// Membership test.
    pub fn contains(&self, x: u32) -> bool {
        match self.slot((x >> 16) as u16) {
            Ok(i) => self.containers[i].1.contains(x as u16),
            Err(_) => false,
        }
    }

    /// Element count.
    pub fn len(&self) -> usize {
        self.containers.iter().map(|(_, c)| c.len()).sum()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.containers.is_empty()
    }

    /// Ascending iteration over every member.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.containers
            .iter()
            .flat_map(|(k, c)| c.iter().map(move |lo| ((*k as u32) << 16) | lo as u32))
    }

    /// Apply `op` wordwise over paired containers; missing side is
    /// treated as all-zeros.
    fn combine(&self, other: &Self, and_not: bool, or: bool) -> Self {
        let mut out = Self::new();
        let mut i = 0;
        let mut j = 0;
        while i < self.containers.len() || j < other.containers.len() {
            let (key, lhs, rhs): (u16, Option<&Container>, Option<&Container>) =
                match (self.containers.get(i), other.containers.get(j)) {
                    (Some(a), Some(b)) if a.0 == b.0 => (a.0, Some(&a.1), Some(&b.1)),
                    (Some(a), Some(b)) if a.0 < b.0 => (a.0, Some(&a.1), None),
                    (Some(a), None) => (a.0, Some(&a.1), None),
                    (_, Some(b)) => (b.0, None, Some(&b.1)),
                    (None, None) => break,
                };
            if self.containers.get(i).is_some_and(|a| a.0 == key) {
                i += 1;
            }
            if other.containers.get(j).is_some_and(|b| b.0 == key) {
                j += 1;
            }
            let aw = lhs.map(|c| c.as_bits());
            let bw = rhs.map(|c| c.as_bits());
            let mut w = [0u64; 1024];
            for idx in 0..1024 {
                let a = aw.as_ref().map(|x| x[idx]).unwrap_or(0);
                let b = bw.as_ref().map(|x| x[idx]).unwrap_or(0);
                w[idx] = if or {
                    a | b
                } else if and_not {
                    a & !b
                } else {
                    a & b
                };
            }
            let c = Container::Bits(Box::new(w)).normalize();
            if c.len() > 0 {
                out.containers.push((key, c));
            }
        }
        out
    }

    /// `self ∪ other`.
    pub fn union(&self, other: &Self) -> Self {
        self.combine(other, false, true)
    }

    /// `self ∩ other`.
    pub fn intersect(&self, other: &Self) -> Self {
        self.combine(other, false, false)
    }

    /// `self ∖ other`.
    pub fn difference(&self, other: &Self) -> Self {
        self.combine(other, true, false)
    }

    /// `self △ other`.
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        self.union(other).difference(&self.intersect(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn to_set(r: &Roaring) -> BTreeSet<u32> {
        r.iter().collect()
    }

    #[test]
    fn basic_add_remove_contains() {
        let mut r = Roaring::new();
        assert!(r.add(5));
        assert!(!r.add(5));
        assert!(r.contains(5));
        assert!(r.remove(5));
        assert!(!r.remove(5));
        assert!(!r.contains(5));
        assert!(r.is_empty());
    }

    #[test]
    fn container_upgrade_downgrade() {
        let mut r = Roaring::new();
        for i in 0..5000u32 {
            r.add(i);
        }
        assert_eq!(r.len(), 5000);
        assert!(matches!(r.containers[0].1, Container::Bits(_)));
        for i in 0..4500u32 {
            r.remove(i);
        }
        assert_eq!(r.len(), 500);
        assert!(matches!(r.containers[0].1, Container::Array(_)));
    }

    #[test]
    fn ops_match_btree_set() {
        let mut rng = SplitMix64::new(41);
        for _ in 0..30 {
            let xs: Vec<u32> = (0..rng.below(300) + 1)
                .map(|_| rng.below(0x0004_0000) | (rng.below(3) << 20))
                .collect();
            let ys: Vec<u32> = (0..rng.below(300) + 1)
                .map(|_| rng.below(0x0004_0000) | (rng.below(3) << 20))
                .collect();
            let ra = Roaring::from_slice(&xs);
            let rb = Roaring::from_slice(&ys);
            let sa: BTreeSet<u32> = xs.iter().copied().collect();
            let sb: BTreeSet<u32> = ys.iter().copied().collect();
            assert_eq!(to_set(&ra), sa);
            assert_eq!(to_set(&ra.union(&rb)), &sa | &sb);
            assert_eq!(to_set(&ra.intersect(&rb)), &sa & &sb);
            assert_eq!(to_set(&ra.difference(&rb)), &sa - &sb);
            assert_eq!(
                to_set(&ra.symmetric_difference(&rb)),
                &(&sa | &sb) - &(&sa & &sb)
            );
            assert_eq!(ra.len(), sa.len());
            for &x in &xs {
                assert!(ra.contains(x));
            }
        }
    }

    #[test]
    fn insertion_order_independence() {
        let mut rng = SplitMix64::new(97);
        let mut xs: Vec<u32> = (0..200).map(|_| rng.below(0x0010_0000)).collect();
        let a = Roaring::from_slice(&xs);
        xs.reverse();
        let b = Roaring::from_slice(&xs);
        assert_eq!(to_set(&a), to_set(&b));
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn sorted_iteration() {
        let mut rng = SplitMix64::new(7);
        let xs: Vec<u32> = (0..500).map(|_| rng.below(0xffff_ffff)).collect();
        let r = Roaring::from_slice(&xs);
        let v: Vec<u32> = r.iter().collect();
        let mut sorted = v.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(v, sorted);
    }
}

//! Minimal perfect hash — a static `n` keys → `{0..n}` bijection
//! built by the CHD/BDZ two-level displacement method. Each key lands
//! in a first-level bucket; per-bucket displacements `d[b]` resolve
//! every bucket's keys into disjoint table slots:
//!
//! ```text
//! bucket = h(key, 0) mod B
//! mphf   = h(key, d[bucket]) mod N
//! ```
//!
//! Buckets are resolved in descending-size order (ties: smaller
//! bucket index) and each displacement is searched upward from `0`,
//! so the table is a pure function of `(key set, seed)` — insertion
//! order cannot leak in. `hash` of a key not in the set still returns
//! a value in `[0, n)`: it is a perfect *hash*, not membership —
//! pair it with [`crate::xorfilter`] or [`crate::cuckoof`] when a
//! membership answer is needed.
//!
//! Intended use: static dispatch tables (skill/opcode → handler
//! index) where a lockstep executor needs index lookups with no heap
//! and no iteration-order dependence.
//!
//! ```
//! use izanagi_kit::mphf::Mphf;
//! let m = Mphf::build(&[10, 20, 30, 40], 0).unwrap();
//! let mut seen = [false; 4];
//! for &k in &[10u64, 20, 30, 40] { seen[m.hash(k)] = true; }
//! assert!(seen.iter().all(|&s| s)); // a bijection on the keys
//! ```

use crate::rng::SplitMix64;

const DOMAIN: u64 = 0x4d50_4846_2020_2020; // "MPHF    "

fn mix(seed: u64, x: u64) -> u64 {
    SplitMix64::new(seed ^ x).next_u64()
}

/// A minimal perfect hash over a fixed `u64` key set.
#[derive(Clone, Debug)]
pub struct Mphf {
    seed: u64,
    /// First-level bucket count.
    bucket_count: usize,
    /// Table size = number of keys.
    n: usize,
    /// Per-bucket displacements.
    d: Vec<u64>,
}

impl Mphf {
    /// Build a perfect hash over `keys`; duplicates are ignored.
    /// Returns `None` when `keys` is empty, or when the displacement
    /// search fails to resolve a bucket within its bound (retry with
    /// a different seed).
    pub fn build(keys: &[u64], seed: u64) -> Option<Self> {
        let mut uniq = keys.to_vec();
        uniq.sort_unstable();
        uniq.dedup();
        let n = uniq.len();
        if n == 0 {
            return None;
        }
        // ~1.5 keys per bucket on average keeps singleton buckets
        // common, which resolve at d = 0.
        let bucket_count = (n / 2).max(1).next_power_of_two();
        let mut buckets: Vec<Vec<u64>> = vec![Vec::new(); bucket_count];
        for &k in &uniq {
            buckets[(mix(seed, k) & (bucket_count - 1) as u64) as usize].push(k);
        }
        // Canonical resolution order: descending bucket size, then
        // smaller index — the order that minimizes expected d.
        let mut order: Vec<usize> = (0..bucket_count).collect();
        order.sort_by_key(|&b| (core::cmp::Reverse(buckets[b].len()), b));
        let mut used = vec![false; n];
        let mut d = vec![0u64; bucket_count];
        for &b in &order {
            if buckets[b].is_empty() {
                continue;
            }
            let mut disp = 0u64;
            let ok = loop {
                if disp > (n as u64).saturating_mul(4) + 64 {
                    // Pathological seed — caller retries with another.
                    break false;
                }
                let mut collided = false;
                let mut local = std::collections::BTreeSet::new();
                for &k in &buckets[b] {
                    let slot = (mix(seed ^ DOMAIN, k ^ disp) % n as u64) as usize;
                    if used[slot] || !local.insert(slot) {
                        collided = true;
                        break;
                    }
                }
                if !collided {
                    for &k in &buckets[b] {
                        used[(mix(seed ^ DOMAIN, k ^ disp) % n as u64) as usize] = true;
                    }
                    break true;
                }
                disp += 1;
            };
            if !ok {
                return None;
            }
            d[b] = disp;
        }
        Some(Self {
            seed,
            bucket_count,
            n,
            d,
        })
    }

    /// The hash of `key`, always in `[0, n)`.
    pub fn hash(&self, key: u64) -> usize {
        let b = (mix(self.seed, key) & (self.bucket_count - 1) as u64) as usize;
        (mix(self.seed ^ DOMAIN, key ^ self.d[b]) % self.n as u64) as usize
    }

    /// Table size = number of distinct keys seen at build.
    pub fn len(&self) -> usize {
        self.n
    }

    /// True when built over zero keys — `build` rejects that, so a
    /// constructed `Mphf` is never empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Memory used by the displacement table, in bytes.
    pub fn bytes(&self) -> usize {
        self.d.len() * 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn is_a_bijection_on_the_key_set() {
        let mut rng = SplitMix64::new(17);
        for _ in 0..30 {
            let n = rng.below(80) as usize + 1;
            let keys: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            let m = Mphf::build(&keys, 5).unwrap();
            let mut seen = BTreeSet::new();
            for &k in &keys {
                let h = m.hash(k);
                assert!(h < keys.len());
                assert!(seen.insert(h), "collision at slot {h}");
            }
            assert_eq!(seen.len(), keys.len());
        }
    }

    #[test]
    fn build_is_insertion_order_independent() {
        let mut a = Mphf::build(&[5, 1, 9, 3], 2).unwrap();
        let b = Mphf::build(&[9, 3, 1, 5], 2).unwrap();
        for k in 0..64u64 {
            assert_eq!(a.hash(k), b.hash(k));
        }
        a.d.sort_unstable();
        let mut bd = b.d;
        bd.sort_unstable();
        assert_eq!(a.d, bd);
    }

    #[test]
    fn tiny_sets_and_foreign_keys() {
        let m = Mphf::build(&[7], 0).unwrap();
        assert_eq!(m.hash(7), 0);
        // Foreign keys hash somewhere in range — no membership claim.
        for k in 0..50u64 {
            assert!(m.hash(k) < 1);
        }
        assert!(Mphf::build(&[], 0).is_none());
    }

    #[test]
    fn deduplicates_before_building() {
        let m = Mphf::build(&[4, 4, 4, 2, 2], 1).unwrap();
        assert_eq!(m.len(), 2);
        assert_ne!(m.hash(4), m.hash(2));
    }
}

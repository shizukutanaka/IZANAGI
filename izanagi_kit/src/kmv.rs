//! K-minimum-values (KMV) cardinality sketch — "about how many
//! distinct items did this stream see?" in `O(k log k)` space with
//! `k ≪ n`. Where [`bloom`](crate::bloom) answers "was this specific
//! item seen", KMV answers "how many were there" — replay-checkpoint
//! dedup counts, entity churn estimates, loot-table diversity audits.
//! The sketch is a pure function of `(seed, k, item multiset)`: the
//! same hashes always land in the same order.
//!
//! ```
//! use izanagi_kit::kmv::Kmv;
//! let mut s = Kmv::new(64, 0x5EED);
//! for i in 0..1000u64 {
//!     s.add(&i.to_le_bytes());
//! }
//! let est = s.estimate();
//! assert!(est > 700 && est < 1300, "est = {est}");
//! ```

use crate::world_hash::Fnv1a;

/// KMV sketch: keeps the `k` smallest seeded hashes ever seen.
/// `estimate` is exact while fewer than `k` distinct items arrive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kmv {
    k: usize,
    seed: u64,
    /// Sorted ascending, deduplicated, `len <= k`.
    min: Vec<u64>,
}

impl Kmv {
    /// Sketch keeping `k` minima; `k == 0` degenerates to "always 0".
    pub fn new(k: usize, seed: u64) -> Self {
        Kmv {
            k,
            seed,
            min: Vec::new(),
        }
    }

    /// Capacity.
    pub fn k(&self) -> usize {
        self.k
    }

    /// Feed one item's bytes.
    pub fn add(&mut self, data: &[u8]) {
        let mut h = Fnv1a::new();
        h.write_u64(self.seed);
        h.write_bytes(data);
        self.add_hash(h.finish());
    }

    /// Feed a pre-hashed value (raw u64 — no re-mixing).
    pub fn add_hash(&mut self, h: u64) {
        if self.k == 0 {
            return;
        }
        match self.min.binary_search(&h) {
            Ok(_) => {}
            Err(pos) => {
                self.min.insert(pos, h);
                self.min.truncate(self.k);
            }
        }
    }

    /// Exact count while `distinct < k`, else the KMV estimate
    /// `(k−1)·2⁶⁴ / vₖ` (`vₖ` = the `k`-th smallest hash), saturated
    /// to `u64::MAX`.
    pub fn estimate(&self) -> u64 {
        if self.k == 0 || self.min.len() < self.k {
            return self.min.len() as u64;
        }
        let vk = self.min[self.k - 1];
        if vk == 0 {
            return u64::MAX;
        }
        let e = (((self.k - 1) as u128) << 64) / vk as u128;
        e.min(u64::MAX as u128) as u64
    }

    /// Union of two sketches — the result equals the sketch of the
    /// merged stream, as long as both share `seed` (`k` may differ:
    /// the result keeps the smaller `k`, which is the maximum
    /// resolution the union can support).
    pub fn merge(&self, other: &Kmv) -> Kmv {
        let k = self.k.min(other.k);
        let mut m = Kmv {
            k,
            seed: self.seed,
            min: Vec::new(),
        };
        for &h in self.min.iter().chain(other.min.iter()) {
            m.add_hash(h);
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn exact_below_capacity_and_estimate_above() {
        let mut s = Kmv::new(64, 7);
        for i in 0..40u64 {
            s.add(&i.to_le_bytes());
        }
        assert_eq!(s.estimate(), 40);
        for i in 40..500u64 {
            s.add(&i.to_le_bytes());
        }
        let est = s.estimate();
        // KMV relative stddev ~ 1/sqrt(k-2); require a loose bound.
        assert!(est > 250 && est < 1000, "est = {est}");
    }

    #[test]
    fn matches_oracle_with_duplicates_and_merge() {
        let mut rng = SplitMix64::new(0xC0FFEE);
        for k in [1usize, 8, 32, 100] {
            let mut s = Kmv::new(k, 0xBEEF);
            let mut truth = BTreeSet::new();
            let mut total = 0usize;
            for _ in 0..2000 {
                let key = rng.below(3000) as u64;
                truth.insert(key);
                s.add(&key.to_le_bytes());
                total += 1;
            }
            let _ = total;
            let n = truth.len() as u64;
            let est = s.estimate();
            if n < k as u64 {
                assert_eq!(est, n);
            } else {
                // Relative stddev ≈ 1/√(k−2): require |est−n|²·(k−2)
                // < 16n² (a 4σ window) in pure integers.
                let sd2 = if k > 2 { (k - 2) as u128 } else { 1 };
                let d = (est.max(n) - est.min(n)) as u128;
                let nn = n as u128;
                assert!(d * d * sd2 < 16 * nn * nn, "k={k} n={n} est={est}");
            }
            // Merge must equal the single-stream sketch's answer.
            let mut a = Kmv::new(k, 0xBEEF);
            let mut b = Kmv::new(k, 0xBEEF);
            for &key in truth.iter().take(truth.len() / 2) {
                a.add(&key.to_le_bytes());
            }
            for &key in truth.iter().skip(truth.len() / 2) {
                b.add(&key.to_le_bytes());
            }
            // Estimates need not match exactly (both are approximate),
            // but the merged sketch's minima equal the union of minima.
            let m = a.merge(&b);
            assert_eq!(m.min.len(), s.min.len());
            assert_eq!(m.min, s.min);
        }
        assert_eq!(Kmv::new(0, 1).estimate(), 0);
        // Raw-hash feed: dedup + exact path without hashing bytes.
        let mut h = Kmv::new(4, 0);
        h.add_hash(9);
        h.add_hash(9);
        h.add_hash(1);
        assert_eq!(h.estimate(), 2);
        assert_eq!(h.k(), 4);
    }
}

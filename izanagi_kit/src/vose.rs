//! Vose's alias method — O(1) weighted sampling after O(n) build.
//! (Vose 1991, *A Linear Algorithm for Generating Random Numbers
//! with a Given Distribution*.) Each index becomes a biased coin
//! plus a redirect: pick a bucket uniformly, flip against
//! `prob[i]`, answer `i` or `alias[i]` — two lookups, one compare.
//!
//! Everything is exact integer arithmetic (`u128` intermediates):
//! `prob[i]` is a numerator over `total = Σw`, never a float.
//! The classic enumeration property holds *exactly*: bucket `i`'s
//! slot resolves to item `k` for exactly `w_k · n` of the `n·total`
//! (bucket, coin) pairs — the table is a perfect distribution, not
//! an approximation. Deterministic for a given `weights` slice.
//!
//! ```
//! use izanagi_kit::vose::AliasTable;
//! use izanagi_kit::rng::SplitMix64;
//! let t = AliasTable::build(&[1, 1, 2]).unwrap();
//! let mut rng = SplitMix64::new(7);
//! let i = t.sample(&mut rng);
//! assert!(i < 3);
//! ```

/// O(1) sampler for a finite `u64`-weighted distribution.
#[derive(Clone, Debug)]
pub struct AliasTable {
    n: u32,
    total: u128,
    /// numerator over `total` — probability of answering i directly.
    prob: Vec<u128>,
    alias: Vec<u32>,
}

impl AliasTable {
    /// Build over `weights` — `None` when empty or all-zero.
    /// Zero-weight items get `prob = 0` (never sampled directly).
    pub fn build(weights: &[u64]) -> Option<Self> {
        let n = weights.len();
        if n == 0 {
            return None;
        }
        let total: u128 = weights.iter().map(|&w| w as u128).sum();
        if total == 0 || total > u64::MAX as u128 {
            // coin flips draw from a u64 stream — a total above that
            // range can't be sampled uniformly.
            return None;
        }
        // scaled[i] = w_i · n — q_i = scaled[i]/total vs 1.
        let mut scaled: Vec<u128> = weights.iter().map(|&w| w as u128 * n as u128).collect();
        // Canonical selection: smallest index — table shape is a
        // pure function of weights regardless of pop order anyway,
        // but fixed order removes all doubt.
        let mut small: Vec<usize> = Vec::new();
        let mut large: Vec<usize> = Vec::new();
        for (i, &s) in scaled.iter().enumerate() {
            if s < total {
                small.push(i);
            } else {
                large.push(i);
            }
        }
        let mut prob = vec![0u128; n];
        let mut alias = vec![0u32; n];
        while !small.is_empty() && !large.is_empty() {
            // pop the *largest* small/large index — any fixed order is
            // canonical; last-element pops keep it O(1) per step.
            let s = small.pop()?;
            let l = large.pop()?;
            prob[s] = scaled[s];
            alias[s] = l as u32;
            scaled[l] = scaled[l] + scaled[s] - total;
            if scaled[l] < total {
                small.push(l);
            } else {
                large.push(l);
            }
        }
        // Remaining slots are exact: prob = total (always answer i).
        for &i in small.iter().chain(large.iter()) {
            prob[i] = total;
            alias[i] = i as u32;
        }
        Some(AliasTable {
            n: n as u32,
            total,
            prob,
            alias,
        })
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.n as usize
    }

    /// `len == 0` — always `false` (build rejects empty).
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Sum of weights.
    pub fn total(&self) -> u128 {
        self.total
    }

    /// Draw one index with `rng` — O(1).
    pub fn sample(&self, rng: &mut crate::rng::SplitMix64) -> usize {
        let i = rng.below(self.n) as usize;
        let coin = (rng.next_u64() % (self.total as u64)) as u128;
        if coin < self.prob[i] {
            i
        } else {
            self.alias[i] as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Exact oracle: enumerate every (bucket, coin) outcome and
    /// count which item each resolves to. A correct alias table
    /// gives item `k` exactly `w_k · n` hits out of `n · total`.
    fn enumerate_counts(t: &AliasTable) -> Vec<u128> {
        let n = t.n as usize;
        let mut coins = vec![0u128; n];
        let mut coin: u128 = 0;
        while coin < t.total {
            for i in 0..n {
                let hit = if coin < t.prob[i] {
                    i
                } else {
                    t.alias[i] as usize
                };
                coins[hit] += 1;
            }
            coin += 1;
        }
        coins
    }

    #[test]
    fn exact_distribution_over_all_outcomes() {
        let mut rng = SplitMix64::new(0xA1A5);
        for _ in 0..300 {
            let n = 1 + rng.below(8) as usize;
            let w: Vec<u64> = (0..n).map(|_| rng.below(20) as u64).collect();
            if w.iter().all(|&x| x == 0) {
                continue;
            }
            let t = AliasTable::build(&w).unwrap();
            let coins = enumerate_counts(&t);
            for (i, &wi) in w.iter().enumerate() {
                assert_eq!(coins[i], wi as u128 * n as u128, "w={w:?}");
            }
        }
    }

    #[test]
    fn sample_frequencies_and_zero_weight() {
        let t = AliasTable::build(&[0, 5, 5, 0, 10]).unwrap();
        let mut rng = SplitMix64::new(3);
        let mut hits = [0u32; 5];
        for _ in 0..40_000 {
            hits[t.sample(&mut rng)] += 1;
        }
        assert_eq!(hits[0], 0);
        assert_eq!(hits[3], 0);
        // 5/20 and 10/20 — loose statistical band
        assert!(hits[1] > 8000 && hits[1] < 12_000);
        assert!(hits[4] > 18_000 && hits[4] < 22_000);
        // determinism: same seed → same sequence
        let mut a = SplitMix64::new(9);
        let mut b = SplitMix64::new(9);
        for _ in 0..100 {
            assert_eq!(t.sample(&mut a), t.sample(&mut b));
        }
    }

    #[test]
    fn validation_and_singleton() {
        assert!(AliasTable::build(&[]).is_none());
        assert!(AliasTable::build(&[0, 0]).is_none());
        let t = AliasTable::build(&[0, 0, 9]).unwrap();
        let mut rng = SplitMix64::new(1);
        for _ in 0..50 {
            assert_eq!(t.sample(&mut rng), 2);
        }
        assert_eq!(t.len(), 3);
        assert!(!t.is_empty());
        assert_eq!(t.total(), 9);
    }
}

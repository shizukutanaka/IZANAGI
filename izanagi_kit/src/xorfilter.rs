//! Xor filter — Graf & Lemire's 2020 static membership structure.
//! The modern successor to [`crate::bloom`] when the key set is
//! frozen: ~1.23 bytes/key for a ~1/256 false-positive rate, exact
//! `false`/`true` answers in three XOR'd lookups, and — unlike a
//! Bloom filter — a query answer is one comparison, no bit-counting.
//!
//! Construction (the interesting part): each key maps to three
//! slots `h1,h2,h3` in disjoint thirds of the table, guaranteeing
//! distinctness without rejection sampling. Keys are *peeled* via
//! degree-1 slots in BFS queue order — fully deterministic for a
//! given `(keys, seed)` pair, so all peers build identical tables.
//! When the residual graph won't peel (a 2-core remains), the next
//! seed is tried automatically (≤64 attempts — probability of
//! failure is negligible for sane sizes).
//!
//! False positives only (~0.4%); a `false` answer is exact.
//! Duplicate keys are deduplicated before building.
//!
//! ```
//! use izanagi_kit::xorfilter::XorFilter;
//! let f = XorFilter::build(&[10, 20, 30, 40], 0xCAFE).unwrap();
//! assert!(f.contains(20));
//! assert!(!f.contains(21) || f.contains(21)); // ~0.4% may pass
//! ```

use crate::rng::SplitMix64;

/// Maximum build attempts — a failed peel needs a fresh seed.
const MAX_SEED_ATTEMPTS: u32 = 64;

/// A static approximate-membership table over `u64` keys.
#[derive(Clone, Debug)]
pub struct XorFilter {
    seed: u64,
    block_len: u32,
    table: Vec<u8>,
    n: usize,
}

fn fingerprint(key: u64, seed: u64) -> u8 {
    let mut r = SplitMix64::new(key ^ seed ^ 0xF1A6_E5C0);
    (r.next_u64() as u8).max(1) // fingerprints are never 0
}

impl XorFilter {
    /// Build over `keys` — `None` when the set is empty or the peel
    /// fails all 64 seeds (astronomically unlikely for a valid call).
    /// Duplicate keys are deduplicated first.
    pub fn build(keys: &[u64], seed: u64) -> Option<Self> {
        let mut uniq = keys.to_vec();
        uniq.sort_unstable();
        uniq.dedup();
        let n = uniq.len();
        if n == 0 {
            return None;
        }
        // Table: three disjoint segments so h1,h2,h3 are always
        // distinct positions — no rejection needed.
        let block_len = ((32 + (n as u64 * 123) / 100) / 3).max(4) as u32;
        let size = (block_len * 3) as usize;
        for attempt in 0..MAX_SEED_ATTEMPTS {
            let s = seed.wrapping_add(attempt as u64);
            // Slot assignment per key.
            let triples: Vec<[u32; 3]> = uniq
                .iter()
                .map(|&k| {
                    let mut r = SplitMix64::new(k ^ s);
                    let a = r.next_u64();
                    [
                        (a % block_len as u64) as u32,
                        block_len + (r.next_u64() % block_len as u64) as u32,
                        2 * block_len + (r.next_u64() % block_len as u64) as u32,
                    ]
                })
                .collect();
            // Count of keys per slot; degree-1 peel queue.
            let mut deg = vec![0u32; size];
            for t in &triples {
                for &p in t {
                    deg[p as usize] += 1;
                }
            }
            // slot -> the single key still claiming it (valid when deg==1)
            let mut claimant = vec![0usize; size];
            for (i, t) in triples.iter().enumerate() {
                for &p in t {
                    claimant[p as usize] ^= i; // xor: survives if exactly one
                }
            }
            let mut queue: Vec<u32> = (0..size as u32).filter(|&p| deg[p as usize] == 1).collect();
            let mut peel: Vec<(usize, u32)> = Vec::with_capacity(n);
            let mut removed = vec![false; n];
            let mut qi = 0;
            while qi < queue.len() {
                let p = queue[qi];
                qi += 1;
                if deg[p as usize] != 1 {
                    continue;
                }
                let k = claimant[p as usize];
                if removed[k] {
                    continue;
                }
                removed[k] = true;
                peel.push((k, p));
                for &q in &triples[k] {
                    let q = q as usize;
                    if q != p as usize && deg[q] > 0 {
                        deg[q] -= 1;
                        claimant[q] ^= k;
                        if deg[q] == 1 {
                            queue.push(q as u32);
                        }
                    }
                }
                deg[p as usize] = 0;
            }
            if peel.len() != n {
                continue; // 2-core remains — try next seed
            }
            // Assign fingerprints in reverse peel order.
            let mut table = vec![0u8; size];
            for &(k, p) in peel.iter().rev() {
                let key = uniq[k];
                table[p as usize] = fingerprint(key, s)
                    ^ table[triples[k][0] as usize]
                    ^ table[triples[k][1] as usize]
                    ^ table[triples[k][2] as usize];
            }
            return Some(XorFilter {
                seed: s,
                block_len,
                table,
                n,
            });
        }
        None
    }

    /// Number of distinct keys baked in.
    pub fn len(&self) -> usize {
        self.n
    }

    /// Always `false` — `build` rejects empty sets.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Approximate membership — `false` is exact; `true` is right
    /// with probability ~255/256 (~0.4% false positives).
    pub fn contains(&self, key: u64) -> bool {
        let mut r = SplitMix64::new(key ^ self.seed);
        let a = r.next_u64();
        let h1 = (a % self.block_len as u64) as usize;
        let h2 = self.block_len as usize + (r.next_u64() % self.block_len as u64) as usize;
        let h3 = 2 * self.block_len as usize + (r.next_u64() % self.block_len as u64) as usize;
        self.table[h1] ^ self.table[h2] ^ self.table[h3] == fingerprint(key, self.seed)
    }

    /// Table size in bytes.
    pub fn bytes(&self) -> usize {
        self.table.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn members_always_found_and_rate_bounded() {
        let mut rng2 = SplitMix64::new(0xA11CE5);
        for trial in 0..40 {
            let n = 1 + (rng2.below(400) as usize);
            let keys: BTreeSet<u64> = (0..n).map(|_| rng2.next_u64()).collect();
            let keys: Vec<u64> = keys.into_iter().collect();
            let f = XorFilter::build(&keys, 0xBEEF).unwrap();
            // zero false negatives — a hard guarantee of the peel
            for &k in &keys {
                assert!(f.contains(k), "member {k} rejected");
            }
            // fp rate ~1/256: bound generously at 4%.
            let mut fp = 0u32;
            let probes = 4000usize;
            let keyset: BTreeSet<u64> = keys.iter().copied().collect();
            for _ in 0..probes {
                let x = rng2.next_u64();
                if !keyset.contains(&x) && f.contains(x) {
                    fp += 1;
                }
            }
            assert!(
                fp * 100 <= probes as u32 * 4,
                "fp rate {fp}/{probes} too high (trial {trial})"
            );
        }
    }

    #[test]
    fn deterministic_and_duplicates_ok() {
        let keys = vec![7u64, 7, 42, 42, 99, 1, 1, 1];
        let a = XorFilter::build(&keys, 5).unwrap();
        let b = XorFilter::build(&keys, 5).unwrap();
        assert_eq!(a.table, b.table);
        assert_eq!(a.len(), 4);
        for &k in &[7u64, 42, 99, 1] {
            assert!(a.contains(k));
        }
        // small bits/key sanity: ~1.23 bytes/key
        assert!(a.bytes() < 2 * a.len() + 64);
    }

    #[test]
    fn empty_rejected_and_singletons() {
        assert!(XorFilter::build(&[], 1).is_none());
        let f = XorFilter::build(&[123], 1).unwrap();
        assert!(f.contains(123));
        assert!(!f.is_empty());
        assert_eq!(f.len(), 1);
    }
}

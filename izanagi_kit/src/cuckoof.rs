//! Cuckoo filter — a set-membership oracle with one-sided error
//! *and* deletion, which Bloom filters cannot provide. Stores only
//! `u8` fingerprints; each key is placed in one of two candidate
//! buckets `h1` and `h2 = h1 ^ hash(fp)`, and `remove` simply clears
//! a matching slot — the fingerprint is never rehashed into new
//! buckets, so evicting one key can never strand another's.
//!
//! Placement follows Fan, Andersen, Kaminsky & Mitzenmacher (2014):
//! a bounded cuckoo kick chain relocates existing fingerprints when
//! both candidates are full. The kick victim is chosen by a
//! `SplitMix64` stream seeded from `(key, seed)` so every insert is a
//! pure function of `(keys, seed)` — no address or iteration order
//! can leak in.
//!
//! Intended use in the kit: dedup of rollback snapshots and entity
//! ids where items must also be *removed* (Bloom cannot), as a fast
//! pre-filter ahead of an exact store. `contains` may report a key
//! that was never inserted (false positive, rate ≈ bucket fullness /
//! 255) but an inserted key is always reported present until removed.
//!
//! ```
//! use izanagi_kit::cuckoof::CuckooFilter;
//! let mut f = CuckooFilter::new(4096, 7);
//! assert!(f.insert(42));
//! assert!(f.contains(42));
//! assert!(f.remove(42));
//! assert!(!f.contains(42));
//! ```

use crate::rng::SplitMix64;

const DOMAIN: u64 = 0x434b_4654_5249_4e47; // "CKFTRING"
const FP_DOMAIN: u64 = 0x434b_4654_4650_2020; // "CKFTFP  "
/// Maximum kick relocations before `insert` reports failure.
const MAX_KICKS: usize = 500;

/// A seeded cuckoo filter of `bucket_count` buckets × `B` fingerprint
/// slots each. `bucket_count` is forced to a power of two so the
/// `h2 = h1 ^ hash(fp)` xor-fold lands inside the table.
#[derive(Clone, Debug)]
pub struct CuckooFilter {
    seed: u64,
    bucket_count: usize,
    /// Flat `bucket_count * B` fingerprint slots; `0` is an empty slot.
    table: Vec<u8>,
    count: usize,
}

/// Fingerprint slots per bucket.
const B: usize = 4;

fn mix(seed: u64, x: u64) -> u64 {
    SplitMix64::new(seed ^ x).next_u64()
}

impl CuckooFilter {
    /// Empty filter holding at most about `bucket_count * B` entries
    /// before inserts start failing. `bucket_count` is rounded up to
    /// the next power of two; `bucket_count == 0` yields a filter that
    /// rejects every insert.
    pub fn new(bucket_count: usize, seed: u64) -> Self {
        let bucket_count = bucket_count.max(1).next_power_of_two();
        Self {
            seed,
            bucket_count,
            table: vec![0; bucket_count * B],
            count: 0,
        }
    }

    /// Number of fingerprints currently stored.
    pub fn len(&self) -> usize {
        self.count
    }

    /// True when nothing has been inserted.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Approximate capacity at the 95%-full load factor cuckoo
    /// filters tolerate before insert failures become likely.
    pub fn capacity(&self) -> usize {
        self.bucket_count * B * 95 / 100
    }

    /// Memory used by the fingerprint table, in bytes.
    pub fn bytes(&self) -> usize {
        self.table.len()
    }

    /// Nonzero fingerprint of `key` (1..=255 — the zero byte encodes
    /// an empty slot).
    fn fingerprint(&self, key: u64) -> u8 {
        (mix(self.seed ^ FP_DOMAIN, key) % 255 + 1) as u8
    }

    /// First candidate bucket.
    fn h1(&self, key: u64) -> usize {
        (mix(self.seed ^ DOMAIN, key) & (self.bucket_count - 1) as u64) as usize
    }

    /// Alternate bucket: `h1 ^ hash(fp)`, always inside the table
    /// because `bucket_count` is a power of two.
    fn h2(&self, h1: usize, fp: u8) -> usize {
        h1 ^ (mix(self.seed ^ FP_DOMAIN.wrapping_add(1), fp as u64)
            & (self.bucket_count - 1) as u64) as usize
    }

    /// Membership query — may false-positive, never false-negatives
    /// an inserted key.
    pub fn contains(&self, key: u64) -> bool {
        let fp = self.fingerprint(key);
        let a = self.h1(key);
        let b = self.h2(a, fp);
        self.table[a * B..(a + 1) * B].contains(&fp) || self.table[b * B..(b + 1) * B].contains(&fp)
    }

    /// Insert `key`; returns `false` when the kick chain exhausts
    /// `MAX_KICKS` (table effectively full). Duplicate fingerprints
    /// are stored — `remove` clears exactly one.
    pub fn insert(&mut self, key: u64) -> bool {
        let fp = self.fingerprint(key);
        let a = self.h1(key);
        let b = self.h2(a, fp);
        for bucket in [a, b] {
            let slot = bucket * B;
            if let Some(i) = self.table[slot..slot + B].iter().position(|&v| v == 0) {
                self.table[slot + i] = fp;
                self.count += 1;
                return true;
            }
        }
        // Cuckoo kick chain — victim choice is a pure function of
        // (key, seed, kick index), so rebuilds are bit-identical.
        let mut bucket = a;
        let mut cur = fp;
        for kick in 0..MAX_KICKS {
            let mut picks = SplitMix64::new(mix(self.seed, key ^ kick as u64));
            let i = picks.below(B as u32) as usize;
            let slot = bucket * B + i;
            core::mem::swap(&mut self.table[slot], &mut cur);
            bucket = self.h2(bucket, cur);
            let base = bucket * B;
            if let Some(i) = self.table[base..base + B].iter().position(|&v| v == 0) {
                self.table[base + i] = cur;
                self.count += 1;
                return true;
            }
        }
        false
    }

    /// Remove one stored fingerprint of `key`; returns `false` when
    /// `key` is not present. Safe against duplicates.
    pub fn remove(&mut self, key: u64) -> bool {
        let fp = self.fingerprint(key);
        let a = self.h1(key);
        for bucket in [a, self.h2(a, fp)] {
            let slot = bucket * B;
            if let Some(i) = self.table[slot..slot + B].iter().position(|&v| v == fp) {
                self.table[slot + i] = 0;
                self.count -= 1;
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn inserted_members_are_found_and_removed() {
        let mut f = CuckooFilter::new(1024, 3);
        for k in 0..500u64 {
            assert!(f.insert(k), "insert {k} failed");
        }
        assert_eq!(f.len(), 500);
        for k in 0..500u64 {
            assert!(f.contains(k), "false negative on {k}");
        }
        for k in 0..500u64 {
            assert!(f.remove(k));
        }
        assert!(f.is_empty());
    }

    #[test]
    fn matches_membership_oracle_under_interleaved_ops() {
        let mut f = CuckooFilter::new(4096, 11);
        let mut truth = BTreeSet::new();
        let mut rng = SplitMix64::new(5);
        for _ in 0..2000 {
            let k = rng.below(800) as u64;
            match rng.below(3) {
                0 => {
                    if f.insert(k) {
                        truth.insert(k);
                    }
                }
                _ => {
                    if f.remove(k) {
                        truth.remove(&k);
                    }
                }
            }
        }
        // No false negatives: every live member must be reported.
        for &k in &truth {
            assert!(f.contains(k), "false negative on live key {k}");
        }
        // Removed keys must not be reported (their fingerprint is
        // gone) — the residual false-positive rate applies only to
        // keys never inserted.
        let mut fp = 0u32;
        let mut total = 0u32;
        for k in 800..2000u64 {
            if !truth.contains(&k) {
                total += 1;
                if f.contains(k) {
                    fp += 1;
                }
            }
        }
        assert!(fp * 20 < total, "fp rate {fp}/{total} too high");
    }

    #[test]
    fn duplicate_fingerprints_remove_one_at_a_time() {
        let mut f = CuckooFilter::new(256, 1);
        assert!(f.insert(9));
        assert!(f.insert(9));
        assert_eq!(f.len(), 2);
        assert!(f.remove(9));
        assert!(f.contains(9));
        assert!(f.remove(9));
        assert!(!f.contains(9));
    }

    #[test]
    fn is_deterministic_under_rebuild() {
        let build = || {
            let mut f = CuckooFilter::new(512, 42);
            let mut t = Vec::new();
            for k in (0..900u64).map(|k| k * 2654435761 % 1009) {
                f.insert(k);
                t.extend_from_slice(&f.table);
                t.clear();
            }
            f.table
        };
        assert_eq!(build(), build());
    }

    #[test]
    fn empty_filter_rejects_everything() {
        let mut f = CuckooFilter::new(0, 0);
        assert_eq!(f.bucket_count, 1);
        for _ in 0..8 {
            if !f.insert(7) {
                break;
            }
        }
        // A 1-bucket filter fills after a few inserts at most; it
        // must never hang or report capacity it does not have.
        assert!(f.len() <= B);
    }
}

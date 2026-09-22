//! Charikar SimHash — 64-bit locality-sensitive fingerprints.
//!
//! A SimHash fingerprint maps a weighted feature set to 64 bits such
//! that *similar* sets get *similar* fingerprints: bit `i` is the
//! majority sign of `Σ w·bit_i(hash(f))` over features `f` with
//! weight `w`. Hamming distance between fingerprints estimates the
//! cosine/dice-style distance between the underlying feature sets —
//! the classic near-duplicate detector used at web scale.
//!
//! Everything is integer-only: hashes are seeded [`Fnv1a`]
//! variants, the vote accumulates in `i64`, and `near_dupes` reports
//! pairs under a Hamming threshold — a pure function of
//! `(features, weights, seed)`.
//!
//! ```
//! use izanagi_kit::simhash::{simhash, hamming};
//! let a = simhash(&[b"alpha".as_slice(), b"beta", b"gamma"]);
//! let b = simhash(&[b"alpha".as_slice(), b"beta", b"delta"]);
//! assert!(hamming(a, b) < 64);
//! assert_eq!(hamming(a, a), 0);
//! ```

use crate::world_hash::Fnv1a;

/// Hash one feature into a 64-bit vote-sign source. Equal byte
/// strings hash identically — a repeated feature simply votes twice
/// (multiplicity IS weight in SimHash semantics).
fn feature_hash(feature: &[u8]) -> u64 {
    let mut h = Fnv1a::new();
    h.write_bytes(feature);
    h.finish()
}

/// Unweighted SimHash — each feature votes ±1 per bit.
///
/// ```
/// use izanagi_kit::simhash::simhash;
/// assert_eq!(simhash(&[]), 0); // empty set: no votes, all ties at 0
/// ```
pub fn simhash(features: &[&[u8]]) -> u64 {
    weighted_simhash(features.iter().map(|&f| (f, 1u32)))
}

/// Weighted SimHash over `(feature, weight)` pairs.
///
/// Bit `i` of the result is 1 iff `Σ_f weight[f] · (2·bit_i(h_f) − 1) > 0`
/// — positive vote wins; exact ties resolve to 0 (the canonical rule
/// keeps the fingerprint a pure function and matches the unsigned
/// half-plane convention).
pub fn weighted_simhash<'a>(features: impl Iterator<Item = (&'a [u8], u32)>) -> u64 {
    let mut votes = [0i64; 64];
    for (f, w) in features {
        let h = feature_hash(f);
        let w = w as i64;
        for (b, v) in votes.iter_mut().enumerate() {
            if h >> b & 1 == 1 {
                *v += w;
            } else {
                *v -= w;
            }
        }
    }
    let mut out = 0u64;
    for (b, &v) in votes.iter().enumerate() {
        if v > 0 {
            out |= 1 << b;
        }
    }
    out
}

/// Hamming distance between two fingerprints.
pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// All pairs `(i, j)` of indices with `hamming(fp[i], fp[j]) <= max_dist`.
///
/// Returns pairs sorted lexicographically — deterministic regardless
/// of input order dependence inside `fingerprints` is preserved (the
/// output follows the slice's own indexing).
pub fn near_dupes(fingerprints: &[u64], max_dist: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for (i, &a) in fingerprints.iter().enumerate() {
        for (j, &b) in fingerprints.iter().enumerate().skip(i + 1) {
            if hamming(a, b) <= max_dist {
                out.push((i as u32, j as u32));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Oracle: recount each bit's vote independently (different loop
    /// order — bit-major instead of feature-major).
    fn oracle(features: &[(&[u8], u32)]) -> u64 {
        let mut out = 0u64;
        for b in 0..64 {
            let mut v = 0i64;
            for &(f, w) in features {
                let h = feature_hash(f);
                v += if h >> b & 1 == 1 {
                    w as i64
                } else {
                    -(w as i64)
                };
            }
            if v > 0 {
                out |= 1 << b;
            }
        }
        out
    }

    #[test]
    fn matches_bitwise_oracle() {
        let mut rng = SplitMix64::new(0x51A4);
        for _ in 0..200 {
            let m = (rng.below(12) + 1) as usize;
            let features: Vec<(Vec<u8>, u32)> = (0..m)
                .map(|_| {
                    let len = (rng.below(8) + 1) as usize;
                    let bytes: Vec<u8> = (0..len).map(|_| rng.below(26) as u8 + b'a').collect();
                    (bytes, rng.below(4) + 1)
                })
                .collect();
            let refs: Vec<(&[u8], u32)> =
                features.iter().map(|(b, w)| (b.as_slice(), *w)).collect();
            assert_eq!(weighted_simhash(refs.iter().copied()), oracle(&refs));
        }
    }

    #[test]
    fn similarity_properties() {
        // Identical feature sets → distance 0.
        let a = simhash(&[b"red", b"green", b"blue"]);
        let b = simhash(&[b"red", b"green", b"blue"]);
        assert_eq!(hamming(a, b), 0);
        // Overlapping sets are closer than disjoint sets (weak but
        // load-bearing claim of the LSH family).
        let x = simhash(&[b"a", b"b", b"c", b"d", b"e"]);
        let y = simhash(&[b"a", b"b", b"c", b"d", b"f"]);
        let z = simhash(&[b"p", b"q", b"r", b"s", b"t"]);
        assert!(hamming(x, y) < hamming(x, z));
        // Single feature: fingerprint IS its hash (all votes one sign).
        let k = simhash(&[b"k"]);
        assert_eq!(k, feature_hash(b"k"));
        // Weight reshapes the vote when other features compete.
        let balanced = weighted_simhash([(b"a".as_slice(), 1), (b"b".as_slice(), 1)].into_iter());
        let boosted = weighted_simhash([(b"a".as_slice(), 9), (b"b".as_slice(), 1)].into_iter());
        assert!(
            hamming(boosted, feature_hash(b"a")) <= hamming(balanced, feature_hash(b"a")),
            "amplified weight pulls the fingerprint toward its hash"
        );
    }

    #[test]
    fn near_dupes_finds_close_pairs() {
        let fps = [
            simhash(&[b"doc1".as_slice(), b"shared"]),
            simhash(&[b"doc1".as_slice(), b"shared"]),
            simhash(&[b"totally".as_slice(), b"different".as_slice(), b"words"]),
        ];
        let pairs = near_dupes(&fps, 10);
        assert!(pairs.contains(&(0, 1)), "identical docs must pair");
        assert!(pairs.iter().all(|&(i, j)| i < j));
        // Sorted, no duplicates.
        let s: BTreeSet<_> = pairs.iter().copied().collect();
        assert_eq!(s.len(), pairs.len());
        let mut sorted = pairs.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, pairs);
        // Threshold 0 keeps only exact matches.
        let exact = near_dupes(&fps, 0);
        assert!(exact
            .iter()
            .all(|&(i, j)| fps[i as usize] == fps[j as usize]));
    }
}

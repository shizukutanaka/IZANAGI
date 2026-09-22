//! MinHash signatures — "how similar are these two *sets*?" in `O(k)`
//! bytes instead of storing them. Near-duplicate detection over seeds
//! and corpora (`markov` outputs, generated maps, replay traces), the
//! set-similarity complement of [`kmv`](crate::kmv)'s cardinality
//! estimate. The signature is a pure function of `(seed, k, set)`:
//! position `i` holds the minimum of `hash(seed, i, x)` over all `x`.
//!
//! ```
//! use izanagi_kit::minhash::{estimate, signature};
//! let a: Vec<Vec<u8>> = (0..100u64).map(|x| x.to_le_bytes().to_vec()).collect();
//! let b: Vec<Vec<u8>> = (50..150u64).map(|x| x.to_le_bytes().to_vec()).collect();
//! let sa = signature(64, 0x5EED, &a);
//! let sb = signature(64, 0x5EED, &b);
//! let sim = estimate(&sa, &sb);
//! // True Jaccard of the ranges is 50/150 ≈ 333 permille.
//! assert!(sim >= 250 && sim <= 420, "permille = {sim}");
//! ```

use crate::world_hash::Fnv1a;

/// MinHash signature: `sig[i]` = minimum hash of the set under seed `i`.
pub type Signature = Vec<u64>;

/// `k`-wide MinHash signature of the element set (each element an
/// arbitrary byte vector). Empty sets → all `u64::MAX`.
pub fn signature(k: usize, seed: u64, items: &[Vec<u8>]) -> Signature {
    let mut sig = vec![u64::MAX; k];
    for b in items {
        for (i, s) in sig.iter_mut().enumerate() {
            let mut h = Fnv1a::new();
            h.write_u64(seed);
            h.write_u64(i as u64);
            h.write_bytes(b);
            let v = h.finish();
            if v < *s {
                *s = v;
            }
        }
    }
    sig
}

/// Jaccard estimate in permille `0..=1000` — the fraction of positions
/// where the signatures agree (MinHash's estimator of `|A∩B|/|A∪B|`).
/// Empty or mismatched signatures return `0` (nothing comparable).
pub fn estimate(a: &Signature, b: &Signature) -> u64 {
    if a.is_empty() || a.len() != b.len() {
        return 0;
    }
    let matches = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count() as u64;
    matches * 1000 / a.len() as u64
}

/// MinHash of `A ∪ B` — position-wise minimum of the signatures.
/// `None` on length mismatch or empty signatures.
pub fn union(a: &Signature, b: &Signature) -> Option<Signature> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    Some(a.iter().zip(b.iter()).map(|(&x, &y)| x.min(y)).collect())
}

/// True Jaccard × 1000 over two sorted dedup'ed element vectors —
/// the verification oracle (kept public so callers can measure
/// estimator error on small sets).
pub fn jaccard_exact(a: &[Vec<u8>], b: &[Vec<u8>]) -> u64 {
    let (mut i, mut j, mut inter, mut uni) = (0usize, 0usize, 0u64, 0u64);
    while i < a.len() || j < b.len() {
        uni += 1;
        if j >= b.len() || (i < a.len() && a[i] < b[j]) {
            i += 1;
        } else if i >= a.len() || b[j] < a[i] {
            j += 1;
        } else {
            inter += 1;
            i += 1;
            j += 1;
        }
    }
    (inter * 1000).checked_div(uni).unwrap_or(1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn set_of(r: &mut SplitMix64, n: usize, space: u32) -> BTreeSet<Vec<u8>> {
        let mut s = BTreeSet::new();
        while s.len() < n {
            s.insert((r.below(space) as u64).to_le_bytes().to_vec());
        }
        s
    }

    #[test]
    fn estimate_tracks_exact_and_union() {
        let mut rng = SplitMix64::new(0x5EED_CAFE);
        for trial in 0..40 {
            let k = [16usize, 64, 128][trial % 3];
            let na = (rng.below(200) + 20) as usize;
            let nb = (rng.below(200) + 20) as usize;
            let a = set_of(&mut rng, na, 2000);
            // b shares half of a plus fresh elements.
            let mut b: BTreeSet<Vec<u8>> = a.iter().take(na / 2).cloned().collect();
            while b.len() < nb {
                b.insert(((2000 + rng.below(2000)) as u64).to_le_bytes().to_vec());
            }
            let av: Vec<Vec<u8>> = a.iter().cloned().collect();
            let bv: Vec<Vec<u8>> = b.iter().cloned().collect();
            let exact = jaccard_exact(&av, &bv);
            let sa = signature(k, 0xBEEF, &av);
            let sb = signature(k, 0xBEEF, &bv);
            let est = estimate(&sa, &sb);
            // MinHash stddev ≈ 500/√k permille; assert err·√k ≤ 3000
            // (≈6σ) in pure integers: err²·k ≤ 9×10⁶.
            let err = (est as i64 - exact as i64).unsigned_abs();
            assert!(
                err * err * (k as u64) <= 9_000_000,
                "est={est} exact={exact} k={k} err={err}"
            );
            // Union: signature of merged sets == per-position min.
            let mut u = a.clone();
            u.extend(b.iter().cloned());
            let uv: Vec<Vec<u8>> = u.iter().cloned().collect();
            let su = signature(k, 0xBEEF, &uv);
            assert_eq!(union(&sa, &sb).unwrap_or_default(), su);
        }
        // Edge cases.
        let empty: Vec<Vec<u8>> = Vec::new();
        assert_eq!(
            estimate(&signature(0, 1, &empty), &signature(0, 1, &empty)),
            0
        );
        assert_eq!(
            estimate(&signature(8, 1, &empty), &signature(4, 1, &empty)),
            0
        );
        assert!(union(&signature(8, 1, &empty), &signature(4, 1, &empty)).is_none());
        // Identical sets → 1000; empty pair → oracle says 1000.
        let av: Vec<Vec<u8>> = (0..50u64).map(|x| x.to_le_bytes().to_vec()).collect();
        let s = signature(64, 3, &av);
        assert_eq!(estimate(&s, &s), 1000);
        assert_eq!(jaccard_exact(&[], &[]), 1000);
    }
}

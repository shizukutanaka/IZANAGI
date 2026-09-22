//! GF(2) linear basis over `u64` — the xor analogue of Gaussian
//! elimination.
//!
//! A basis is stored in row-echelon form: `basis[b]` (when present) has
//! its highest set bit at `b`, and full reduction additionally removes
//! that bit from every *other* basis vector, which is what makes `kth`
//! enumeration order work.
//!
//! Deterministic: insertion order does not matter — the reduced basis
//! is a canonical form of the span, so `XorBasis::new` is a pure
//! function of the input *multiset's* distinct values.
//!
//! ```
//! use izanagi_kit::xorbasis::XorBasis;
//! let b = XorBasis::new(&[3, 5, 6]);
//! assert_eq!(b.rank(), 2);       // 6 = 3^5 is dependent; {3,5} spans
//! assert!(b.contains(6));
//! assert!(!b.contains(1));
//! assert_eq!(b.max_xor(0), 6);   // span is {0,3,5,6}
//! assert_eq!(b.kth(2), Some(5));
//! ```

/// Reduced xor basis. `basis[i]` has MSB at bit `i` and (after
/// reduction) no other basis vector has bit `i` set.
pub struct XorBasis {
    basis: [u64; 64],
    /// Reduced basis vectors sorted by MSB — canonical list.
    vec: Vec<u64>,
}

impl XorBasis {
    /// Builds the reduced basis of `vals`. `O(n·64)`.
    ///
    /// Maintains the invariant that the basis is always in reduced
    /// row-echelon form: each vector owns its MSB exclusively. Because
    /// every incoming `x` is pre-eliminated against all existing
    /// pivots, `x` has no bit set at any pivot position — clearing
    /// `x`'s MSB out of the other basis vectors (the `for j` loop)
    /// therefore cannot create pivot bits anywhere else.
    pub fn new(vals: &[u64]) -> Self {
        let mut basis = [0u64; 64];
        for &x0 in vals {
            let mut x = x0;
            // Eliminate x against existing pivots (top-down).
            for b in (0..64).rev() {
                if x >> b & 1 == 1 && basis[b] != 0 {
                    x ^= basis[b];
                }
            }
            if x == 0 {
                continue; // linearly dependent
            }
            let hb = 63 - x.leading_zeros() as usize;
            basis[hb] = x;
            for (j, b) in basis.iter_mut().enumerate() {
                if j != hb && *b >> hb & 1 == 1 {
                    *b ^= x;
                }
            }
        }
        let mut vec: Vec<u64> = basis.iter().copied().filter(|&b| b != 0).collect();
        vec.sort_unstable();
        XorBasis { basis, vec }
    }

    /// Dimension of the span — number of independent vectors.
    pub fn rank(&self) -> u32 {
        self.vec.len() as u32
    }

    /// Size of the spanned set: `2^rank`, or `u64::MAX`-cap… no —
    /// saturates at `2^64 - 1` when `rank == 64` since `u64` cannot
    /// hold `2^64`; callers should treat `rank() == 64` as "spans
    /// everything" via `contains` instead.
    pub fn span_size(&self) -> u64 {
        if self.rank() == 64 {
            u64::MAX
        } else {
            1u64 << self.rank()
        }
    }

    /// Is `x` xor-representable by the input multiset?
    pub fn contains(&self, mut x: u64) -> bool {
        for b in (0..64).rev() {
            if x >> b & 1 == 1 {
                if self.basis[b] == 0 {
                    return false;
                }
                x ^= self.basis[b];
            }
        }
        x == 0
    }

    /// Maximum `x ⊕ seed` over the span — the basis greedy
    /// (set each result bit when possible, MSB to LSB).
    pub fn max_xor(&self, seed: u64) -> u64 {
        let mut x = seed;
        for b in (0..64).rev() {
            let cand = x ^ self.basis[b];
            if cand > x {
                x = cand;
            }
        }
        x
    }

    /// `k`-th smallest element of the span (`k = 0` → `0`).
    /// With the reduced basis sorted ascending, the enumeration maps
    /// `k`'s bit `i` to basis vector `vec[i]` — the RREF guarantee that
    /// each `vec[i]` alone owns its MSB makes this order exact.
    /// `None` when `k >= span_size` (or `rank == 64`).
    pub fn kth(&self, k: u64) -> Option<u64> {
        if k >= self.span_size() {
            return None;
        }
        let mut x = 0u64;
        let mut kk = k;
        let mut i = 0usize;
        while kk != 0 {
            if kk & 1 == 1 {
                x ^= self.vec[i];
            }
            kk >>= 1;
            i += 1;
        }
        Some(x)
    }

    /// The basis vectors in MSB order — reduced row-echelon form.
    /// Exposed mainly for debugging and determinism audits.
    pub fn vectors(&self) -> &[u64] {
        &self.vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Oracle: the full xor-closure of the input (sorted set).
    fn oracle_span(vals: &[u64]) -> BTreeSet<u64> {
        let mut span = BTreeSet::new();
        span.insert(0u64);
        for &v in vals {
            let cur: Vec<u64> = span.iter().copied().collect();
            for x in cur {
                span.insert(x ^ v);
            }
        }
        span
    }

    /// Oracle: GF(2) rank via free-variable elimination.
    fn oracle_rank(vals: &[u64]) -> u32 {
        let mut vs: Vec<u64> = vals.to_vec();
        let mut rank = 0;
        for b in (0..64).rev() {
            if rank >= vs.len() {
                break;
            }
            // find pivot with bit b among the not-yet-used rows
            if let Some(p) = vs[rank..].iter().position(|&v| v >> b & 1 == 1) {
                vs.swap(rank, rank + p);
                for i in 0..vs.len() {
                    if i != rank && vs[i] >> b & 1 == 1 {
                        vs[i] ^= vs[rank];
                    }
                }
                rank += 1;
            }
        }
        rank as u32
    }

    #[test]
    fn all_queries_match_oracle_span() {
        let mut rng = SplitMix64::new(0xBA51);
        for _ in 0..200 {
            let n = (rng.below(12) + 1) as usize;
            let width = rng.below(4) + 1; // small widths → collisions
            let vals: Vec<u64> = (0..n).map(|_| rng.below(1 << width) as u64).collect();
            let b = XorBasis::new(&vals);
            let span = oracle_span(&vals);
            assert_eq!(b.rank(), oracle_rank(&vals));
            assert_eq!(b.span_size(), span.len() as u64);
            // contains: every span member and random outsiders
            for &x in &span {
                assert!(b.contains(x), "{x} of {vals:?}");
            }
            for _ in 0..20 {
                let x = rng.below(1 << (width + 2)) as u64;
                assert_eq!(b.contains(x), span.contains(&x), "{x} of {vals:?}");
            }
            // max_xor(seed) = max over span elements xored with seed.
            let seed = rng.below(1 << width) as u64;
            let want = span.iter().map(|&x| x ^ seed).max().unwrap();
            assert_eq!(b.max_xor(seed), want);
            // kth enumerates the sorted span exactly.
            for (k, &want) in span.iter().enumerate() {
                assert_eq!(b.kth(k as u64), Some(want), "k={k} vals={vals:?}");
            }
            assert_eq!(b.kth(span.len() as u64), None);
        }
    }

    #[test]
    fn input_order_independence() {
        let mut rng = SplitMix64::new(0x0F0F);
        for _ in 0..100 {
            let n = (rng.below(10) + 1) as usize;
            let vals: Vec<u64> = (0..n).map(|_| rng.below(64) as u64).collect();
            let base = XorBasis::new(&vals);
            // Permute deterministically.
            let mut shuffled = vals.clone();
            for i in (1..n).rev() {
                let j = rng.below(i as u32 + 1) as usize;
                shuffled.swap(i, j);
            }
            let alt = XorBasis::new(&shuffled);
            assert_eq!(base.vectors(), alt.vectors());
        }
    }

    #[test]
    fn known_cases() {
        let b = XorBasis::new(&[]);
        assert_eq!(b.rank(), 0);
        assert_eq!(b.span_size(), 1);
        assert!(b.contains(0));
        assert!(!b.contains(1));
        assert_eq!(b.max_xor(0), 0);
        assert_eq!(b.kth(0), Some(0));
        assert_eq!(b.kth(1), None);

        // Full rank over GF(2)^3: {1,2,4}.
        let b = XorBasis::new(&[1, 2, 4]);
        assert_eq!(b.rank(), 3);
        assert_eq!(b.span_size(), 8);
        for x in 0..8u64 {
            assert!(b.contains(x));
            assert_eq!(b.kth(x), Some(x));
        }
        assert_eq!(b.max_xor(0), 7);

        // Dependent input: {2,4,6} — 6 = 2^4 → rank 2.
        let b = XorBasis::new(&[2, 4, 6]);
        assert_eq!(b.rank(), 2);
        assert_eq!(b.span_size(), 4);
        assert!(b.contains(6));
        assert!(!b.contains(1));
        // Sorted span: {0,2,4,6}.
        assert_eq!(b.kth(3), Some(6));
    }
}

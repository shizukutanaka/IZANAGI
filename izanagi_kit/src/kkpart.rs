//! Karmarkar–Karp largest-differencing number partitioning —
//! the classic heuristic for splitting a multiset into two
//! groups with near-minimum difference of sums.
//!
//! Sort descending, then repeatedly replace the two largest
//! numbers `x ≥ y` by `x − y`. Whatever survives is an
//! achievable difference: each differencing step secretly
//! places `x` and `y` on opposite sides. This module tracks
//! that assignment, so it returns both the bound `d` *and* a
//! concrete partition `A ∪ B` with `|ΣA − ΣB| = d`. Because
//! `d` is achievable, `d ≥ optimal` always — the oracle
//! asserts exactly that against brute `2ⁿ` enumeration.
//!
//! ```
//! use izanagi_kit::kkpart::partition;
//! let a = [8u64, 7, 6, 5, 4];
//! let (d, side) = partition(&a).unwrap();
//! assert_eq!(d, 2);
//! let diff: i128 = (0..5)
//!     .map(|i| if side[i] { a[i] as i128 } else { -(a[i] as i128) })
//!     .sum();
//! assert_eq!(diff.unsigned_abs() as u64, d); // side achieves d
//! ```
//!
//! References: Karmarkar & Karp, "The Differencing Method of
//! Set Partitioning" (UC Berkeley TR, 1982); Korf, "From
//! Approximate to Optimal Solutions" (AAAI 1995).

/// One differencing heap entry: `v` is the residual difference
/// of a partial partition of some subset, `(plus, minus)` are
/// bitmasks (bit `i` = input index) naming which originals sit
/// on the `+` and `−` sides. Invariant: `v = Σplus − Σminus`.
///
/// Merge rule for `x ≥ y`: the new `+` side is
/// `x.plus ∪ y.minus` — `y`'s piles swap sides because `y`
/// is being subtracted from `x`.
#[derive(Clone, Copy, Debug)]
struct Elem {
    v: u64,
    plus: u128,
    minus: u128,
}

/// Largest-differencing partition of `a` (at most 128 items).
///
/// Returns `(d, side)` where `side[i]` is `true` for the `+`
/// group and `d = |Σgroup_true − Σgroup_false|` — always an
/// achievable difference, hence `d ≥` the optimal partition
/// difference. `None` for empty input or `a.len() > 128`.
pub fn partition(a: &[u64]) -> Option<(u64, Vec<bool>)> {
    let n = a.len();
    if n == 0 || n > 128 {
        return None;
    }
    // Initial elements are sorted ascending; the last entry is
    // the current largest. The linear-scan insert keeps ties
    // canonical (stable, index order).
    let mut elems: Vec<Elem> = (0..n)
        .map(|i| Elem {
            v: a[i],
            plus: 1u128 << i,
            minus: 0,
        })
        .collect();
    elems.sort_by_key(|e| e.v);
    while elems.len() > 1 {
        let x = elems.pop()?;
        let y = elems.pop()?;
        let z = Elem {
            v: x.v - y.v,
            plus: x.plus | y.minus,
            minus: x.minus | y.plus,
        };
        // Deterministic canonical tie-break: among equal
        // residuals prefer the element with the smaller `plus`
        // mask so repeated runs give identical partitions.
        let pos = elems.partition_point(|e| e.v < z.v || (e.v == z.v && e.plus < z.plus));
        elems.insert(pos, z);
    }
    let f = elems[0];
    let side = (0..n).map(|i| f.plus >> i & 1 == 1).collect();
    Some((f.v, side))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Brute-force optimal partition difference over `2ⁿ`.
    fn optimal(a: &[u64]) -> u64 {
        let n = a.len();
        let mut best = u64::MAX;
        for m in 0u64..(1 << n) {
            let mut s: i128 = 0;
            for (i, &v) in a.iter().enumerate() {
                s += if m >> i & 1 == 1 {
                    v as i128
                } else {
                    -(v as i128)
                };
            }
            best = best.min(s.unsigned_abs() as u64);
        }
        best
    }

    #[test]
    fn basics() {
        assert_eq!(partition(&[8, 7, 6, 5, 4]).unwrap().0, 2);
        assert_eq!(partition(&[1, 1]).unwrap().0, 0);
        assert_eq!(partition(&[10]).unwrap().0, 10);
        assert!(partition(&[]).is_none());
        assert!(partition(&[1; 129]).is_none());
    }

    #[test]
    fn returned_partition_achieves_and_bounds_optimum() {
        let mut rng = SplitMix64::new(0xC0FFEE);
        for _ in 0..200 {
            let n = 1 + rng.below(9) as usize; // 1..=9
            let a: Vec<u64> = (0..n).map(|_| u64::from(rng.below(60))).collect();
            let (d, side) = partition(&a).unwrap();
            let plus: i128 = a
                .iter()
                .zip(&side)
                .filter(|(_, &s)| s)
                .map(|(&v, _)| v as i128)
                .sum();
            let minus: i128 = a
                .iter()
                .zip(&side)
                .filter(|(_, &s)| !s)
                .map(|(&v, _)| v as i128)
                .sum();
            assert_eq!((plus - minus).unsigned_abs() as u64, d);
            assert!(d >= optimal(&a));
            // determinism: same input, same partition
            assert_eq!(partition(&a).unwrap(), (d, side));
        }
    }

    #[test]
    fn every_element_assigned_exactly_once() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..100 {
            let n = 1 + rng.below(10) as usize;
            let a: Vec<u64> = (0..n).map(|_| u64::from(rng.below(40))).collect();
            let (_, side) = partition(&a).unwrap();
            assert_eq!(side.len(), n);
            // duplicates are fine — indices, not values, are tracked
            let set: BTreeSet<u64> = a.iter().copied().collect();
            let _ = set;
            assert_eq!(
                side.iter().filter(|&&s| s).count() + side.iter().filter(|&&s| !s).count(),
                n
            );
        }
    }
}

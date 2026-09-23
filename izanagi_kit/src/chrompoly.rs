//! Exact chromatic counting — `P_G(k)`, the number of
//! proper `k`-colorings of a graph, via the
//! deletion–contraction recurrence
//! `P(G) = P(G − e) − P(G / e)`.
//!
//! `P_G` is a polynomial in `k` with integer coefficients;
//! [`count`] evaluates it at a concrete `k` without ever
//! building the coefficient vector (the recursion on graphs
//! is the computation), and [`count_poly`] evaluates over
//! `BigInt` so `k` is unbounded. [`chromatic`] is the
//! smallest `k` with `P_G(k) > 0`.
//!
//! Graphs are vertex sets `0..n` plus an edge list —
//! `chromatic`/`count` validate edges into range. Intended
//! for *small* graphs (the recurrence is exponential;
//! `n ≤ ~15` in tests); dense graphs contract quickly.
//!
//! ```
//! use izanagi_kit::chrompoly::{count, chromatic};
//! // path P3 on 3 vertices: P_G(k) = k·(k−1)² → k=2 gives 2
//! assert_eq!(count(3, &[(0,1),(1,2)], 2), Some(2));
//! // the triangle needs 3 colors
//! assert_eq!(count(3, &[(0,1),(1,2),(0,2)], 2), Some(0));
//! assert_eq!(chromatic(3, &[(0,1),(1,2),(0,2)]), Some(3));
//! ```
//!
//! References: Birkhoff–Lewis deletion–contraction
//! (Read's *An introduction to chromatic polynomials*,
//! JCT 1968).

use crate::bigint::BigInt;

/// Canonical edge: `u < v`.
fn norm_edge(u: u32, v: u32) -> (u32, u32) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

/// Count proper `k`-colorings — `0` when `k = 0` and `n > 0`,
/// `1` for the empty graph `n = 0` regardless of `k`.
/// `None` on malformed input (edge endpoint ≥ `n` or a loop)
/// or when the count exceeds `i128` (use [`count_poly`]).
pub fn count(n: u32, edges: &[(u32, u32)], k: u64) -> Option<u128> {
    let b = count_big(n, edges, &BigInt::from_i128(i128::from(k)))?;
    b.to_i128().map(|x| x as u128)
}

/// `count` with arbitrary-precision `k` — `None` on
/// malformed input.
pub fn count_poly(n: u32, edges: &[(u32, u32)], k: &BigInt) -> Option<BigInt> {
    count_big(n, edges, k)
}

fn count_big(n: u32, edges: &[(u32, u32)], k: &BigInt) -> Option<BigInt> {
    // validate
    for &(u, v) in edges {
        if u >= n || v >= n || u == v {
            return None;
        }
    }
    // dedupe edges canonically
    let es: std::collections::BTreeSet<(u32, u32)> =
        edges.iter().map(|&(u, v)| norm_edge(u, v)).collect();
    let es_vec: Vec<(u32, u32)> = es.iter().copied().collect();
    Some(rec(n, &es_vec, k))
}

fn rec(n: u32, edges: &[(u32, u32)], k: &BigInt) -> BigInt {
    let Some(&(u, v)) = edges.iter().min() else {
        // edgeless on n vertices: k^n
        return k.pow(u64::from(n));
    };
    // G − e
    let deleted: Vec<(u32, u32)> = edges.iter().copied().filter(|&e| e != (u, v)).collect();
    // G / e — contract v into u (u < v): map x ↦ x for x < v,
    // v ↦ u, x > v ↦ x−1; dedupe and drop loops
    let mut contracted: std::collections::BTreeSet<(u32, u32)> = std::collections::BTreeSet::new();
    for &(a, b) in edges {
        if (a, b) == (u, v) {
            continue;
        }
        let na = if a == v {
            u
        } else if a > v {
            a - 1
        } else {
            a
        };
        let nb = if b == v {
            u
        } else if b > v {
            b - 1
        } else {
            b
        };
        if na != nb {
            contracted.insert(norm_edge(na, nb));
        }
    }
    let contracted_vec: Vec<(u32, u32)> = contracted.into_iter().collect();
    rec(n, &deleted, k).sub(&rec(n - 1, &contracted_vec, k))
}

/// The chromatic number `χ(G)` — smallest `k` with
/// `P_G(k) > 0` (`0` for `n = 0`). `None` on malformed
/// input.
pub fn chromatic(n: u32, edges: &[(u32, u32)]) -> Option<u64> {
    // reuse count's validation path
    count_big(n, edges, &BigInt::from_i64(0))?;
    let mut k = 0u64;
    loop {
        let c = count_big(n, edges, &BigInt::from_i128(i128::from(k)))?;
        if c != BigInt::from_i64(0) {
            return Some(k);
        }
        k += 1;
        if k > u64::from(n) + 1 {
            // P_G(k) > 0 for k ≥ n always (greedy) — unreachable
            return Some(n as u64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        // edgeless
        assert_eq!(count(3, &[], 2), Some(8));
        // path P3: k·(k−1)²
        assert_eq!(count(3, &[(0, 1), (1, 2)], 2), Some(2));
        assert_eq!(count(3, &[(0, 1), (1, 2)], 3), Some(12));
        // triangle K3: k(k−1)(k−2)
        let tri = [(0, 1), (1, 2), (0, 2)];
        assert_eq!(count(3, &tri, 2), Some(0));
        assert_eq!(count(3, &tri, 3), Some(6));
        // K4
        let k4: Vec<(u32, u32)> = (0..4)
            .flat_map(|i| (i + 1..4).map(move |j| (i, j)))
            .collect();
        assert_eq!(count(4, &k4, 4), Some(24));
        assert_eq!(chromatic(4, &k4), Some(4));
        assert_eq!(chromatic(3, &tri), Some(3));
        assert_eq!(chromatic(0, &[]), Some(0));
        // arbitrary-precision count agrees at small k
        let big = count_poly(4, &k4, &BigInt::from_i64(4));
        assert_eq!(big, Some(BigInt::from_i64(24)));
        // malformed
        assert_eq!(count(3, &[(0, 3)], 2), None);
        assert_eq!(count_poly(3, &[(0, 3)], &BigInt::from_i64(2)), None);
        assert_eq!(count(3, &[(1, 1)], 2), None);
        assert_eq!(chromatic(3, &[(0, 3)]), None);
    }

    /// Brute-force oracle: count proper k-colorings by
    /// enumerating all k^n assignments on n ≤ 7.
    fn brute(n: u32, edges: &[(u32, u32)], k: u64) -> u128 {
        let mut c = 0u128;
        let mut assign = vec![0u64; n as usize];
        let total = k.pow(n);
        'outer: for mut code in 0..total {
            for slot in assign.iter_mut() {
                *slot = code % k;
                code /= k;
            }
            for &(u, v) in edges {
                if assign[u as usize] == assign[v as usize] {
                    continue 'outer;
                }
            }
            c += 1;
        }
        c
    }

    #[test]
    fn oracle_vs_brute() {
        let mut rng = SplitMix64::new(0xC40);
        for _ in 0..400 {
            let n = 1 + rng.below(7);
            let k = 1 + rng.below(4) as u64;
            let mut edges = Vec::new();
            for u in 0..n {
                for v in u + 1..n {
                    if rng.coin(1, 3) {
                        edges.push((u, v));
                    }
                }
            }
            assert_eq!(count(n, &edges, k), Some(brute(n, &edges, k)));
            // chromatic agrees with brute search
            let mut chi = 0u64;
            while chi <= u64::from(n) {
                if brute(n, &edges, chi) > 0 {
                    break;
                }
                chi += 1;
            }
            assert_eq!(chromatic(n, &edges), Some(chi));
        }
    }

    /// The polynomial identity: P_C4(k) = k(k−1)(k²−3k+3) —
    /// the cycle C4's closed form evaluated at several k.
    #[test]
    fn cycle_c4_closed_form() {
        let c4 = [(0, 1), (1, 2), (2, 3), (3, 0)];
        for k in 0u64..6 {
            let kk = k as u128;
            let want = kk * kk.saturating_sub(1) * (kk * kk + 3 - 3 * kk);
            assert_eq!(count(4, &c4, k), Some(want));
        }
    }
}

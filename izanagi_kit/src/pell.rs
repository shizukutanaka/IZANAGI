//! Pell equations `x² − d·y² = ±1` — the continued-fraction
//! machinery over [`BigInt`], since solutions grow
//! exponentially (the smallest solution of `d = 61` is
//! already a 33-digit pair).
//!
//! The CF of `√d` is computed exactly by the standard
//! `(m, d, a)` recurrence — every term is an integer, no
//! floats anywhere — and the fundamental solution is the
//! first convergent `p/q` with `p² − d·q² = 1`. The full
//! positive solution set is `(x₁ + y₁√d)^k`, which
//! [`power`] produces via exact `Z[√d]` multiplication.
//! [`negative`] finds `x² − d·y² = −1` (it exists iff the
//! CF period of `√d` is odd).
//!
//! ```
//! use izanagi_kit::pell::{solve, negative, power};
//! use izanagi_kit::bigint::BigInt;
//! // x² − 2y² = 1: fundamental solution (3, 2)
//! let (x, y) = solve(2).unwrap();
//! assert_eq!(x, BigInt::from_i64(3));
//! assert_eq!(y, BigInt::from_i64(2));
//! // x² − 5y² = −1 → (2, 1) since period of √5 is odd
//! assert!(negative(5).is_some());
//! ```
//!
//! References: the CF algorithm for `√d` and the theorem
//! that Pell solutions are convergents of `√d` — standard
//! (cf. *An Introduction to the Theory of Numbers*,
//! Hardy & Wright, §14.5; Library-Checker style
//! `surd_cf` implementations on Qiita/Zenn).

use crate::bigint::BigInt;

/// Integer square root `⌊√n⌋` for `n ≤ u64::MAX`.
pub fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// The continued fraction of `√d` for nonsquare `d`:
/// `(a₀, period)` with `√d = [a₀; p₁, p₂, …, pₖ]`.
/// `None` when `d` is a perfect square or `d < 2`.
pub fn surd_cf(d: u64) -> Option<(u64, Vec<u64>)> {
    if d < 2 || d > u64::from(u32::MAX) {
        // `d ≤ u32::MAX` keeps every `m²` inside u64 —
        // beyond that the (m,d,a) recurrence could overflow
        return None;
    }
    let a0 = isqrt(d);
    if a0 * a0 == d {
        return None; // perfect square — no Pell problem
    }
    // (m_k, d_k, a_k) recurrence; the period ends exactly
    // when a_k = 2·a₀ (textbook theorem for √d CFs)
    let mut period = Vec::new();
    let (mut m, mut dd, mut a) = (0u64, 1u64, a0);
    loop {
        m = dd * a - m;
        dd = (d - m * m) / dd;
        a = (a0 + m) / dd;
        period.push(a);
        if a == 2 * a0 {
            break;
        }
        if period.len() as u64 > 4 * a0 + 4 {
            // period of √d is < 2·a₀ — unreachable for
            // nonsquare d, honest bail instead of hanging
            break;
        }
    }
    Some((a0, period))
}

/// Convergents `p_k/q_k` of `√d` as `BigInt` pairs — enough
/// terms to cover `steps` CF terms.
fn convergents(d: u64, steps: usize) -> Option<Vec<(BigInt, BigInt)>> {
    let (a0, period) = surd_cf(d)?;
    let mut convs = Vec::new();
    let (mut pm2, mut pm1) = (BigInt::from_i64(1), BigInt::from_i64(a0 as i64));
    let (mut qm2, mut qm1) = (BigInt::from_i64(0), BigInt::from_i64(1));
    convs.push((pm1.clone(), qm1.clone()));
    for i in 0..steps {
        let a = period[i % period.len()];
        let a_big = BigInt::from_i64(a as i64);
        let pm = a_big.mul(&pm1).add(&pm2);
        let qm = a_big.mul(&qm1).add(&qm2);
        convs.push((pm.clone(), qm.clone()));
        pm2 = pm1;
        pm1 = pm;
        qm2 = qm1;
        qm1 = qm;
    }
    Some(convs)
}

fn is_one(b: &BigInt) -> bool {
    *b == BigInt::from_i64(1)
}

fn is_neg_one(b: &BigInt) -> bool {
    *b == BigInt::from_i64(-1)
}

fn pell_residue(x: &BigInt, y: &BigInt, d: u64) -> BigInt {
    let db = BigInt::from_i64(d as i64);
    x.mul(x).sub(&db.mul(&y.mul(y)))
}

/// The fundamental positive solution `(x₁, y₁)` of
/// `x² − d·y² = 1` — the first convergent of `√d` whose
/// residue is `+1` (guaranteed to appear within one period
/// if the period is even, two if odd). `None` for perfect
/// squares or `d < 2` — the equation has no nontrivial
/// solution there.
pub fn solve(d: u64) -> Option<(BigInt, BigInt)> {
    let (a0, period) = surd_cf(d)?;
    let _ = a0;
    // +1 appears within the first 2·period convergents
    let convs = convergents(d, 2 * period.len() + 2)?;
    for (p, q) in &convs {
        if is_one(&pell_residue(p, q, d)) {
            return Some((p.clone(), q.clone()));
        }
    }
    None
}

/// The fundamental solution of `x² − d·y² = −1`, which
/// exists iff the CF period of `√d` is odd. `None` for
/// perfect squares, `d < 2`, or even period.
pub fn negative(d: u64) -> Option<(BigInt, BigInt)> {
    let (_, period) = surd_cf(d)?;
    let convs = convergents(d, 2 * period.len() + 2)?;
    for (p, q) in &convs {
        if is_neg_one(&pell_residue(p, q, d)) {
            return Some((p.clone(), q.clone()));
        }
    }
    None
}

/// `(x + y√d)^k` in `Z[√d]` — the `k`-th power of a Pell
/// solution `(x, y)` is again a solution (`x²−dy²=±1`
/// multiplies). Returns the `(x', y')` coefficients.
pub fn power(x: &BigInt, y: &BigInt, d: u64, k: u64) -> (BigInt, BigInt) {
    let db = BigInt::from_i64(d as i64);
    let (mut rx, mut ry) = (BigInt::from_i64(1), BigInt::from_i64(0));
    let (mut bx, mut by) = (x.clone(), y.clone());
    let mut kk = k;
    while kk > 0 {
        if kk & 1 == 1 {
            // (rx + ry√d)(bx + by√d) = (rx·bx + d·ry·by, rx·by + ry·bx)
            let nx = rx.mul(&bx).add(&db.mul(&ry.mul(&by)));
            let ny = rx.mul(&by).add(&ry.mul(&bx));
            rx = nx;
            ry = ny;
        }
        let nx = bx.mul(&bx).add(&db.mul(&by.mul(&by)));
        let ny = bx.mul(&by).add(&by.mul(&bx));
        bx = nx;
        by = ny;
        kk >>= 1;
    }
    (rx, ry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(17), 4);
        assert_eq!(isqrt(u64::MAX), 0xFFFF_FFFF);
        // √2 = [1; 2, 2, 2, …]
        assert_eq!(surd_cf(2), Some((1, vec![2])));
        // √3 = [1; 1, 2]
        assert_eq!(surd_cf(3), Some((1, vec![1, 2])));
        // √7 = [2; 1, 1, 1, 4]
        assert_eq!(surd_cf(7), Some((2, vec![1, 1, 1, 4])));
        // perfect squares and d<2 rejected
        assert_eq!(surd_cf(4), None);
        assert_eq!(surd_cf(1), None);
    }

    /// Known fundamental solutions (OEIS A033313/A033317):
    /// d=2:(3,2), d=3:(2,1), d=5:(9,4), d=6:(5,2), d=7:(8,3).
    #[test]
    fn known_fundamentals() {
        let cases = [
            (2u64, 3i64, 2i64),
            (3, 2, 1),
            (5, 9, 4),
            (6, 5, 2),
            (7, 8, 3),
            (10, 19, 6),
            (11, 10, 3),
            (13, 649, 180),
        ];
        for (d, ex, ey) in cases {
            let (x, y) = solve(d).unwrap();
            assert_eq!(
                (x.to_i128().unwrap(), y.to_i128().unwrap()),
                (i128::from(ex), i128::from(ey)),
                "d = {d}"
            );
        }
        // d=61: the famous huge fundamental (1766319049, 226153980)
        let (x, y) = solve(61).unwrap();
        assert_eq!(x, BigInt::from_i64(1766319049));
        assert_eq!(y, BigInt::from_i64(226153980));
        assert!(is_one(&pell_residue(&x, &y, 61)));
    }

    /// Powers of the fundamental stay solutions — the group
    /// law of Z[√d] units, verified residue-exactly.
    #[test]
    fn powers_are_solutions() {
        let mut rng = SplitMix64::new(0x2E11);
        for _ in 0..30 {
            let d = 2 + u64::from(rng.below(20));
            if isqrt(d) * isqrt(d) == d {
                continue;
            }
            let Some((x, y)) = solve(d) else { continue };
            for k in [2u64, 3, 5] {
                let (xk, yk) = power(&x, &y, d, k);
                assert!(is_one(&pell_residue(&xk, &yk, d)));
            }
            // negative-Pell solvable d: (x,y)² → +1 solutions
            if let Some((nx, ny)) = negative(d) {
                assert!(is_neg_one(&pell_residue(&nx, &ny, d)));
                let (sqx, sqy) = power(&nx, &ny, d, 2);
                assert!(is_one(&pell_residue(&sqx, &sqy, d)));
            }
        }
        // d=5 has odd period (1) → negative exists (2,1)
        assert_eq!(
            negative(5).map(|(x, y)| (x.to_i128().unwrap(), y.to_i128().unwrap())),
            Some((2, 1))
        );
        // d=3 has even period (2) → no negative
        assert_eq!(negative(3), None);
    }
}

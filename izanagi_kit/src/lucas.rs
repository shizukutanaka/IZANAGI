//! Lucas sequences `Uₖ(P,Q)` and `Vₖ(P,Q)` — the second-order
//! linear recurrences `Xₙ = P·Xₙ₋₁ − Q·Xₙ₋₂` with
//! `U₀=0, U₁=1` and `V₀=2, V₁=P`. Fibonacci is the
//! `(1,−1)` case (`U=F`, `V=L`); Lucas sequences are also
//! the backbone of Lucas pseudoprime tests, so a fast
//! doubling variant over `Z_m` is included.
//!
//! Exact values use `i128` checked arithmetic (`None` on
//! overflow — terms grow like `αᵏ`); the modular variant
//! uses the standard fast-doubling identities with a
//! halving trick that requires `m` odd.
//!
//! ```
//! use izanagi_kit::lucas::{lucas, lucas_mod};
//! // Fibonacci: U_10(1,−1) = F_10 = 55, V_10 = L_10 = 123
//! assert_eq!(lucas(10, 1, -1), Some((55, 123)));
//! // same values mod 100 via fast doubling
//! assert_eq!(lucas_mod(10, 1, -1, 1000003), Some((55, 123, 1)));
//! ```
//!
//! References: the doubling formulas
//! `U(2k)=U(k)V(k)`, `V(2k)=V(k)²−2Qᵏ`,
//! `U(2k+1)=(P·U(2k)+V(2k))/2` are the standard ones used
//! in Lucas–Lehmer-style tests (cp-algorithms / Library
//! Checker `lucas_number` write-ups on Qiita/Zenn).

/// Exact `(U_k, V_k)` for `P, Q` — `i128` checked
/// arithmetic, `None` on overflow.
pub fn lucas(k: u64, p: i64, q: i64) -> Option<(i128, i128)> {
    // U_0=0, U_1=1; V_0=2, V_1=P — linear scan, every
    // multiply is checked so the function stays total
    if k == 0 {
        return Some((0, 2));
    }
    let mut u_prev = 0i128;
    let mut u_cur = 1i128;
    let mut v_prev = 2i128;
    let mut v_cur = i128::from(p);
    let mut i = 1u64;
    while i < k {
        let u_next = u_cur
            .checked_mul(i128::from(p))?
            .checked_sub(u_prev.checked_mul(i128::from(q))?)?;
        let v_next = v_cur
            .checked_mul(i128::from(p))?
            .checked_sub(v_prev.checked_mul(i128::from(q))?)?;
        u_prev = u_cur;
        u_cur = u_next;
        v_prev = v_cur;
        v_cur = v_next;
        i += 1;
    }
    Some((u_cur, v_cur))
}

/// `(U_k, V_k, Qᵏ)` mod `m` via fast doubling — `m` must
/// be odd and `≥ 3` (the halving steps divide by 2, which
/// is only invertible mod odd `m`). `None` otherwise.
pub fn lucas_mod(k: u64, p: i64, q: i64, m: i64) -> Option<(i64, i64, i64)> {
    if m < 3 || m % 2 == 0 {
        return None;
    }
    if k == 0 {
        return Some((0, 2 % m, 1));
    }
    let m128 = i128::from(m);
    let norm = |x: i128| x.rem_euclid(m128);
    // halve the residue r ∈ [0,m): r odd → r+m is even
    // (m is odd, so adding m flips parity exactly when needed)
    let half = |x: i128| {
        let r = norm(x);
        (r + (r % 2) * m128) / 2
    };
    let d = norm(i128::from(p) * i128::from(p) - 4 * i128::from(q));
    let (mut u, mut v, mut qk) = (0i128, 2i128, 1i128); // U_0, V_0, Q^0
    let pb = norm(i128::from(p));
    let qb = norm(i128::from(q));
    let mut bits = 63 - k.leading_zeros();
    loop {
        // double: U(2k)=U·V, V(2k)=V²−2Qᵏ, Q^(2k)=Qᵏ²
        let u2 = norm(u * v);
        let v2 = norm(v * v - 2 * qk);
        let q2 = norm(qk * qk);
        u = u2;
        v = v2;
        qk = q2;
        if k >> bits & 1 == 1 {
            // odd step: U(k+1) = (P·U + V)/2, V(k+1) = (D·U + P·V)/2
            let u_next = half(pb * u + v);
            let v_next = half(d * u + pb * v);
            u = u_next;
            v = v_next;
            qk = norm(qk * qb);
        }
        if bits == 0 {
            break;
        }
        bits -= 1;
    }
    Some((u as i64, v as i64, qk as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        // Fibonacci (1,−1): U = F, V = Lucas numbers
        let (u10, v10) = lucas(10, 1, -1).unwrap();
        assert_eq!(u10, 55);
        assert_eq!(v10, 123);
        assert_eq!(lucas(0, 1, -1), Some((0, 2)));
        assert_eq!(lucas(1, 1, -1), Some((1, 1)));
        // Pell numbers (2,−1): U = 0,1,2,5,12,29,70
        assert_eq!(lucas(6, 2, -1).map(|(u, _)| u), Some(70));
        // m must be odd
        assert_eq!(lucas_mod(5, 1, -1, 4), None);
        assert_eq!(lucas_mod(10, 1, -1, 1000003), Some((55, 123, 1)));
    }

    /// Identity oracle: `Vₙ² − D·Uₙ² = 4·Qⁿ` for
    /// `D = P²−4Q` — the fundamental Lucas-sequence
    /// invariant, exact over i128.
    #[test]
    fn lucas_identity() {
        let mut rng = SplitMix64::new(0x1AC5);
        for _ in 0..800 {
            let p = i64::from(rng.below(20)) - 10;
            let q = i64::from(rng.below(10)) - 5;
            let k = u64::from(rng.below(12));
            let Some((u, v)) = lucas(k, p, q) else {
                continue;
            };
            let d = i128::from(p) * i128::from(p) - 4 * i128::from(q);
            let mut qn = 1i128;
            for _ in 0..k {
                qn = qn.checked_mul(i128::from(q)).unwrap_or(0);
            }
            assert_eq!(v * v - d * u * u, 4 * qn, "P={p} Q={q} k={k}");
        }
    }

    /// Fast doubling vs linear scan — same (U,V) mod m for
    /// random odd m, P, Q; also `Qᵏ` matches `mod_pow`.
    #[test]
    fn doubling_matches_scan() {
        let mut rng = SplitMix64::new(0xD0B1);
        for _ in 0..3000 {
            let p = i64::from(rng.below(20)) - 10;
            let q = i64::from(rng.below(10)) - 5;
            let k = u64::from(rng.below(25));
            let m = (i64::from(rng.below(2000)) | 1) + 2; // odd ≥ 3
            let Some((um, vm, qk)) = lucas_mod(k, p, q, m) else {
                continue;
            };
            let Some((u, v)) = lucas(k, p, q) else {
                continue;
            };
            let mm = i128::from(m);
            assert_eq!(i128::from(um), u.rem_euclid(mm));
            assert_eq!(i128::from(vm), v.rem_euclid(mm));
            let Some(qe) =
                crate::ntheory::mod_pow(q.rem_euclid(m), i64::try_from(k).unwrap_or(i64::MAX), m)
            else {
                continue;
            };
            assert_eq!(qk, qe, "Q^k mismatch");
        }
    }
}

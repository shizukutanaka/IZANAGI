//! Exact Bernoulli numbers `B₀=1, B₁=1/2, B₂=1/6, …`
//! via the Akiyama–Tanigawa algorithm over [`Frac`]
//! — `Bₙ` is a rational, so the whole triangle is exact.
//! The convention is the "+1/2" one (`B₁ = 1/2`), which
//! makes Faulhaber's formula read uniformly:
//! `Σᵢ₌₁ⁿ iᵏ = (1/(k+1))·Σⱼ₌₀ᵏ C(k+1,j)·Bⱼ·n^{k+1−j}`.
//!
//! ```
//! use izanagi_kit::bernoulli::{bernoulli, faulhaber};
//! use izanagi_kit::frac::Frac;
//! assert_eq!(bernoulli(2), Frac::new(1, 6));
//! assert_eq!(bernoulli(4), Frac::new(-1, 30));
//! // Σ i² for i=1..10 = 385
//! assert_eq!(faulhaber(10, 2), Frac::new(385, 1));
//! ```
//!
//! References: Akiyama–Tanigawa appears in Knuth TAOCP
//! §1.2.9 and the standard `bernoulli_number` entries on
//! cp-algorithms/Qiita; the Faulhaber identity is
//! Jacob Bernoulli's `Summae Potestatum` (1713).

use crate::frac::Frac;

/// The `n`-th Bernoulli number (convention `B₁ = +1/2`),
/// as an exact [`Frac`] — Akiyama–Tanigawa in `O(n²)`
/// rational ops.
pub fn bernoulli(n: u32) -> Frac {
    // a[j] holds the current row; Akiyama–Tanigawa:
    // start a[m] = 1/(m+1), then a[j-1] = j·(a[j-1] − a[j])
    // and B_m = a[0] after the sweep.
    let n = n as usize;
    let mut a = vec![Frac::new(0, 1); n + 1];
    let mut res = Frac::new(0, 1);
    for m in 0..=n {
        a[m] = Frac::new(1, (m + 1) as i128);
        for j in (1..=m).rev() {
            a[j - 1] = (a[j - 1] - a[j]) * Frac::new(j as i128, 1);
        }
        if m == n {
            res = a[0];
        }
    }
    res
}

/// `Σᵢ₌₁ⁿ iᵏ` as an exact [`Frac`] via Faulhaber's
/// formula `1/(k+1)·Σⱼ C(k+1,j)·Bⱼ·n^{k+1−j}` — the
/// result is always an integer, kept as `Frac` for
/// exactness.
pub fn faulhaber(n: u64, k: u32) -> Frac {
    let k = k as usize;
    let mut sum = Frac::new(0, 1);
    // binomial row (k+1 choose j) iteratively, exact i128
    let mut binom = 1i128;
    for j in 0..=k {
        let b = bernoulli(j as u32);
        let exp = (k + 1 - j) as u32;
        let mut np = 1i128;
        for _ in 0..exp {
            np = np.saturating_mul(n as i128);
        }
        sum = sum + b * Frac::new(binom.saturating_mul(np), 1);
        // binom_{j+1} = binom_j * (k+1−j) / (j+1)
        binom = binom * (k + 1 - j) as i128 / (j + 1) as i128;
    }
    sum * Frac::new(1, (k + 1) as i128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(bernoulli(0), Frac::new(1, 1));
        assert_eq!(bernoulli(1), Frac::new(1, 2));
        assert_eq!(bernoulli(2), Frac::new(1, 6));
        assert_eq!(bernoulli(3), Frac::new(0, 1));
        assert_eq!(bernoulli(4), Frac::new(-1, 30));
        assert_eq!(bernoulli(6), Frac::new(1, 42));
        assert_eq!(bernoulli(8), Frac::new(-1, 30));
        assert_eq!(bernoulli(10), Frac::new(5, 66));
        assert_eq!(bernoulli(12), Frac::new(-691, 2730));
        assert_eq!(faulhaber(10, 2), Frac::new(385, 1));
        assert_eq!(faulhaber(100, 1), Frac::new(5050, 1));
        assert_eq!(faulhaber(1, 5), Frac::new(1, 1));
    }

    /// Faulhaber vs the direct power-sum oracle, and the
    /// B_{2k+1}=0 zero theorem for odd k ≥ 1.
    #[test]
    fn faulhaber_and_odd_zeros() {
        let mut rng = SplitMix64::new(0xB347);
        for _ in 0..400 {
            let n = u64::from(rng.below(60));
            let k = rng.below(8);
            let mut direct = 0i128;
            for i in 1..=n.max(1) {
                let mut p = 1i128;
                for _ in 0..k {
                    p = p.saturating_mul(i as i128);
                }
                direct += p;
            }
            // note: Σ_{i=1}^n i^k — n=0 sums to 0
            if n == 0 {
                assert_eq!(faulhaber(0, k), Frac::new(0, 1));
            } else {
                assert_eq!(faulhaber(n, k), Frac::new(direct, 1), "n={n} k={k}");
            }
        }
        for k in 1..20u32 {
            let b = bernoulli(2 * k + 1);
            assert_eq!(b, Frac::new(0, 1), "B_{} must be 0", 2 * k + 1);
        }
    }

    /// Recurrence oracle: the Akiyama triangle's defining
    /// identity `Σⱼ C(n+1,j)·Bⱼ = n+1` for all n — a
    /// theorem-level check that B's satisfy the standard
    /// generating recurrence, not just tabulated values.
    #[test]
    fn recurrence_identity() {
        for n in 0..20usize {
            let mut sum = Frac::new(0, 1);
            let mut binom = 1i128;
            for j in 0..=n {
                sum = sum + bernoulli(j as u32) * Frac::new(binom, 1);
                binom = binom * (n + 1 - j) as i128 / (j + 1) as i128;
            }
            assert_eq!(sum, Frac::new(n as i128 + 1, 1), "n={n}");
        }
    }
}

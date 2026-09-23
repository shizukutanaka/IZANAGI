//! Egyptian fraction decompositions — writing a positive
//! rational as a sum of *distinct* unit fractions
//! `1/d₁ + 1/d₂ + …`. Fibonacci–Sylvester greedy: always
//! take the largest unit fraction not exceeding the
//! remainder, `d = ⌈den/num⌉`, then continue with
//! `f − 1/d`. The numerator strictly decreases at each
//! step, so the greedy always terminates — but
//! denominators can grow doubly exponentially, so a hard
//! step cap keeps the function honest (`None` when the
//! `i128` range would be exceeded).
//!
//! ```
//! use izanagi_kit::egypt::{expand, verify};
//! use izanagi_kit::frac::Frac;
//! // 5/6 = 1/2 + 1/3
//! let ds = expand(&Frac::new(5, 6)).unwrap();
//! assert_eq!(ds, vec![2, 3]);
//! assert!(verify(&Frac::new(5, 6), &ds));
//! // 4/13 = 1/4 + 1/18 + 1/468
//! assert_eq!(expand(&Frac::new(4, 13)).unwrap(), vec![4, 18, 468]);
//! ```
//!
//! References: Fibonacci's greedy (1202, *Liber Abaci*)
//! with Sylvester's termination proof (1880); the
//! `den·d` overflow bound is the practical determinism
//! guard.

use crate::frac::Frac;

/// `⌈a/b⌉` for positive `a, b` — `i128::div_ceil` is not
/// yet stable on this toolchain.
fn ceil_div(a: i128, b: i128) -> i128 {
    a / b + i128::from(a % b != 0)
}

/// Greedy Egyptian fraction expansion of a *proper*
/// fraction `0 < f < 1` — terms are distinct unit
/// fractions. Returns `None` for `f ≤ 0` or `f ≥ 1`
/// (improper fractions need an integer part the unit-sum
/// form can't carry), and `None` when a denominator would
/// exceed `i128` (the decomposition exists but is
/// unrepresentable). `steps` caps the expansion length —
/// every step shrinks the remainder's numerator, so
/// `steps ≥ f.num` suffices in principle, but the bound
/// also keeps runaway inputs finite.
pub fn decompose(f: &Frac, steps: usize) -> Option<Vec<i128>> {
    if f.num <= 0 || f.num >= f.den || steps == 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut rem = *f;
    for _ in 0..steps {
        if rem.num == 0 {
            break;
        }
        // d = ⌈den/num⌉
        let d = ceil_div(rem.den, rem.num);
        out.push(d);
        // rem − 1/d = (rem.num·d − rem.den)/(rem.den·d) —
        // reject early when the new numerator overflows
        let nn = rem.num.checked_mul(d)?.checked_sub(rem.den)?;
        let nd = rem.den.checked_mul(d)?;
        rem = Frac::new(nn, nd);
        if rem.num == 0 {
            break;
        }
    }
    if rem.num == 0 {
        Some(out)
    } else {
        None // ran out of steps
    }
}

/// `decompose` with the default cap — enough for every
/// `f` whose greedy expansion stays inside `i128`.
pub fn expand(f: &Frac) -> Option<Vec<i128>> {
    decompose(f, 1_000)
}

/// Check that `Σ 1/dᵢ` exactly equals `f` and that all
/// denominators are positive and distinct — the oracle
/// used by tests and by callers validating a cached
/// decomposition.
pub fn verify(f: &Frac, ds: &[i128]) -> bool {
    let mut sum = Frac::new(0, 1);
    let mut seen = std::collections::BTreeSet::new();
    for &d in ds {
        if d <= 0 || !seen.insert(d) {
            return false;
        }
        sum = Frac::new(sum.num * d + sum.den, sum.den * d);
    }
    sum == *f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(decompose(&Frac::new(5, 6), 10), Some(vec![2, 3]));
        assert_eq!(decompose(&Frac::new(4, 13), 10), Some(vec![4, 18, 468]));
        assert_eq!(decompose(&Frac::new(1, 7), 10), Some(vec![7]));
        assert_eq!(decompose(&Frac::new(0, 5), 10), None);
        assert_eq!(decompose(&Frac::new(-1, 5), 10), None);
        // unit input steps out
        assert_eq!(decompose(&Frac::new(5, 6), 1), None);
        assert!(verify(&Frac::new(5, 6), &[2, 3]));
        assert!(!verify(&Frac::new(5, 6), &[2, 4]));
        // repeated denominators rejected
        assert!(!verify(&Frac::new(1, 1), &[2, 2]));
        assert!(verify(&Frac::new(1, 1), &[1]));
    }

    /// Greedy oracle: verify every expansion and the
    /// numerator-descent invariant that proves termination.
    #[test]
    fn greedy_oracle() {
        let mut rng = SplitMix64::new(0xE6F7);
        for _ in 0..2000 {
            let num = 1 + i128::from(rng.below(60));
            let den = num + i128::from(rng.below(60));
            let f = Frac::new(num, den);
            let Some(ds) = expand(&f) else {
                // doubly-exponential cases honestly give up
                continue;
            };
            assert!(verify(&f, &ds), "f = {f:?}");
            // greedy invariant: first denominator is the
            // ceiling of den/num
            let first = ceil_div(f.den, f.num);
            assert_eq!(ds[0], first);
        }
    }

    /// Fibonacci–Sylvester termination: the remainder's
    /// numerator strictly decreases at each greedy step —
    /// `num·d − den < num` since `d < den/num + 1`.
    #[test]
    fn numerator_descent() {
        let mut rng = SplitMix64::new(0x7A11);
        for _ in 0..500 {
            let num = 2 + i128::from(rng.below(30));
            let den = num + 1 + i128::from(rng.below(40));
            let mut rem = Frac::new(num, den);
            let mut prev = rem.num;
            for _ in 0..64 {
                let d = ceil_div(rem.den, rem.num);
                let nn = rem.num * d - rem.den;
                let Some(nd) = rem.den.checked_mul(d) else {
                    break;
                };
                rem = Frac::new(nn, nd);
                if rem.num == 0 {
                    break;
                }
                assert!(rem.num < prev, "num must descend: {prev} -> {}", rem.num);
                prev = rem.num;
            }
        }
    }
}

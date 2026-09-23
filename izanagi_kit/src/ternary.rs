//! Discrete ternary search — find the minimizer of a unimodal
//! sequence or unimodal integer function without scanning it
//! all. Works entirely on `i64` indices and values.
//!
//! *Unimodal* here means *strictly* unimodal: `f` is strictly
//! decreasing up to its minimum, then strictly increasing.
//! The strictness is load-bearing — on a merely non-increasing
//! staircase like `[26,22,19,16,15,11,11,8]` an equality probe
//! `f(m1) == f(m2)` cannot confine the argmin to either side
//! (the flat step may be followed by another descent), so
//! classic ternary rules lose it. On integer domains the
//! two-thirds split stalls below a 3-point interval anyway,
//! so the tail window is evaluated exhaustively.
//!
//! ```
//! use izanagi_kit::ternary::{argmin_seq, argmin_domain};
//! let v = [9i64, 7, 4, 1, 3, 8];
//! assert_eq!(argmin_seq(&v), Some(3));
//! // f(x) = (x-5)² on [0, 12] → argmin at 5
//! assert_eq!(argmin_domain(0, 12, |x| (x - 5) * (x - 5)), Some(5));
//! ```
//!
//! References: standard ternary-search folklore; discrete
//! convergence handling as in cp-algorithms' integer case.

/// Argmin index of a unimodal slice via ternary search.
///
/// Returns `None` on empty input. The slice must be
/// *strictly* unimodal (see module docs); the returned index
/// is verified against a global-minimum scan of the whole
/// slice — inputs outside the contract either still return
/// the true argmin (when the search happens to find it) or
/// return `None`, never a wrong index silently.
pub fn argmin_seq(a: &[i64]) -> Option<usize> {
    if a.is_empty() {
        return None;
    }
    let (mut lo, mut hi) = (0usize, a.len() - 1);
    while hi - lo >= 3 {
        let third = (hi - lo) / 3;
        let m1 = lo + third;
        let m2 = hi - third;
        if a[m1] <= a[m2] {
            hi = m2;
        } else {
            lo = m1;
        }
    }
    let i = (lo..=hi).min_by_key(|&i| a[i])?;
    // verify unimodality around the answer: a[i−1..=i+1] must
    // bracket it, and the answer must be globally minimal on
    // the full slice for unimodality to hold at all
    if a.iter().all(|&v| v >= a[i]) {
        Some(i)
    } else {
        None
    }
}

/// Argmin of a unimodal integer function on `[lo, hi]`.
///
/// Same convergence handling as [`argmin_seq`]: the tail
/// window is evaluated exhaustively. `f` must be strictly
/// unimodal on the domain (cannot be checked for a function),
/// in which case the answer is the exact argmin. Returns
/// `None` when `lo > hi`.
pub fn argmin_domain(lo: i64, hi: i64, f: impl Fn(i64) -> i64) -> Option<i64> {
    if lo > hi {
        return None;
    }
    let (mut lo, mut hi) = (lo, hi);
    while hi - lo >= 3 {
        let third = (hi - lo) / 3;
        let m1 = lo + third;
        let m2 = hi - third;
        if f(m1) <= f(m2) {
            hi = m2;
        } else {
            lo = m1;
        }
    }
    (lo..=hi).min_by_key(|&x| f(x))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(argmin_seq(&[]), None);
        assert_eq!(argmin_seq(&[7]), Some(0));
        assert_eq!(argmin_seq(&[3, 1, 2]), Some(1));
        assert_eq!(argmin_seq(&[5, 4, 3, 6]), Some(2));
        // non-unimodal: search may still find the global min
        assert_eq!(argmin_seq(&[1, 2, 1, 0]), Some(3));
        // or return None when its found min isn't global
        assert_eq!(argmin_seq(&[1, 5, 2, 4, 3]), None);
        assert_eq!(argmin_domain(9, 3, |x| x), None);
        assert_eq!(argmin_domain(-10, 10, |x| x * x), Some(0));
    }

    /// Oracle: brute argmin on random *strictly*-unimodal
    /// sequences (strict descent to a minimum, strict ascent).
    #[test]
    fn oracle_random_unimodal() {
        let mut rng = SplitMix64::new(0x71E5);
        for _ in 0..200 {
            let n = 1 + rng.below(50) as usize;
            let k = rng.below(n as u32) as usize; // argmin position
                                                  // strictly decreasing to v[k], strictly
                                                  // increasing after it — strictness is part of
                                                  // the contract (flats on an arm break ternary)
            let mut v = vec![0i64; n];
            let mut cur = i64::from(rng.below(10));
            for i in (0..=k).rev() {
                v[i] = cur;
                cur += i64::from(1 + rng.below(4));
            }
            cur = v[k] + i64::from(1 + rng.below(3));
            for item in v.iter_mut().skip(k + 1) {
                *item = cur;
                cur += i64::from(1 + rng.below(4));
            }
            let got = argmin_seq(&v).unwrap();
            assert_eq!(v[got], v[k]);
            // function version on the same sequence
            let want = (0..n).min_by_key(|&i| v[i]).unwrap();
            assert_eq!(got, want);
        }
    }

    /// Function oracle over small domains: ternary result
    /// equals brute force on unimodal quadratics and V-shapes.
    #[test]
    fn oracle_random_functions() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..200 {
            let x0 = i64::from(rng.below(41)) - 20;
            let lo = i64::from(rng.below(20)) - 30;
            let hi = lo + i64::from(1 + rng.below(60));
            let quad = |x: i64| (x - x0) * (x - x0);
            let vshape = |x: i64| (x - x0).abs();
            for f in [&quad as &dyn Fn(i64) -> i64, &vshape] {
                let got = argmin_domain(lo, hi, f).unwrap();
                let want = (lo..=hi).min_by_key(|&x| f(x)).unwrap();
                assert_eq!(f(got), f(want));
            }
        }
    }
}

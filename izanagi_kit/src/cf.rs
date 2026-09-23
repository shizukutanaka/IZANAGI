//! Continued fractions over exact rationals. `to_cf` runs Euclid's
//! algorithm on the numerator/denominator pair — pure integer
//! recursion, so the expansion of a `Frac` terminates and is
//! canonical (final term > 1 when the list has more than one
//! entry). `best_approx` returns the closest rational with
//! denominator ≤ `max_den` — the "snap a rotation to a friendly
//! fraction" primitive for tile angles and tempo ratios.
//!
//! ```
//! use izanagi_kit::cf::{to_cf, from_cf, convergents, best_approx};
//! use izanagi_kit::frac::Frac;
//!
//! let pi = Frac::new(355, 113);
//! let cf = to_cf(&pi); // [3; 7, 15, 1, 292] terminates exactly
//! assert_eq!(from_cf(&cf), pi);
//! // 245/78 ≈ 3.14103 is closer to 3.141 than 22/7 ≈ 3.14286.
//! assert_eq!(best_approx(&Frac::new(3141, 1000), 100), Some(Frac::new(245, 78)));
//! ```
//!
//! References: Knuth TAOCP 4.5.3; Khinchin, "Continued
//! Fractions"; the theorem that every convergent is a best
//! approximation of the second kind.

use crate::frac::Frac;

/// Simple continued-fraction expansion `[a0; a1, a2, ...]` of `f`.
/// `a0` may be any integer (floor of `f`), `a1..` are positive,
/// and a multi-term expansion never ends in `1` — that trailing
/// `1` is folded into the previous term so the encoding is unique.
pub fn to_cf(f: &Frac) -> Vec<i128> {
    let (mut n, mut d) = (f.num, f.den);
    let mut out = Vec::new();
    while d != 0 {
        let a = n.div_euclid(d); // d > 0 keeps this a true floor
        out.push(a);
        let r = n - a * d;
        n = d;
        d = r;
    }
    // Canonicalize [a0; .., k, 1] -> [a0; .., k+1].
    if out.len() > 1 && out.last() == Some(&1) {
        out.pop();
        let last = out.len() - 1;
        out[last] += 1;
    }
    out
}

/// Rebuild the rational value from its expansion.
pub fn from_cf(cf: &[i128]) -> Frac {
    let (mut p, mut q) = (0i128, 1i128);
    for &a in cf.iter().rev() {
        // p/q -> a + q/p  == (a p + q) / p; guard empty tail.
        let (np, nq) = (a * p + q, p);
        if nq == 0 {
            p = a;
            q = 1;
        } else {
            p = np;
            q = nq;
        }
    }
    Frac::new(p, q)
}

/// All convergents `p_i/q_i` of the expansion, lowest terms.
pub fn convergents(cf: &[i128]) -> Vec<Frac> {
    let (mut pp, mut p) = (0i128, 1i128); // p_{-2}, p_{-1}
    let (mut qq, mut q) = (1i128, 0i128); // q_{-2}, q_{-1}
    let mut out = Vec::with_capacity(cf.len());
    for &a in cf {
        let (np, nq) = (a * p + pp, a * q + qq);
        out.push(Frac::new(np, nq));
        pp = p;
        p = np;
        qq = q;
        q = nq;
    }
    out
}

/// Closest rational to `f` with denominator `<= max_den`.
/// Walks convergents until the next denominator overshoots, then
/// tests the last convergent against the largest feasible
/// semiconvergent — the classic best-approximation theorem.
/// `None` when `max_den == 0` (no legal denominator exists).
pub fn best_approx(f: &Frac, max_den: u64) -> Option<Frac> {
    if max_den == 0 {
        return None;
    }
    let cap = max_den as i128;
    let cf = to_cf(f);
    let (mut pp, mut p) = (0i128, 1i128);
    let (mut qq, mut q) = (1i128, 0i128);
    for &a in &cf {
        let (np, nq) = (a * p + pp, a * q + qq);
        if nq > cap {
            // Largest semiconvergent coefficient between q_prev and cap.
            let t = (cap - qq) / q;
            let mut best = Frac::new(p, q);
            if t >= 1 {
                let cand = Frac::new(t * p + pp, t * q + qq);
                if closer_to(&cand, &best, f) {
                    best = cand;
                }
            }
            return Some(best);
        }
        pp = p;
        p = np;
        qq = q;
        q = nq;
    }
    Some(Frac::new(p, q)) // exact — every convergent fit
}

/// `|a - x| < |b - x|`, or smaller denominator on a tie.
fn closer_to(a: &Frac, b: &Frac, x: &Frac) -> bool {
    let da = *a - *x;
    let db = *b - *x;
    let lhs = da.num.unsigned_abs() * db.den as u128;
    let rhs = db.num.unsigned_abs() * da.den as u128;
    lhs < rhs || (lhs == rhs && a.den < b.den)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_canonical() {
        assert_eq!(to_cf(&Frac::new(415, 93)), vec![4, 2, 6, 7]);
        assert_eq!(to_cf(&Frac::new(3, 1)), vec![3]);
        assert_eq!(to_cf(&Frac::new(-1, 2)), vec![-1, 2]);
        assert_eq!(to_cf(&Frac::new(0, 1)), vec![0]);
        // Canonical form never ends in 1:
        assert_eq!(to_cf(&Frac::new(3, 2)), vec![1, 2]); // 1+1/(1+1/1)
        let mut r = crate::rng::SplitMix64::new(0xCA11);
        for _ in 0..500 {
            let n = r.next_u64() as i128 % 10000;
            let d = 1 + (r.next_u64() % 5000) as i128;
            let f = Frac::new(n, d);
            assert_eq!(from_cf(&to_cf(&f)), f, "round trip {f:?}");
        }
    }

    #[test]
    fn convergents_alternate_and_approach() {
        let cf = to_cf(&Frac::new(355, 113));
        let conv = convergents(&cf);
        assert_eq!(conv.last().copied(), Some(Frac::new(355, 113)));
        assert_eq!(conv[0], Frac::new(3, 1));
        // Each convergent is a best-approximation witness: denominators
        // strictly increase.
        for w in conv.windows(2) {
            assert!(w[0].den < w[1].den);
        }
    }

    fn brute_best(f: &Frac, cap: i128) -> Frac {
        // Closest p/q with 1 <= q <= cap; tie -> smaller q.
        let mut best: Option<Frac> = None;
        for q in 1..=cap {
            let center = f.num * q;
            for p in (center / f.den - 2)..=(center / f.den + 2) {
                let g = Frac::new(p, q);
                match best {
                    None => best = Some(g),
                    Some(b) => {
                        if closer_to(&g, &b, f) {
                            best = Some(g);
                        }
                    }
                }
            }
        }
        best.unwrap_or(Frac::new(0, 1))
    }

    #[test]
    fn best_approx_matches_brute_force() {
        let mut r = crate::rng::SplitMix64::new(0xB357);
        for _ in 0..300 {
            let num = (r.next_u64() % 200) as i128 - 100;
            let den = 1 + (r.next_u64() % 60) as i128;
            let f = Frac::new(num, den);
            let cap = (r.below(30) + 1) as i128;
            assert_eq!(
                best_approx(&f, cap as u64).unwrap_or(Frac::new(-999, 1)),
                brute_best(&f, cap),
                "f={f:?} cap={cap}"
            );
        }
    }

    #[test]
    fn known_approximations() {
        // 355/113 = [3;7,15,1,292]: q=106 overshoots cap 60, so the
        // answer is the largest semiconvergent t=8 -> (8·22+3)/(8·7+1).
        assert_eq!(
            best_approx(&Frac::new(355, 113), 60),
            Some(Frac::new(179, 57))
        );
        assert_eq!(
            best_approx(&Frac::new(355, 113), 113),
            Some(Frac::new(355, 113))
        );
        assert_eq!(best_approx(&Frac::new(1, 2), 0), None);
        // den 3 <= 4 -> the value itself is legal and returned exactly.
        assert_eq!(best_approx(&Frac::new(-7, 3), 4), Some(Frac::new(-7, 3)));
        assert_eq!(best_approx(&Frac::new(22, 7), 10), Some(Frac::new(22, 7)));
        assert_eq!(best_approx(&Frac::new(22, 7), 6), Some(Frac::new(19, 6)));
    }
}

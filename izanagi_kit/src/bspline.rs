//! Integer-exact B-spline evaluation — de Boor's stable
//! recurrence over [`Frac`] with arbitrary integer knot
//! vectors (clamped, open-uniform, or repeated). Complements
//! [`crate::bezier`], which is the single-segment
//! Bernstein special case (knots `[0…0,1…1]`, degree n).
//!
//! `eval` implements the NURBS-book algorithm A2.1 in the
//! homogeneous-rational form over 2-D integer control points:
//! the returned point is a `Frac` pair, exact for any rational
//! parameter — no `f64` anywhere, so curves are bit-identical
//! across platforms.
//!
//! Convention: the parameter domain is `[knots[p], knots[n+1]]`
//! where `p` = degree and `n = ctrl.len()-1`; the right end
//! returns the last control point for clamped vectors, and the
//! `0/0 → 0` rule applies at repeated knots.
//!
//! ```
//! use izanagi_kit::bspline::eval;
//! use izanagi_kit::frac::Frac;
//! // cubic with clamped ends through its endpoints
//! let knots = [0, 0, 0, 0, 1, 2, 2, 2, 2];
//! let ctrl = [(0, 0), (4, 8), (8, 8), (10, 0), (12, 0)];
//! let (x, _y) = eval(3, &ctrl, &knots, &Frac::from_int(0)).unwrap();
//! assert_eq!(x, Frac::from_int(0));
//! ```
//!
//! Reference: Piegl & Tiller, *The NURBS Book*, §A2.1.

use crate::frac::Frac;

/// de Boor evaluation of the 2-D B-spline with `degree` p,
/// control points `ctrl` (n+1 of them), knot vector `knots`
/// (n+p+2 of them), at rational parameter `t`. `None` when t is
/// outside `[knots[p], knots[n+1]]` or inputs are malformed.
pub fn eval(degree: usize, ctrl: &[(i64, i64)], knots: &[i64], t: &Frac) -> Option<(Frac, Frac)> {
    let n = ctrl.len().checked_sub(1)?;
    if knots.len() != ctrl.len() + degree + 1 {
        return None;
    }
    for w in knots.windows(2) {
        if w[0] > w[1] {
            return None;
        }
    }
    let lo = Frac::from_int(knots[degree] as i128);
    let hi = Frac::from_int(knots[n + 1] as i128);
    if t.cmp_frac(&lo) == std::cmp::Ordering::Less || t.cmp_frac(&hi) == std::cmp::Ordering::Greater
    {
        return None;
    }
    if lo == hi {
        return None; // empty domain
    }
    // span: for t < hi, the unique s with u[s] ≤ t < u[s+1];
    // at the right end t == hi, the curve is *left-continuous*
    // — take the last non-empty span in [p, n], which for
    // degree 0 is the only convention that yields the limit
    let s = if *t == hi {
        let mut last = None;
        for i in degree..=n {
            if knots[i] < knots[i + 1] {
                last = Some(i);
            }
        }
        last?
    } else {
        let mut s = degree;
        while s < n
            && Frac::from_int(knots[s + 1] as i128).cmp_frac(t) != std::cmp::Ordering::Greater
        {
            s += 1;
        }
        s
    };
    let mut d: Vec<(Frac, Frac)> = Vec::with_capacity(degree + 1);
    for j in 0..=degree {
        d.push((
            Frac::from_int(ctrl[s + j - degree].0 as i128),
            Frac::from_int(ctrl[s + j - degree].1 as i128),
        ));
    }
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = s + j - degree;
            let denom = knots[i + degree - r + 1].saturating_sub(knots[i]);
            // repeated knot → α = 0 (the 0/0 rule), i.e.
            // d[j] takes the left term d[j-1] unchanged
            let alpha = if denom == 0 {
                Frac::from_int(0)
            } else {
                (*t - Frac::from_int(knots[i] as i128))
                    .checked_div(Frac::from_int(denom as i128))?
            };
            let (lx, ly) = d[j - 1];
            let (rx, ry) = d[j];
            d[j] = (lx + (rx - lx) * alpha, ly + (ry - ly) * alpha);
        }
    }
    Some(d[degree])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Cox–de Boor basis oracle — the recursive definition,
    /// evaluated independently of de Boor's in-place loop.
    /// `n` is the last control index — needed because the
    /// domain ends at `u[n+1]`, not `u[m]`: spans after
    /// `u[n+1]` sit outside the curve's domain.
    fn basis(i: usize, k: usize, knots: &[i64], t: &Frac, n: usize) -> Frac {
        let u = |j: usize| Frac::from_int(knots[j] as i128);
        if k == 0 {
            let (a, b) = (u(i), u(i + 1));
            // right-end convention: t == u[n+1] belongs to the
            // last *non-empty* elementary span inside the domain
            // (repeated or out-of-domain tail knots excluded)
            let right_end = u(n + 1);
            let last_span = (0..=n)
                .rev()
                .find(|&j| knots[j] < knots[j + 1])
                .unwrap_or(0);
            // the domain-end t == right_end belongs to the
            // last non-empty domain span *only* — spans that
            // start at right_end must not also claim it
            return if (t.cmp_frac(&a) != std::cmp::Ordering::Less
                && t.cmp_frac(&b) == std::cmp::Ordering::Less
                && *t != right_end)
                || (t.cmp_frac(&right_end) == std::cmp::Ordering::Equal && i == last_span)
            {
                Frac::from_int(1)
            } else {
                Frac::from_int(0)
            };
        }
        let mut acc = Frac::from_int(0);
        let d1 = knots[i + k] - knots[i];
        if d1 != 0 {
            let a = (*t - u(i))
                .checked_div(Frac::from_int(d1 as i128))
                .unwrap_or(Frac::from_int(0));
            acc = acc + basis(i, k - 1, knots, t, n) * a;
        }
        let d2 = knots[i + k + 1] - knots[i + 1];
        if d2 != 0 {
            let a = (u(i + k + 1) - *t)
                .checked_div(Frac::from_int(d2 as i128))
                .unwrap_or(Frac::from_int(0));
            acc = acc + basis(i + 1, k - 1, knots, t, n) * a;
        }
        acc
    }

    fn oracle(ctrl: &[(i64, i64)], degree: usize, knots: &[i64], t: &Frac) -> (Frac, Frac) {
        let mut x = Frac::from_int(0);
        let mut y = Frac::from_int(0);
        let n = ctrl.len() - 1;
        for (i, &(cx, cy)) in ctrl.iter().enumerate() {
            let b = basis(i, degree, knots, t, n);
            x = x + Frac::from_int(cx as i128) * b;
            y = y + Frac::from_int(cy as i128) * b;
        }
        (x, y)
    }

    fn random_knots(rng: &mut SplitMix64, len: usize) -> Vec<i64> {
        let mut k = Vec::with_capacity(len);
        let mut cur = 0i64;
        for _ in 0..len {
            cur += (rng.below(3)) as i64; // repeats allowed
            k.push(cur);
        }
        k
    }

    #[test]
    fn bezier_coincidence() {
        // degree-n Bézier = B-spline on [0…0, 1…1]: endpoints
        // land on the end control points, midpoint = oracle
        let ctrl = [(0, 0), (3, 6), (6, -3), (9, 0)];
        let knots = [0, 0, 0, 0, 1, 1, 1, 1];
        let t = Frac::new(1, 2);
        let got = eval(3, &ctrl, &knots, &t).unwrap();
        assert_eq!(got, oracle(&ctrl, 3, &knots, &t));
        assert_eq!(
            eval(3, &ctrl, &knots, &Frac::from_int(0)).unwrap().0,
            Frac::from_int(0)
        );
        assert_eq!(
            eval(3, &ctrl, &knots, &Frac::from_int(1)).unwrap().0,
            Frac::from_int(9)
        );
    }

    #[test]
    fn degree1_piecewise_linear() {
        // degree-1 is straight lerp between adjacent ctrl points
        let ctrl = [(0, 0), (10, 4), (20, 0)];
        let knots = [0, 0, 1, 2, 2];
        // t = 1/2 → midpoint of first leg (5,2)
        let (x, y) = eval(1, &ctrl, &knots, &Frac::new(1, 2)).unwrap();
        assert_eq!((x, y), (Frac::from_int(5), Frac::from_int(2)));
        // t = 3/2 → midpoint of second leg (15,2)
        let (x, y) = eval(1, &ctrl, &knots, &Frac::new(3, 2)).unwrap();
        assert_eq!((x, y), (Frac::from_int(15), Frac::from_int(2)));
    }

    #[test]
    fn oracle_random_knots() {
        let mut rng = SplitMix64::new(97);
        for _round in 0..200 {
            let degree = (rng.below(4)) as usize;
            let n = (rng.below(4) + 1) as usize;
            let ctrl: Vec<(i64, i64)> = (0..=n)
                .map(|_| (rng.below(17) as i64 - 8, rng.below(17) as i64 - 8))
                .collect();
            let knots = random_knots(&mut rng, ctrl.len() + degree + 1);
            if knots[degree] == knots[n + 1] {
                continue; // degenerate empty domain
            }
            // 5 rational parameters inside the domain
            for _ in 0..5 {
                let lo = knots[degree];
                let hi = knots[n + 1];
                let num = lo + (rng.below((hi - lo + 1).max(1) as u32)) as i64;
                let den = 1 + (rng.below(3)) as i64;
                let t = Frac::new((num * 10) as i128, (den * 10) as i128);
                // clamp t into domain
                if t.cmp_frac(&Frac::from_int(lo as i128)) == std::cmp::Ordering::Less
                    || t.cmp_frac(&Frac::from_int(hi as i128)) == std::cmp::Ordering::Greater
                {
                    continue;
                }
                let got = eval(degree, &ctrl, &knots, &t).unwrap();
                let want = oracle(&ctrl, degree, &knots, &t);
                assert_eq!(got, want, "round {_round} t={t:?} knots={knots:?}");
            }
        }
    }

    #[test]
    fn partition_of_unity() {
        // Σ N_i,p(t) ≡ 1 over the whole domain — independent
        // of de Boor; catches span-seek and blending slips
        let mut rng = SplitMix64::new(31);
        for _round in 0..100 {
            let degree = (rng.below(3)) as usize;
            let n = (rng.below(4) + 1) as usize;
            let knots = random_knots(&mut rng, n + degree + 2);
            if knots[degree] == knots[n + 1] {
                continue;
            }
            let lo = knots[degree];
            let hi = knots[n + 1];
            let t = Frac::new(
                (lo * 4 + rng.below(((hi - lo) * 4 + 1).max(1) as u32) as i64) as i128,
                4,
            );
            let mut sum = Frac::from_int(0);
            for i in 0..=n {
                sum = sum + basis(i, degree, &knots, &t, n);
            }
            assert_eq!(
                sum,
                Frac::from_int(1),
                "round {_round} t={t:?} knots={knots:?}"
            );
        }
    }

    #[test]
    fn malformed_inputs() {
        let ctrl = [(0, 0), (1, 1)];
        assert_eq!(eval(1, &ctrl, &[0, 1], &Frac::from_int(0)), None);
        assert_eq!(eval(1, &ctrl, &[0, 0, 2, 1], &Frac::from_int(0)), None);
        assert_eq!(eval(1, &ctrl, &[0, 0, 1, 1], &Frac::from_int(2)), None);
        assert_eq!(eval(1, &ctrl, &[0, 0, 1, 1], &Frac::new(-1, 2)), None);
    }
}

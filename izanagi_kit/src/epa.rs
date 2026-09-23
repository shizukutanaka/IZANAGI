//! EPA — Expanding Polytope Algorithm, the overlapping-case
//! companion to `gjk`. When the Minkowski difference `A ⊖ B`
//! already contains the origin, `gjk::distance2` is `0` and says
//! nothing about *how deep*. EPA grows a polygon inside
//! `A ⊖ B` until an edge touches the boundary: the closest edge
//! to the origin is the minimum translation vector (MTV).
//!
//! All comparisons are exact `i128` rationals — the edge closest
//! point is kept as `(num/den)` like `gjk`, so the `dot(w,n) ==
//! dot(edge,n)` convergence test needs no epsilon. Inputs must
//! overlap: `penetration` returns `None` otherwise (use
//! `gjk::overlap` first if unsure).
//!
//! ```
//! use izanagi_kit::epa;
//! // unit-overlapped boxes
//! let a = [(0, 0), (10, 0), (10, 10), (0, 10)];
//! let b = [(9, 0), (19, 0), (19, 10), (9, 10)];
//! let (depth2, (nx, ny)) = epa::penetration(&a, &b).unwrap();
//! assert_eq!((depth2.num, depth2.den), (1, 1)); // depth 1 along -x
//! assert_eq!((nx.signum(), ny), (1, 0));
//! ```
//!
//! References: Van den Bergen (2001); bullet's btGjkEpa2 for the
//! closest-edge bookkeeping.

use crate::frac::Frac;
use crate::gjk::{reduce, support_diff, V};

type P = (i64, i64);

fn dot(a: (i128, i128), b: P) -> i128 {
    a.0 * b.0 as i128 + a.1 * b.1 as i128
}

/// Seed: a triangle of `A ⊖ B` containing the origin, CCW.
/// `None` when the shapes don't overlap (or share only a face —
/// depth 0 is reported as `Some` via the degenerate path below).
fn seed(a: &[P], b: &[P]) -> Option<Vec<P>> {
    let ca = centroid(a);
    let cb = centroid(b);
    let mut dir = (cb.0 - ca.0, cb.1 - ca.1);
    if dir == (0, 0) {
        dir = (1, 0);
    }
    let w = support_diff(a, b, (dir.0 as i128, dir.1 as i128));
    let mut simplex = vec![w];
    let mut v = V::of(w);
    for _ in 0..64 {
        if v.is_zero() {
            break;
        }
        let w = support_diff(a, b, (-v.nx, -v.ny));
        if v.len2_num() <= v.den * v.dot_i(w) {
            // converged without enclosing the origin → disjoint
            return None;
        }
        simplex.push(w);
        v = reduce(&mut simplex);
    }
    if !v.is_zero() {
        return None;
    }
    // `reduce` left the origin on a feature: a triangle when the
    // origin is strictly inside, a segment when it's exactly on
    // the boundary. Degenerate depth is still meaningful: expand
    // the segment into a thin triangle via a perpendicular probe.
    while simplex.len() < 3 {
        let d = if simplex.len() == 2 {
            let (dx, dy) = (
                simplex[1].0 as i128 - simplex[0].0 as i128,
                simplex[1].1 as i128 - simplex[0].1 as i128,
            );
            (-dy, dx)
        } else {
            (1i128, 0i128)
        };
        let w = support_diff(a, b, d);
        if simplex.contains(&w) {
            let w2 = support_diff(a, b, (-d.0, -d.1));
            if simplex.contains(&w2) {
                // touching-only contact — origin on the boundary
                simplex.push(w);
            } else {
                simplex.push(w2);
            }
        } else {
            simplex.push(w);
        }
    }
    // order CCW for outward normals
    let (pa, pb, pc) = (simplex[0], simplex[1], simplex[2]);
    let area = (pb.0 as i128 - pa.0 as i128) * (pc.1 as i128 - pa.1 as i128)
        - (pb.1 as i128 - pa.1 as i128) * (pc.0 as i128 - pa.0 as i128);
    if area < 0 {
        simplex.swap(0, 2);
    }
    Some(simplex)
}

fn centroid(pts: &[P]) -> P {
    let (mut sx, mut sy) = (0i64, 0i64);
    for p in pts {
        sx = sx.wrapping_add(p.0);
        sy = sy.wrapping_add(p.1);
    }
    let n = pts.len() as i64;
    (sx / n, sy / n)
}

/// Minimum translation between two overlapping convex polygons:
/// `(depth², normal)` with `normal` pointing from `a` toward `b`
/// (move `a` by `−depth·n̂`, or `b` by `+depth·n̂`, to separate —
/// `depth` is the square root of the returned `Frac`, which is
/// kept squared so the answer stays exact). `None` when the
/// shapes are disjoint or empty.
///
/// Depth is the minimum distance from the origin to the boundary
/// of `A ⊖ B`; the returned integer `normal` is parallel to the
/// separating axis (un-normalized — scale to taste).
pub fn penetration(a: &[P], b: &[P]) -> Option<(Frac, (i128, i128))> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let mut poly = seed(a, b)?;
    for _ in 0..128 {
        // closest edge to the origin + its outward normal
        let mut best_i = 0;
        let mut best_d2: Option<(i128, i128)> = None; // (num, den)
        let mut best_n = (0i128, 0i128);
        let m = poly.len();
        for i in 0..m {
            let (p, q) = (poly[i], poly[(i + 1) % m]);
            // outward normal for CCW winding: right of p→q
            let n = (q.1 as i128 - p.1 as i128, p.0 as i128 - q.0 as i128);
            let nn = n.0 * n.0 + n.1 * n.1;
            if nn == 0 {
                continue;
            }
            // signed distance² from origin to the edge's line —
            // negative-side sign impossible for a valid polytope
            let s = dot(n, p); // |signed_dist|² = s² / nn
            let better = match best_d2 {
                Some((bn, bd)) => s * s * bd < bn * nn,
                None => true,
            };
            if better {
                best_d2 = Some((s * s, nn));
                best_n = n;
                best_i = i;
            }
        }
        let (dn, dd) = best_d2?;
        let w = support_diff(a, b, best_n);
        let p = poly[best_i];
        // converged when the support in `n` cannot push the edge out
        if dot(best_n, w) <= dot(best_n, p) {
            return Some((Frac::new(dn, dd), best_n));
        }
        poly.insert(best_i + 1, w);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::convex_hull;
    use crate::rng::SplitMix64;
    use crate::sat;

    fn to_i32(v: &[P]) -> Vec<(i32, i32)> {
        v.iter().map(|&(x, y)| (x as i32, y as i32)).collect()
    }

    fn rand_convex(r: &mut SplitMix64, n: usize, spread: i64) -> Vec<P> {
        let mut pts = Vec::new();
        for _ in 0..n {
            pts.push((
                (r.next_u64() % (2 * spread) as u64) as i64 - spread,
                (r.next_u64() % (2 * spread) as u64) as i64 - spread,
            ));
        }
        let h = convex_hull(&to_i32(&pts));
        h.iter().map(|&(x, y)| (x as i64, y as i64)).collect()
    }

    /// Oracle: minimum overlap over every face normal of both
    /// polygons (the SAT witness that reports contact). Exact for
    /// convex polygons — the MTV always lies on a face normal.
    fn brute(a: &[P], b: &[P]) -> Option<(Frac, (i128, i128))> {
        if a.len() < 3 || b.len() < 3 {
            return None;
        }
        if !sat::overlap(&to_i32(a), &to_i32(b)) {
            return None;
        }
        let mut best: Option<(i128, i128, (i128, i128))> = None; // (num, den, axis)
        let mut check = |axis: (i128, i128)| {
            if axis == (0, 0) {
                return;
            }
            let (mut amin, mut amax) = (i128::MAX, i128::MIN);
            for p in a {
                let s = dot(axis, *p);
                amin = amin.min(s);
                amax = amax.max(s);
            }
            let (mut bmin, mut bmax) = (i128::MAX, i128::MIN);
            for p in b {
                let s = dot(axis, *p);
                bmin = bmin.min(s);
                bmax = bmax.max(s);
            }
            // signed overlap separating a from b along -axis…
            // MTV magnitude along axis = min(amax-bmin, bmax-amin)
            let o1 = amax - bmin; // b pulls toward -axis
            let o2 = bmax - amin; // b pushes toward +axis
            let (overlap, sign) = if o1 <= o2 { (o1, -1i128) } else { (o2, 1) };
            let nn = axis.0 * axis.0 + axis.1 * axis.1;
            let (num, den) = (overlap * overlap, nn);
            let nrm = (axis.0 * sign, axis.1 * sign);
            let better = match best {
                Some((bn, bd, _)) => num * bd < bn * den,
                None => true,
            };
            if better {
                best = Some((num, den, nrm));
            }
        };
        for i in 0..a.len() {
            let (p, q) = (a[i], a[(i + 1) % a.len()]);
            check((q.1 as i128 - p.1 as i128, p.0 as i128 - q.0 as i128));
        }
        for i in 0..b.len() {
            let (p, q) = (b[i], b[(i + 1) % b.len()]);
            check((q.1 as i128 - p.1 as i128, p.0 as i128 - q.0 as i128));
        }
        best.map(|(n, d, axis)| (Frac::new(n, d), axis))
    }

    #[test]
    fn boxes_one_unit() {
        let a = [(0, 0), (10, 0), (10, 10), (0, 10)];
        let b = [(9, 0), (19, 0), (19, 10), (9, 10)];
        let (d2, n) = penetration(&a, &b).unwrap();
        assert_eq!((d2.num, d2.den), (1, 1));
        assert_eq!((n.0.signum(), n.1.signum()), (1, 0));
    }

    #[test]
    fn diagonal_overlap() {
        let a = [(0, 0), (4, 0), (4, 4), (0, 4)];
        let b = [(2, 2), (6, 2), (6, 6), (2, 6)];
        let (d2, _n) = penetration(&a, &b).unwrap();
        // min overlap: x-overlap = 2, y-overlap = 2
        assert_eq!((d2.num, d2.den), (4, 1));
    }

    #[test]
    fn disjoint_is_none() {
        let a = [(0, 0), (10, 0), (10, 10), (0, 10)];
        let b = [(20, 0), (30, 0), (30, 10), (20, 10)];
        assert!(penetration(&a, &b).is_none());
    }

    #[test]
    fn oracle_random_convex() {
        let mut r = SplitMix64::new(0xE9A0);
        for _ in 0..400 {
            let a = rand_convex(&mut r, 8, 40);
            let b = rand_convex(&mut r, 8, 40);
            if a.len() < 3 || b.len() < 3 {
                continue;
            }
            let want = brute(&a, &b);
            match (penetration(&a, &b), want) {
                (None, None) => {}
                (Some((gd, gn)), Some((wd, wn))) => {
                    assert_eq!(
                        (gd.num, gd.den),
                        (wd.num, wd.den),
                        "depth mismatch a={a:?} b={b:?}"
                    );
                    // axis parallel to the oracle's (cross == 0)
                    assert_eq!(gn.0 * wn.1 - gn.1 * wn.0, 0, "axis a={a:?} b={b:?}");
                }
                (g, w) => panic!("epa={g:?} brute={w:?} a={a:?} b={b:?}"),
            }
        }
    }
}

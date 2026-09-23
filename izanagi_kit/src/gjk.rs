//! GJK — Gilbert–Johnson–Keerthi distance between convex shapes,
//! computed in exact `i128` rational arithmetic. `sat` answers
//! *whether* two convex polygons touch; GJK additionally answers
//! *how far apart* they are, and it does so without enumerating
//! features — the Minkowski difference `A ⊖ B` is explored only
//! through its support function.
//!
//! The algorithm keeps a 1–3 point simplex inside `A ⊖ B`, tracks
//! the closest point `v` to the origin on that simplex, and
//! searches in direction `−v`. It terminates when the best
//! support in `−v` cannot improve the current estimate —
//! `dot(w − v, −v) ≤ 0` — which exact integer arithmetic decides
//! with no epsilon at all. The closest point is kept as a
//! rational pair `(num/den)` so every comparison stays in
//! `i128`.
//!
//! ```
//! use izanagi_kit::gjk;
//! let a = [(0, 0), (10, 0), (10, 10), (0, 10)];
//! let b = [(20, 0), (30, 0), (30, 10), (20, 10)];
//! assert!(!gjk::overlap(&a, &b));
//! assert_eq!(gjk::distance2(&a, &b).map(|f| f.num), Some(100)); // gap of 10
//! let c = [(5, 5), (15, 5), (15, 15), (5, 15)];
//! assert!(gjk::overlap(&a, &c));
//! ```
//!
//! References: Gilbert, Johnson & Keerthi (1988); Ericson RTCD
//! §9.5 for the closest-sub-simplex bookkeeping.

use crate::frac::Frac;

type P = (i64, i64);

fn sub(a: P, b: P) -> (i128, i128) {
    (a.0 as i128 - b.0 as i128, a.1 as i128 - b.1 as i128)
}

fn cross(a: (i128, i128), b: (i128, i128)) -> i128 {
    a.0 * b.1 - a.1 * b.0
}

/// A point on the convex set expressed exactly: `(x,y) =
/// (nx/den, ny/den)` with `den > 0`.
#[derive(Clone, Copy, Debug)]
struct V {
    nx: i128,
    ny: i128,
    den: i128,
}

impl V {
    fn of(p: P) -> V {
        V {
            nx: p.0 as i128,
            ny: p.1 as i128,
            den: 1,
        }
    }
    /// `dot(self, w)` for integer `w`, scaled by `den` (compare
    /// against integer targets multiplied by `den` too).
    fn dot_i(&self, w: P) -> i128 {
        self.nx * w.0 as i128 + self.ny * w.1 as i128
    }
    /// `|v|² * den` — the numerator of `|v|²` times `den`, so
    /// `|v|² · den²` comparisons stay integral: `v_len2() vs
    /// den·dot(w,v)`.
    fn len2_num(&self) -> i128 {
        self.nx * self.nx + self.ny * self.ny
    }
    fn is_zero(&self) -> bool {
        self.nx == 0 && self.ny == 0
    }
}

/// Closest point to the origin on segment `pa`–`pb`.
fn closest_seg(pa: P, pb: P) -> V {
    let abx = pb.0 as i128 - pa.0 as i128;
    let aby = pb.1 as i128 - pa.1 as i128;
    let d = abx * abx + aby * aby;
    if d == 0 {
        return V::of(pa);
    }
    // t = dot(-a, ab)/d
    let t = -(pa.0 as i128 * abx + pa.1 as i128 * aby);
    if t <= 0 {
        V::of(pa)
    } else if t >= d {
        V::of(pb)
    } else {
        // a + t/d * ab, den = d
        V {
            nx: pa.0 as i128 * d + t * abx,
            ny: pa.1 as i128 * d + t * aby,
            den: d,
        }
    }
}

/// Compare `|u|²` vs `|w|²` across different denominators.
fn lt_len2(u: V, w: V) -> bool {
    u.len2_num() * w.den * w.den < w.len2_num() * u.den * u.den
}

/// Reduce a 3-point simplex to the sub-simplex carrying the
/// closest point to the origin; returns the point itself.
fn closest_tri(p: &[P]) -> (Vec<P>, V) {
    let (a, b, c) = (p[0], p[1], p[2]);
    // inside test: the origin lies inside iff the cross products
    // of each edge with the vector edge→origin share one sign
    let (ax, ay) = (a.0 as i128, a.1 as i128);
    let (bx, by) = (b.0 as i128, b.1 as i128);
    let (cx, cy) = (c.0 as i128, c.1 as i128);
    let ab = (bx - ax, by - ay);
    let bc = (cx - bx, cy - by);
    let ca = (ax - cx, ay - cy);
    let d1 = cross(ab, (-ax, -ay));
    let d2 = cross(bc, (-bx, -by));
    let d3 = cross(ca, (-cx, -cy));
    let neg = (d1 < 0) as u8 + (d2 < 0) as u8 + (d3 < 0) as u8;
    if neg == 0 || neg == 3 {
        // origin inside the triangle — distance is exactly zero
        return (
            vec![a, b, c],
            V {
                nx: 0,
                ny: 0,
                den: 1,
            },
        );
    }
    // outside: closest lives on one of the three edges
    let edges = [(a, b), (b, c), (c, a)];
    let mut best_pts = vec![a, b];
    let mut best = closest_seg(a, b);
    for &(u, w) in &edges[1..] {
        let cand = closest_seg(u, w);
        if lt_len2(cand, best) {
            best = cand;
            best_pts = vec![u, w];
        }
    }
    (best_pts, best)
}

/// Farthest point of `pts` in direction `dir` (a rational pair
/// `(dx,dy)`; scaling `den` out: `argmax nx·x + ny·y`).
fn support_pts(pts: &[P], dir: (i128, i128)) -> P {
    let mut best = pts[0];
    let mut bs = dir.0 * best.0 as i128 + dir.1 * best.1 as i128;
    for &p in &pts[1..] {
        let s = dir.0 * p.0 as i128 + dir.1 * p.1 as i128;
        if s > bs {
            bs = s;
            best = p;
        }
    }
    best
}

/// Support of the Minkowski difference `a ⊖ b`: the point
/// `sup_a(dir) − sup_b(−dir)`.
fn support_diff(a: &[P], b: &[P], dir: (i128, i128)) -> P {
    let sa = support_pts(a, dir);
    let sb = support_pts(b, (-dir.0, -dir.1));
    (sa.0.wrapping_sub(sb.0), sa.1.wrapping_sub(sb.1))
}

/// Closest point of the current simplex to the origin, with the
/// simplex reduced to the feature carrying it.
fn reduce(simplex: &mut Vec<P>) -> V {
    match simplex.len() {
        1 => V::of(simplex[0]),
        2 => {
            let (a, b) = (simplex[0], simplex[1]);
            let d = sub(b, a);
            let dd = d.0 * d.0 + d.1 * d.1;
            let t = -(a.0 as i128 * d.0 + a.1 as i128 * d.1);
            if dd == 0 || t <= 0 {
                simplex.truncate(1);
                V::of(a)
            } else if t >= dd {
                simplex.remove(0);
                V::of(b)
            } else {
                V {
                    nx: a.0 as i128 * dd + t * d.0,
                    ny: a.1 as i128 * dd + t * d.1,
                    den: dd,
                }
            }
        }
        _ => {
            let (pts, v) = closest_tri(simplex);
            *simplex = pts;
            v
        }
    }
}

/// Squared distance between two convex sets, as an exact
/// rational. `0` iff they touch or overlap. Empty input yields
/// `None`.
pub fn distance2(a: &[P], b: &[P]) -> Option<Frac> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    // seed: any support pair; take direction between centroids
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
            return Some(Frac::new(0, 1));
        }
        // dir = -v : support argmax p·(-v) ⇔ argmin p·v
        let w = support_diff(a, b, (-v.nx, -v.ny));
        // terminate when no improvement: dot(w - v, -v) <= 0
        // ⇔ |v|² <= dot(w,v) ⇔ len2_num/den² <= dot_i/den
        // ⇔ len2_num <= den * dot_i(w)
        if v.len2_num() <= v.den * v.dot_i(w) {
            break;
        }
        simplex.push(w);
        v = reduce(&mut simplex);
    }
    Some(Frac::new(v.len2_num(), v.den * v.den))
}

/// Do the convex sets touch or overlap?
pub fn overlap(a: &[P], b: &[P]) -> bool {
    match distance2(a, b) {
        Some(f) => f.num == 0,
        None => false,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::convex_hull;
    use crate::rng::SplitMix64;
    use crate::sat;
    use crate::segment::segment_dist2;

    /// Oracle: minimum squared distance over all edge pairs, or 0
    /// when `sat` reports contact. Exact for convex polygons.
    /// Point-to-segment squared distance as an exact rational.
    fn point_seg2(p: (i32, i32), a: (i32, i32), b: (i32, i32)) -> Frac {
        let abx = b.0 as i128 - a.0 as i128;
        let aby = b.1 as i128 - a.1 as i128;
        let dd = abx * abx + aby * aby;
        if dd == 0 {
            let dx = p.0 as i128 - a.0 as i128;
            let dy = p.1 as i128 - a.1 as i128;
            return Frac::new(dx * dx + dy * dy, 1);
        }
        let pax = p.0 as i128 - a.0 as i128;
        let pay = p.1 as i128 - a.1 as i128;
        let t = pax * abx + pay * aby;
        if t <= 0 {
            Frac::new(pax * pax + pay * pay, 1)
        } else if t >= dd {
            let pbx = p.0 as i128 - b.0 as i128;
            let pby = p.1 as i128 - b.1 as i128;
            Frac::new(pbx * pbx + pby * pby, 1)
        } else {
            // |pa|² - t²/|ab|² (t = pa·ab), in lowest terms
            Frac::new((pax * pax + pay * pay) * dd - t * t, dd)
        }
    }

    fn seg_seg2(p: (i32, i32), q: (i32, i32), u: (i32, i32), w: (i32, i32)) -> Frac {
        if segment_dist2(p, q, u, w) == 0 {
            return Frac::new(0, 1);
        }
        let mut best = point_seg2(p, u, w);
        for cand in [
            point_seg2(q, u, w),
            point_seg2(u, p, q),
            point_seg2(w, p, q),
        ] {
            if cand.num * best.den < best.num * cand.den {
                best = cand;
            }
        }
        best
    }

    /// Oracle: minimum squared distance over all edge pairs, or 0
    /// when `sat` reports contact. Exact for convex polygons —
    /// the closest feature pair is always a vertex or an
    /// edge-foot, both rational.
    fn brute_dist2(a: &[(i32, i32)], b: &[(i32, i32)]) -> Frac {
        if sat::overlap(a, b) {
            return Frac::new(0, 1);
        }
        let mut best: Option<Frac> = None;
        for i in 0..a.len() {
            let (p, q) = (a[i], a[(i + 1) % a.len()]);
            for j in 0..b.len() {
                let (u, w) = (b[j], b[(j + 1) % b.len()]);
                let d = seg_seg2(p, q, u, w);
                let better = match best {
                    Some(bf) => d.num * bf.den < bf.num * d.den,
                    None => true,
                };
                if better {
                    best = Some(d);
                }
            }
        }
        best.unwrap_or(Frac::new(0, 1))
    }

    fn to_i32(v: &[(i64, i64)]) -> Vec<(i32, i32)> {
        v.iter().map(|&(x, y)| (x as i32, y as i32)).collect()
    }

    /// Random convex polygon from a point cloud via hull.
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

    #[test]
    fn separate_boxes_have_distance() {
        let a = [(0, 0), (10, 0), (10, 10), (0, 10)];
        let b = [(20, 0), (30, 0), (30, 10), (20, 10)];
        let d = distance2(&a, &b);
        assert_eq!(d.map(|f| (f.num, f.den)), Some((100, 1)));
        assert!(!overlap(&a, &b));
    }

    #[test]
    fn overlapping_is_zero() {
        let a = [(0, 0), (10, 0), (10, 10), (0, 10)];
        let b = [(5, 5), (15, 5), (15, 15), (5, 15)];
        assert!(overlap(&a, &b));
        assert_eq!(distance2(&a, &b).map(|f| f.num), Some(0));
    }

    #[test]
    fn touching_is_zero() {
        let a = [(0, 0), (10, 0), (10, 10), (0, 10)];
        let b = [(10, 0), (20, 0), (20, 10), (10, 10)];
        assert!(overlap(&a, &b));
    }

    #[test]
    fn agrees_with_sat_and_brute() {
        let mut r = SplitMix64::new(0x6A4B);
        for _ in 0..400 {
            let a = rand_convex(&mut r, 8, 50);
            let b = rand_convex(&mut r, 8, 50);
            if a.len() < 3 || b.len() < 3 {
                continue;
            }
            let (ai, bi) = (to_i32(&a), to_i32(&b));
            assert_eq!(overlap(&a, &b), sat::overlap(&ai, &bi));
            let got = distance2(&a, &b).unwrap_or(Frac::new(-1, 1));
            let want = brute_dist2(&ai, &bi);
            assert_eq!(
                (got.num, got.den),
                (want.num, want.den),
                "a={a:?} b={b:?} gjk={got:?} brute={want:?}"
            );
        }
    }

    #[test]
    fn point_vs_segment() {
        let a = [(0, 0)];
        let b = [(3, 4), (13, 4)];
        // closest point on segment = (3,4), distance² 25
        let d = distance2(&a, &b);
        assert_eq!(d.map(|f| (f.num, f.den)), Some((25, 1)));
        let c = [(3, 4), (13, 4)];
        let p = [(7, 9)];
        // perpendicular foot (7,4): distance² 25
        assert_eq!(distance2(&p, &c).map(|f| (f.num, f.den)), Some((25, 1)));
    }
}

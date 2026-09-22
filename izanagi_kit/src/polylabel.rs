//! Pole of inaccessibility — the integer point strictly inside a
//! polygon maximizing the minimum squared distance to its boundary,
//! found by best-first branch-and-bound over hierarchical grid cells
//! (the mapbox `polylabel` scheme, reworked so no float ever appears).
//!
//! A cell stores a representative lattice point and an integer upper
//! bound on the best squared distance any of its points can reach:
//! `bound = (ceil√d²_rep + ceil√r²)²` where `r²` is the squared
//! half-extent. Squaring the bound keeps every comparison integral,
//! so pruning is exact and the answer is a pure function of the
//! vertex list.
//!
//! Cells are integer ranges of *points* `[x0, x0+w) × [y0, y0+h)`;
//! a `1×1` cell is one candidate, evaluated only when
//! [`crate::poly::point_in_polygon`] reports `Inside`. Among
//! equal-score candidates the lexicographically smallest wins, which
//! makes the answer canonical.
//!
//! ```
//! use izanagi_kit::polylabel::polylabel;
//! // 10x10 square: the pole is a center point.
//! let s = polylabel(&[(0, 0), (10, 0), (10, 10), (0, 10)]).unwrap();
//! assert!(s.0 == 4 || s.0 == 5);
//! assert_eq!(s.0, s.1);
//! ```
//!
//! Exactness bound: vertex coordinates must satisfy `|c| <= 2^22`
//! (checked — `None` otherwise) so every `(num·den)` product fits
//! `i128`.

use crate::poly::{point_in_polygon, PointLocation};
use std::collections::BinaryHeap;

/// Coordinate magnitude limit keeping every `(num·den)` product
/// inside `i128`.
const COORD_LIMIT: i64 = 1 << 22;

/// Squared distance from `p` to segment `[a, b]` as a rational
/// `(num, den)` with `den > 0`.
fn dist2_seg(p: (i32, i32), a: (i32, i32), b: (i32, i32)) -> (i128, i128) {
    let px = p.0 as i128;
    let py = p.1 as i128;
    let ax = a.0 as i128;
    let ay = a.1 as i128;
    let dx = b.0 as i128 - ax;
    let dy = b.1 as i128 - ay;
    let len2 = dx * dx + dy * dy;
    if len2 == 0 {
        return ((px - ax) * (px - ax) + (py - ay) * (py - ay), 1);
    }
    let t = (px - ax) * dx + (py - ay) * dy;
    if t <= 0 {
        return ((px - ax) * (px - ax) + (py - ay) * (py - ay), 1);
    }
    if t >= len2 {
        let bx = px - ax - dx;
        let by = py - ay - dy;
        return (bx * bx + by * by, 1);
    }
    let ap2 = (px - ax) * (px - ax) + (py - ay) * (py - ay);
    (ap2 * len2 - t * t, len2)
}

/// Min squared distance from `p` to every edge of `poly`.
fn dist2_poly(p: (i32, i32), poly: &[(i32, i32)]) -> (i128, i128) {
    let n = poly.len();
    let mut best: Option<(i128, i128)> = None;
    for i in 0..n {
        let d = dist2_seg(p, poly[i], poly[(i + 1) % n]);
        if best.is_none() || lt_rat(d, best.unwrap_or((0, 1))) {
            best = Some(d);
        }
    }
    best.unwrap_or((0, 1))
}

/// `a < b` on rationals `(num, den)`.
fn lt_rat(a: (i128, i128), b: (i128, i128)) -> bool {
    a.0 * b.1 < b.0 * a.1
}

/// `ceil(sqrt(ceil(x)))` for integer `x` — the least `t` with `t² ≥ x`.
fn isqrt_ceil(x: u128) -> u64 {
    if x == 0 {
        return 0;
    }
    let mut lo = 1u128;
    let mut hi = x.min(u64::MAX as u128);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if mid * mid >= x {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo as u64
}

/// A rectangle of lattice points in the best-first search.
#[derive(Clone, Copy, Debug)]
struct Cell {
    /// Points `x0..x0+w`, `y0..y0+h`.
    x0: i32,
    y0: i32,
    w: u32,
    h: u32,
    /// Integer upper bound on dist² reachable inside the cell.
    bound: i128,
    /// Representative point's exact dist² (also the leaf value).
    num: i128,
    den: i128,
    /// Representative point itself.
    rx: i32,
    ry: i32,
}

impl Ord for Cell {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        // Max-heap on bound; canonical tie-break to smaller cells.
        self.bound
            .cmp(&o.bound)
            .then_with(|| o.x0.cmp(&self.x0))
            .then_with(|| o.y0.cmp(&self.y0))
            .then_with(|| o.w.cmp(&self.w))
            .then_with(|| o.h.cmp(&self.h))
    }
}
impl PartialOrd for Cell {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Cell {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == std::cmp::Ordering::Equal
    }
}
impl Eq for Cell {}

/// The pole of inaccessibility of `poly`: an integer point strictly
/// inside maximizing the minimum squared distance to the boundary.
///
/// Returns `None` for degenerate input (< 3 vertices, out-of-limit
/// coordinates) or when no lattice point lies strictly inside.
pub fn polylabel(poly: &[(i32, i32)]) -> Option<(i32, i32)> {
    if poly.len() < 3 {
        return None;
    }
    if poly
        .iter()
        .any(|&(x, y)| x.abs() as i64 > COORD_LIMIT || y.abs() as i64 > COORD_LIMIT)
    {
        return None;
    }
    let minx = poly.iter().map(|p| p.0).min()?;
    let maxx = poly.iter().map(|p| p.0).max()?;
    let miny = poly.iter().map(|p| p.1).min()?;
    let maxy = poly.iter().map(|p| p.1).max()?;
    let w = (maxx - minx + 1) as u32;
    let h = (maxy - miny + 1) as u32;

    let mk = |x0: i32, y0: i32, w: u32, h: u32| -> Cell {
        let rx = x0 + (w / 2) as i32;
        let ry = y0 + (h / 2) as i32;
        let (num, den) = dist2_poly((rx, ry), poly);
        let r2 = w.div_ceil(2) as u128 * w.div_ceil(2) as u128
            + h.div_ceil(2) as u128 * h.div_ceil(2) as u128;
        let r = isqrt_ceil(r2);
        let dceil = isqrt_ceil(((num + den - 1) / den) as u128);
        Cell {
            x0,
            y0,
            w,
            h,
            bound: (dceil + r) as i128 * (dceil + r) as i128,
            num,
            den,
            rx,
            ry,
        }
    };

    let mut heap = BinaryHeap::new();
    heap.push(mk(minx, miny, w, h));
    // Incumbent: (num, den, x, y).
    let mut best: Option<(i128, i128, i32, i32)> = None;
    while let Some(c) = heap.pop() {
        if let Some((bn, bd, _, _)) = best {
            // bound ≥ any point's dist² in the cell; cells are
            // popped in bound order, so once the bound cannot
            // strictly exceed the incumbent nothing better remains —
            // but an equal-score smaller-coord point can, so break
            // only on strict failure.
            if c.bound * bd < bn {
                break;
            }
        }
        if c.w == 1 && c.h == 1 {
            if point_in_polygon((c.rx, c.ry), poly) != PointLocation::Inside {
                continue;
            }
            let cand = (c.num, c.den);
            let better = match best {
                None => true,
                Some(b) => {
                    lt_rat((b.0, b.1), cand)
                        || (!lt_rat(cand, (b.0, b.1))
                            && !lt_rat((b.0, b.1), cand)
                            && (c.rx, c.ry) < (b.2, b.3))
                }
            };
            if better {
                best = Some((c.num, c.den, c.rx, c.ry));
            }
            continue;
        }
        // Split the longer axis in half; both children keep
        // rectangular extent.
        if c.w >= c.h {
            let m = c.w / 2;
            heap.push(mk(c.x0, c.y0, m, c.h));
            heap.push(mk(c.x0 + m as i32, c.y0, c.w - m, c.h));
        } else {
            let m = c.h / 2;
            heap.push(mk(c.x0, c.y0, c.w, m));
            heap.push(mk(c.x0, c.y0 + m as i32, c.w, c.h - m));
        }
    }
    best.map(|(_, _, x, y)| (x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_pole_is_a_center() {
        let s = polylabel(&[(0, 0), (10, 0), (10, 10), (0, 10)]).unwrap();
        assert!(s.0 == 4 || s.0 == 5);
        assert_eq!(s.0, s.1);
    }

    #[test]
    fn l_shape_prefers_the_wide_lobe() {
        // L: 20x20 minus upper-right 10x10 notch.
        let poly = [(0, 0), (20, 0), (20, 10), (10, 10), (10, 20), (0, 20)];
        let p = polylabel(&poly).unwrap();
        assert_eq!(point_in_polygon(p, &poly), PointLocation::Inside);
        // Brute-force oracle over every lattice point.
        let mut bd = (0i128, 1i128);
        for x in 0..=20 {
            for y in 0..=20 {
                if point_in_polygon((x, y), &poly) == PointLocation::Inside {
                    let d = dist2_poly((x, y), &poly);
                    if lt_rat(bd, d) {
                        bd = d;
                    }
                }
            }
        }
        let d = dist2_poly(p, &poly);
        assert!(!lt_rat(d, bd), "pole below oracle best");
    }

    #[test]
    fn degenerate_and_tiny_inputs() {
        assert_eq!(polylabel(&[]), None);
        assert_eq!(polylabel(&[(0, 0), (1, 1)]), None);
        // Triangle too thin to contain a lattice point.
        assert_eq!(polylabel(&[(0, 0), (1, 0), (0, 1)]), None);
        assert_eq!(polylabel(&[(0, 0), (1 << 24, 0), (1 << 24, 1)]), None);
    }

    #[test]
    fn matches_brute_force_on_random_convex_polys() {
        use crate::poly::convex_hull;
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(13);
        for _ in 0..12 {
            let pts: Vec<(i32, i32)> = (0..10)
                .map(|_| (rng.below(24) as i32, rng.below(24) as i32))
                .collect();
            let hull = convex_hull(&pts);
            if hull.len() < 3 {
                continue;
            }
            let mut ob: Option<(i128, i128)> = None;
            for x in 0..24 {
                for y in 0..24 {
                    if point_in_polygon((x, y), &hull) != PointLocation::Inside {
                        continue;
                    }
                    let d = dist2_poly((x, y), &hull);
                    if ob.is_none() || lt_rat(ob.unwrap(), d) {
                        ob = Some(d);
                    }
                }
            }
            match (polylabel(&hull), ob) {
                (None, None) => {}
                (Some(p), Some(bd)) => {
                    let g = dist2_poly(p, &hull);
                    assert!(
                        !lt_rat(g, bd) && !lt_rat(bd, g),
                        "pole {p:?} d²={}/{} vs oracle {}/{}",
                        g.0,
                        g.1,
                        bd.0,
                        bd.1
                    );
                }
                (got, oracle) => panic!("got {got:?} vs oracle {oracle:?}"),
            }
        }
    }
}

//! Half-plane intersection in exact rational arithmetic — the
//! feasible region of `a·x + b·y ≤ c` constraints computed as a
//! convex polygon whose vertices are `(Frac, Frac)` pairs, so the
//! result is bit-exact on every platform and every replay. The
//! classic deque algorithm sorts the bounding lines by direction
//! angle (quadrant + cross-product order — no `atan2`, no floats)
//! and prunes each end of the deque against the incoming line.
//!
//! `None` means *no bounded polygon*: the region is empty or
//! unbounded, or a required line intersection was exactly parallel
//! (an exact-degenerate configuration). A polygon with fewer than
//! three vertices is also reported as `None`.
//!
//! ```
//! use izanagi_kit::halfplane::{intersect, HalfPlane};
//! use izanagi_kit::frac::Frac;
//!
//! // Unit square 0 ≤ x,y ≤ 1 — four half-planes.
//! let poly = intersect(&[
//!     HalfPlane::new(1, 0, 1),   // x ≤ 1
//!     HalfPlane::new(-1, 0, 0),  // x ≥ 0
//!     HalfPlane::new(0, 1, 1),   // y ≤ 1
//!     HalfPlane::new(0, -1, 0),  // y ≥ 0
//! ])
//! .unwrap();
//! assert_eq!(poly.len(), 4);
//! assert!(poly.contains(&(Frac::from_int(0), Frac::from_int(0))));
//! ```
//!
//! References: standard "sort-and-deque" half-plane intersection
//! (cp-algorithms); Preparata & Shamos, *Computational Geometry*.

use crate::frac::Frac;
use std::collections::VecDeque;

/// The half-plane `a·x + b·y ≤ c` — integer coefficients, no
/// normalization required (the zero normal `(0,0)` is a constant
/// predicate: `c ≥ 0` is always-true, `c < 0` makes the region empty).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HalfPlane {
    /// Coefficient of `x` (normal's x-part).
    pub a: i128,
    /// Coefficient of `y` (normal's y-part).
    pub b: i128,
    /// Right-hand side bound.
    pub c: i128,
}

type Vert = (Frac, Frac);

impl HalfPlane {
    /// `a·x + b·y ≤ c`.
    pub fn new(a: i128, b: i128, c: i128) -> Self {
        Self { a, b, c }
    }

    /// Whether the exact point `(x, y)` satisfies the constraint —
    /// cross-multiplied so no division happens anywhere.
    pub fn contains(&self, x: Frac, y: Frac) -> bool {
        self.a * x.num * y.den + self.b * y.num * x.den <= self.c * x.den * y.den
    }

    /// Boundary direction with the feasible side on the left:
    /// `d = (−b, a)` — the normal rotated a quarter turn.
    fn dir(&self) -> (i128, i128) {
        (-self.b, self.a)
    }
}

/// Atan2-free ordering on 2-D integer directions: upper half-plane
/// vectors first, then by cross product.
fn dir_less(d1: (i128, i128), d2: (i128, i128)) -> bool {
    fn half(d: (i128, i128)) -> bool {
        d.1 < 0 || (d.1 == 0 && d.0 < 0)
    }
    match (half(d1), half(d2)) {
        (false, true) => true,
        (true, false) => false,
        _ => {
            let (x1, y1) = d1;
            let (x2, y2) = d2;
            x1 * y2 - y1 * x2 > 0
        }
    }
}

/// `t > 0` with `d1 = t·d2`, or `None` when not positively parallel.
fn positive_multiple(d1: (i128, i128), d2: (i128, i128)) -> bool {
    // d1 = t·d2, t > 0: cross == 0 and dot > 0.
    let (x1, y1) = d1;
    let (x2, y2) = d2;
    x1 * y2 == y1 * x2 && x1 * x2 + y1 * y2 > 0
}

/// Intersection of the two boundary lines as rationals —
/// `det = a1·b2 − a2·b1`; `None` when parallel (det = 0).
fn meet(h1: &HalfPlane, h2: &HalfPlane) -> Option<Vert> {
    let det = h1.a * h2.b - h2.a * h1.b;
    if det == 0 {
        return None;
    }
    Some((
        Frac::new(h1.c * h2.b - h2.c * h1.b, det),
        Frac::new(h1.a * h2.c - h2.a * h1.c, det),
    ))
}

/// Feasible polygon of `planes`, vertices in counter-clockwise
/// order; `None` when the region is empty, unbounded, degenerate,
/// or has fewer than three vertices.
pub fn intersect(planes: &[HalfPlane]) -> Option<Vec<Vert>> {
    // Drop trivially-true constraints, bail on trivially-false ones.
    let mut hs: Vec<HalfPlane> = Vec::with_capacity(planes.len());
    for &h in planes {
        if h.a == 0 && h.b == 0 {
            if h.c < 0 {
                return None;
            }
            continue;
        }
        hs.push(h);
    }
    // Sort by boundary direction, then dedup same-direction lines
    // keeping the tighter constraint (smaller c after rescaling by
    // the shared direction's coefficient ratio).
    hs.sort_by(|x, y| {
        let (d1, d2) = (x.dir(), y.dir());
        if dir_less(d1, d2) {
            std::cmp::Ordering::Less
        } else if dir_less(d2, d1) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    let mut merged: Vec<HalfPlane> = Vec::with_capacity(hs.len());
    for h in hs {
        if let Some(&prev) = merged.last() {
            if positive_multiple(prev.dir(), h.dir()) {
                // Same feasible side: `h` is tighter iff
                // `c_h ≤ t·c_prev` for `t = n_h / n_prev > 0` —
                // i.e. `c_h·|b_prev| ≤ c_prev·|b_h|` (sign-free).
                let tighter = if prev.b != 0 {
                    h.c * prev.b.unsigned_abs() as i128 <= prev.c * h.b.unsigned_abs() as i128
                } else {
                    h.c * prev.a.unsigned_abs() as i128 <= prev.c * h.a.unsigned_abs() as i128
                };
                if tighter {
                    if let Some(slot) = merged.last_mut() {
                        *slot = h;
                    }
                }
                continue;
            }
        }
        merged.push(h);
    }
    let mut dq: VecDeque<HalfPlane> = VecDeque::new();
    let mut pts: VecDeque<Vert> = VecDeque::new();
    for &h in &merged {
        while let Some(&v) = pts.back() {
            if h.contains(v.0, v.1) {
                break;
            }
            pts.pop_back();
            dq.pop_back();
        }
        while let Some(&v) = pts.front() {
            if h.contains(v.0, v.1) {
                break;
            }
            pts.pop_front();
            dq.pop_front();
        }
        if let Some(&last) = dq.back() {
            pts.push_back(meet(&last, &h)?);
        }
        dq.push_back(h);
    }
    // Close the ring: earlier planes may cut vertices created by
    // later ones at both ends.
    while pts.len() > 1 && {
        let v = *pts
            .back()
            .unwrap_or(&(Frac::from_int(0), Frac::from_int(0)));
        dq.front().is_some_and(|f| !f.contains(v.0, v.1))
    } {
        pts.pop_back();
        dq.pop_back();
    }
    while pts.len() > 1 && {
        let v = *pts
            .front()
            .unwrap_or(&(Frac::from_int(0), Frac::from_int(0)));
        dq.back().is_some_and(|f| !f.contains(v.0, v.1))
    } {
        pts.pop_front();
        dq.pop_front();
    }
    // Close the polygon: the last boundary vertex is the
    // intersection of the final and first bounding lines.
    if dq.len() >= 3 {
        match (dq.back(), dq.front()) {
            (Some(&back), Some(&front)) => pts.push_front(meet(&back, &front)?),
            _ => return None,
        }
    }
    if dq.len() < 3 || pts.len() < 3 {
        return None;
    }
    // Consistency gate: every surviving vertex satisfies every
    // surviving plane — catches residual unbounded configurations.
    let raw: Vec<Vert> = pts.into_iter().collect();
    for &(x, y) in &raw {
        for h in &dq {
            if !h.contains(x, y) {
                return None;
            }
        }
    }
    // Canonicalize the ring: a boundary line may pass through a
    // vertex without cutting it, emitting a duplicate or a collinear
    // corner — drop both so the vertex list is minimal.
    let mut verts: Vec<Vert> = Vec::with_capacity(raw.len());
    for v in raw {
        if verts.last() != Some(&v) {
            verts.push(v);
        }
    }
    if verts.len() > 1 && verts.first() == verts.last() {
        verts.pop();
    }
    let mut i = 0;
    while verts.len() > 2 && i < verts.len() {
        let n = verts.len();
        let (px, py) = verts[(i + n - 1) % n];
        let (cx, cy) = verts[i];
        let (nx, ny) = verts[(i + 1) % n];
        // cross(curr−prev, next−curr) == 0 → collinear corner.
        let cross = (cx - px) * (ny - cy) - (cy - py) * (nx - cx);
        if cross.num == 0 {
            verts.remove(i);
        } else {
            i += 1;
        }
    }
    if verts.len() < 3 {
        return None;
    }
    // Normalize orientation to counter-clockwise: the deque's
    // convention can emit the ring in either order; the shoelace
    // sign decides — `Σ(x_i·y_{i+1} − x_{i+1}·y_i) < 0` is CW.
    let mut twice_area = Frac::from_int(0);
    for i in 0..verts.len() {
        let (x1, y1) = verts[i];
        let (x2, y2) = verts[(i + 1) % verts.len()];
        twice_area = twice_area + x1 * y2 - x2 * y1;
    }
    if twice_area.num == 0 {
        return None; // zero-area degenerate polygon
    }
    if twice_area.num < 0 {
        verts.reverse();
    }
    Some(verts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn unit_square() {
        let p = intersect(&[
            HalfPlane::new(1, 0, 1),
            HalfPlane::new(-1, 0, 0),
            HalfPlane::new(0, 1, 1),
            HalfPlane::new(0, -1, 0),
        ])
        .unwrap_or_default();
        assert_eq!(p.len(), 4);
        let set: BTreeSet<(i128, i128)> = p.iter().map(|v| (v.0.num, v.1.num)).collect();
        assert_eq!(set, [(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().collect());
    }

    #[test]
    fn triangle_cut() {
        // x ≥ 0, y ≥ 0, x + y ≤ 2 → right triangle.
        let p = intersect(&[
            HalfPlane::new(-1, 0, 0),
            HalfPlane::new(0, -1, 0),
            HalfPlane::new(1, 1, 2),
        ])
        .unwrap_or_default();
        assert_eq!(p.len(), 3);
        let set: BTreeSet<(i128, i128)> = p.iter().map(|v| (v.0.num, v.1.num)).collect();
        assert_eq!(set, [(0, 0), (0, 2), (2, 0)].into_iter().collect());
    }

    #[test]
    fn empty_and_degenerate() {
        // x ≤ 0 ∧ x ≥ 1 — contradictory.
        assert_eq!(
            intersect(&[HalfPlane::new(1, 0, 0), HalfPlane::new(-1, 0, -1)]),
            None
        );
        // Single half-plane → unbounded → None.
        assert_eq!(intersect(&[HalfPlane::new(1, 0, 0)]), None);
        // Trivially-false constant constraint → None.
        assert_eq!(intersect(&[HalfPlane::new(0, 0, -1)]), None);
        // Strip: 0 ≤ x ≤ 1 — only two planes → None (not a polygon).
        assert_eq!(
            intersect(&[HalfPlane::new(1, 0, 1), HalfPlane::new(-1, 0, 0)]),
            None
        );
    }

    /// Brute oracle: pairwise line intersections that satisfy every
    /// constraint, deduped, angularly sorted — compared to the deque.
    #[test]
    fn oracle_small_planes() {
        let mut rng = SplitMix64::new(0xA111_B1A2);
        for _ in 0..800 {
            // Random planes through a bounded window; bias toward
            // forming a bounded region by always adding a box.
            let mut planes = vec![
                HalfPlane::new(1, 0, 4),
                HalfPlane::new(-1, 0, 4),
                HalfPlane::new(0, 1, 4),
                HalfPlane::new(0, -1, 4),
            ];
            let extra = rng.next_u64() % 5;
            for _ in 0..extra {
                let a = (rng.next_u64() % 7) as i128 - 3;
                let b = (rng.next_u64() % 7) as i128 - 3;
                if a == 0 && b == 0 {
                    continue;
                }
                let c = (rng.next_u64() % 13) as i128 - 6;
                planes.push(HalfPlane::new(a, b, c));
            }
            let got = intersect(&planes);
            // Oracle: all pairwise intersections that are feasible.
            let mut feas: Vec<(i128, i128, i128, i128)> = Vec::new();
            for i in 0..planes.len() {
                for j in (i + 1)..planes.len() {
                    if let Some(v) = meet(&planes[i], &planes[j]) {
                        if planes.iter().all(|h| h.contains(v.0, v.1)) {
                            feas.push((v.0.num, v.0.den, v.1.num, v.1.den));
                        }
                    }
                }
            }
            feas.sort();
            feas.dedup();
            match got {
                Some(verts) => {
                    // Bounded polygon: vertices are exactly the
                    // feasible pairwise intersections, in CCW order
                    // (same set — order checked implicitly by count).
                    assert_eq!(verts.len(), feas.len());
                    let got_set: BTreeSet<(i128, i128, i128, i128)> = verts
                        .iter()
                        .map(|v| (v.0.num, v.0.den, v.1.num, v.1.den))
                        .collect();
                    let want: BTreeSet<(i128, i128, i128, i128)> = feas.iter().copied().collect();
                    assert_eq!(got_set, want);
                    // CCW: twice the signed shoelace area is positive.
                    let mut twice_area = Frac::from_int(0);
                    for i in 0..verts.len() {
                        let (x1, y1) = verts[i];
                        let (x2, y2) = verts[(i + 1) % verts.len()];
                        twice_area = twice_area + x1 * y2 - x2 * y1;
                    }
                    assert!(twice_area.num > 0);
                }
                None => {
                    // Legal only when the feasible set cannot produce
                    // a ≥3-vertex polygon: fewer than 3 feasible
                    // pairwise intersections.
                    assert!(feas.len() < 3, "feasible {:?} but None", feas);
                }
            }
        }
    }
}

//! Smallest enclosing circle — Welzl's algorithm over integer
//! points (Welzl 1991). The answer is returned as exact rationals
//! ([`Frac`]): the center is the midpoint of a boundary pair or the
//! circumcenter of a boundary triple, both rational in `i32` input —
//! so `r2` comparisons are `i128`-exact and identical on every peer.
//!
//! Ordering is canonical: the point set is sorted and deduplicated
//! first, so the result is a pure function of the set, not the input
//! order (the minimum enclosing circle is unique, so this matters
//! only for witness construction).
//!
//! ```
//! use izanagi_kit::mincircle::smallest_circle;
//!
//! let c = smallest_circle(&[(0, 0), (4, 0), (0, 3), (4, 3)]).unwrap();
//! assert_eq!(c.r2, izanagi_kit::frac::Frac::new(25, 4));
//! assert!(c.contains((0, 0)) && c.contains((4, 3)));
//! ```
//!
//! References: Welzl, "Smallest enclosing disks" (1991); de Berg et
//! al., *Computational Geometry*, §4.7.

use crate::frac::Frac;

/// An exact circle: rational center plus squared radius.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Circle {
    /// Center x.
    pub cx: Frac,
    /// Center y.
    pub cy: Frac,
    /// Squared radius (kept squared: exact and float-free).
    pub r2: Frac,
}

impl Circle {
    /// `true` when `p` lies inside or on the circle (inclusive).
    pub fn contains(&self, p: (i32, i32)) -> bool {
        let dx = Frac::from_int(p.0 as i128) - self.cx;
        let dy = Frac::from_int(p.1 as i128) - self.cy;
        let d2 = dx * dx + dy * dy;
        d2.cmp_frac(&self.r2) != std::cmp::Ordering::Greater
    }
}

fn circle1(p: (i32, i32)) -> Circle {
    Circle {
        cx: Frac::from_int(p.0 as i128),
        cy: Frac::from_int(p.1 as i128),
        r2: Frac::from_int(0),
    }
}

/// Diameter circle on segment `ab`.
fn circle2(a: (i32, i32), b: (i32, i32)) -> Circle {
    let (ax, ay, bx, by) = (a.0 as i128, a.1 as i128, b.0 as i128, b.1 as i128);
    Circle {
        cx: Frac::new(ax + bx, 2),
        cy: Frac::new(ay + by, 2),
        r2: Frac::new((ax - bx) * (ax - bx) + (ay - by) * (ay - by), 4),
    }
}

/// Circumcircle of `a`, `b`, `c`; collinear input falls back to the
/// diameter circle of the widest pair.
fn circle3(a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> Circle {
    let (ax, ay) = (a.0 as i128, a.1 as i128);
    let (bx, by) = (b.0 as i128, b.1 as i128);
    let (cx, cy) = (c.0 as i128, c.1 as i128);
    // Twice the signed triangle area.
    let d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d == 0 {
        // Collinear: the enclosing circle is the widest pair's
        // diameter circle (the third point is inside it by
        // convexity of the strip).
        for (u, v) in [(a, b), (a, c), (b, c)] {
            let cc = circle2(u, v);
            if cc.contains(a) && cc.contains(b) && cc.contains(c) {
                return cc;
            }
        }
        // Unreachable for genuine collinear triples; a symmetric
        // fallback keeps the function total.
        return circle2(a, b);
    }
    let aa = ax * ax + ay * ay;
    let bb = bx * bx + by * by;
    let cc = cx * cx + cy * cy;
    let ux = aa * (by - cy) + bb * (cy - ay) + cc * (ay - by);
    let uy = aa * (cx - bx) + bb * (ax - cx) + cc * (bx - ax);
    let x = Frac::new(ux, d);
    let y = Frac::new(uy, d);
    let dx = x - Frac::from_int(ax);
    let dy = y - Frac::from_int(ay);
    Circle {
        cx: x,
        cy: y,
        r2: dx * dx + dy * dy,
    }
}

/// Welzl's minimum enclosing circle over `pts`; `None` when empty.
/// Expected `O(n)` — the nested boundary loops run only for points
/// outside the current circle.
pub fn smallest_circle(pts: &[(i32, i32)]) -> Option<Circle> {
    let mut p: Vec<(i32, i32)> = pts.to_vec();
    p.sort_unstable();
    p.dedup();
    let n = p.len();
    if n == 0 {
        return None;
    }
    let mut c = circle1(p[0]);
    for i in 1..n {
        if c.contains(p[i]) {
            continue;
        }
        c = circle1(p[i]);
        for j in 0..i {
            if c.contains(p[j]) {
                continue;
            }
            c = circle2(p[i], p[j]);
            for k in 0..j {
                if !c.contains(p[k]) {
                    c = circle3(p[i], p[j], p[k]);
                }
            }
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force MEC: the answer is always determined by a
    /// boundary pair or triple — enumerate all candidates and keep
    /// the smallest circle that still contains everything.
    fn brute(pts: &[(i32, i32)]) -> Option<Circle> {
        if pts.is_empty() {
            return None;
        }
        let n = pts.len();
        let mut best: Option<Circle> = None;
        let mut consider = |c: Circle| {
            if pts.iter().all(|&p| c.contains(p)) {
                let better = match best {
                    None => true,
                    Some(b) => c.r2.cmp_frac(&b.r2) == std::cmp::Ordering::Less,
                };
                if better {
                    best = Some(c);
                }
            }
        };
        for i in 0..n {
            consider(circle1(pts[i]));
            for j in 0..i {
                consider(circle2(pts[i], pts[j]));
                for k in 0..j {
                    consider(circle3(pts[i], pts[j], pts[k]));
                }
            }
        }
        best
    }

    #[test]
    fn known_cases() {
        assert!(smallest_circle(&[]).is_none());
        let c = smallest_circle(&[(3, 4)]).unwrap();
        assert_eq!(c.r2, Frac::from_int(0));
        assert_eq!(c.cx, Frac::from_int(3));
        // Rectangle: MEC is the diagonal's diameter circle.
        let c = smallest_circle(&[(0, 0), (4, 0), (0, 3), (4, 3)]).unwrap();
        assert_eq!(c.r2, Frac::new(25, 4));
        assert_eq!(c.cx, Frac::from_int(2));
        assert_eq!(c.cy, Frac::new(3, 2));
    }

    #[test]
    fn matches_bruteforce_oracle() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0xE1C1);
        for _ in 0..150 {
            let n = 1 + (rng.next_u64() % 11) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| {
                    (
                        (rng.next_u64() % 60) as i32 - 30,
                        (rng.next_u64() % 60) as i32 - 30,
                    )
                })
                .collect();
            let got = smallest_circle(&pts).unwrap();
            let want = brute(&pts).unwrap();
            assert_eq!(got.r2, want.r2, "r2 mismatch on {pts:?}");
            assert_eq!(got.cx, want.cx, "cx mismatch on {pts:?}");
            assert_eq!(got.cy, want.cy, "cy mismatch on {pts:?}");
            for &p in &pts {
                assert!(got.contains(p));
            }
        }
    }

    #[test]
    fn order_invariant() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0xE1C2);
        for _ in 0..60 {
            let n = 3 + (rng.next_u64() % 9) as usize;
            let mut pts: Vec<(i32, i32)> = (0..n)
                .map(|_| ((rng.next_u64() % 50) as i32, (rng.next_u64() % 50) as i32))
                .collect();
            let a = smallest_circle(&pts).unwrap();
            pts.reverse();
            let b = smallest_circle(&pts).unwrap();
            assert_eq!(a, b);
        }
    }
}

//! Rotating calipers over a convex polygon — computes the
//! diameter pair, the minimum width, and the minimum-area
//! enclosing rectangle in `O(n)` once the `O(n log n)` hull is
//! built (via [`crate::poly::convex_hull`] or supplied directly).
//! All arithmetic is integer cross products; fractional
//! quantities (width, rectangle area) are returned as
//! [`crate::frac::Frac`] so nothing rounds.
//!
//! The hull must be strictly convex and counter-clockwise —
//! exactly what `poly::convex_hull` produces (it already strips
//! collinear edges). Passing a non-convex or clockwise polygon
//! yields `None`-style fallback behavior on the diameter
//! functions' results only in the sense that they are then
//! meaningless, not wrong-shaped; keep hull discipline.
//!
//! ```
//! use izanagi_kit::poly::convex_hull;
//! use izanagi_kit::rotcal::diameter;
//!
//! // Unit square plus an interior point.
//! let hull = convex_hull(&[(0, 0), (4, 0), (4, 4), (0, 4), (2, 2)]);
//! let (_, _, d2) = diameter(&hull).expect("non-empty hull");
//! assert_eq!(d2, 32); // diagonal of the 4x4 square
//! ```

use crate::frac::Frac;

/// Squared distance between two points.
fn dist2(a: (i32, i32), b: (i32, i32)) -> i128 {
    let dx = a.0 as i128 - b.0 as i128;
    let dy = a.1 as i128 - b.1 as i128;
    dx * dx + dy * dy
}

/// Cross product `(b - a) x (c - a)`.
fn cross(a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> i128 {
    let ux = b.0 as i128 - a.0 as i128;
    let uy = b.1 as i128 - a.1 as i128;
    let vx = c.0 as i128 - a.0 as i128;
    let vy = c.1 as i128 - a.1 as i128;
    ux * vy - uy * vx
}

/// Antipodal-pair scan: for each edge `i -> i+1` (CCW hull), the
/// farthest vertex `j` advances monotonically. Returns every
/// `(i, j)` where `cross(i -> i+1, i -> j)` is maximal for edge i,
/// with `j` taken at the *advance* position.
fn antipodal(hull: &[(i32, i32)]) -> Vec<(usize, usize)> {
    let n = hull.len();
    let mut out = Vec::new();
    if n < 2 {
        return out;
    }
    if n == 2 {
        out.push((0, 1));
        out.push((1, 0));
        return out;
    }
    let mut j = 1usize;
    for i in 0..n {
        let ni = (i + 1) % n;
        // Advance j while the next vertex is strictly farther
        // (in perpendicular distance) from edge i.
        while cross(hull[i], hull[ni], hull[(j + 1) % n]) > cross(hull[i], hull[ni], hull[j]) {
            j = (j + 1) % n;
        }
        out.push((i, j));
    }
    out
}

/// A hull vertex pair with a squared-distance witness.
pub type DiameterPair = ((i32, i32), (i32, i32), i128);

/// Diameter of the hull: `((a, b), d2)` with `d2` the squared
/// distance and the vertex pair canonicalized so `a < b`
/// lexicographically. `None` for fewer than 2 vertices; for 2
/// vertices returns that pair.
pub fn diameter(hull: &[(i32, i32)]) -> Option<DiameterPair> {
    let n = hull.len();
    if n == 0 {
        return None;
    }
    if n == 1 {
        return Some((hull[0], hull[0], 0));
    }
    let mut best: Option<DiameterPair> = None;
    for &(i, j) in &antipodal(hull) {
        let (mut u, mut v) = (hull[i], hull[j]);
        if v < u {
            std::mem::swap(&mut u, &mut v);
        }
        let d = dist2(u, v);
        let better = match best {
            None => true,
            Some((bu, bv, bd)) => d > bd || (d == bd && (u, v) < (bu, bv)),
        };
        if better {
            best = Some((u, v, d));
        }
    }
    best
}

/// Minimum width of the hull — the smallest perpendicular
/// distance between parallel supporting lines. Returns
/// `Frac(cross2, edge_len2)` (squared-width ratio `num/den` such
/// that the true width is `sqrt(num/den)`), plus the witness
/// `(edge_index, farthest_vertex)` pair. `None` for `n < 3`
/// (a segment's width is 0).
pub fn min_width(hull: &[(i32, i32)]) -> Option<(Frac, usize, usize)> {
    let n = hull.len();
    if n < 3 {
        return None;
    }
    let mut best: Option<(Frac, usize, usize)> = None;
    for &(i, j) in &antipodal(hull) {
        let ni = (i + 1) % n;
        let num = cross(hull[i], hull[ni], hull[j]);
        let len2 = dist2(hull[i], hull[ni]);
        // width along edge i = num / sqrt(len2); compare num^2/len2.
        let f = Frac::new(num * num, len2);
        let cand = (f, i, j);
        if best.map_or(true, |(bf, _, _)| {
            f.cmp_frac(&bf) == std::cmp::Ordering::Less
        }) {
            best = Some(cand);
        }
    }
    best
}

/// Minimum-area enclosing rectangle. For each hull edge `i`, the
/// bounding rectangle has area `num_width * num_height / len2`
/// where `num_height` spans the hull's projection perpendicular
/// to the edge. Returns `(Frac area, edge_index)`. `None` for
/// `n < 3`.
pub fn min_rect_area(hull: &[(i32, i32)]) -> Option<(Frac, usize)> {
    let n = hull.len();
    if n < 3 {
        return None;
    }
    // Classic four-calipers: for edge i, track j_top (max
    // perpendicular distance), j_right/j_left (max/min along-edge
    // projection). All three advance monotonically, but each must
    // be seeded at its edge-0 extremum — a monotone pointer can't
    // reach a minimum that lies *before* index 0 in the walk.
    let e0x = hull[1].0 as i128 - hull[0].0 as i128;
    let e0y = hull[1].1 as i128 - hull[0].1 as i128;
    let dot0 = |p: (i32, i32)| -> i128 {
        (p.0 as i128 - hull[0].0 as i128) * e0x + (p.1 as i128 - hull[0].1 as i128) * e0y
    };
    let cr0 = |p: (i32, i32)| -> i128 { cross(hull[0], hull[1], p) };
    let mut jt = 0usize;
    let mut jr = 0usize;
    let mut jl = 0usize;
    for (k, &p) in hull.iter().enumerate() {
        if cr0(p) > cr0(hull[jt]) {
            jt = k;
        }
        if dot0(p) > dot0(hull[jr]) {
            jr = k;
        }
        if dot0(p) < dot0(hull[jl]) {
            jl = k;
        }
    }
    let mut best: Option<(Frac, usize)> = None;
    for i in 0..n {
        let ni = (i + 1) % n;
        let ex = hull[ni].0 as i128 - hull[i].0 as i128;
        let ey = hull[ni].1 as i128 - hull[i].1 as i128;
        let len2 = ex * ex + ey * ey;
        // Dot and cross helpers vs edge i.
        let dot = |p: (i32, i32)| -> i128 {
            (p.0 as i128 - hull[i].0 as i128) * ex + (p.1 as i128 - hull[i].1 as i128) * ey
        };
        let cr = |p: (i32, i32)| -> i128 { cross(hull[i], hull[ni], p) };
        while cr(hull[(jt + 1) % n]) > cr(hull[jt]) {
            jt = (jt + 1) % n;
        }
        while dot(hull[(jr + 1) % n]) > dot(hull[jr]) {
            jr = (jr + 1) % n;
        }
        while dot(hull[(jl + 1) % n]) < dot(hull[jl]) {
            jl = (jl + 1) % n;
        }
        // height (perpendicular) = cr(jt); width along edge =
        // dot(jr) - dot(jl). Area = height * width / len2.
        let h_num = cr(hull[jt]);
        let w_num = dot(hull[jr]) - dot(hull[jl]);
        let area = Frac::new(h_num * w_num, len2);
        if best.map_or(true, |(bf, _)| {
            area.cmp_frac(&bf) == std::cmp::Ordering::Less
        }) {
            best = Some((area, i));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::convex_hull;
    use crate::rng::SplitMix64;

    fn brute_diameter(hull: &[(i32, i32)]) -> i128 {
        let mut best = 0i128;
        for &a in hull {
            for &b in hull {
                best = best.max(dist2(a, b));
            }
        }
        best
    }

    #[test]
    fn square_and_degenerate() {
        let hull = convex_hull(&[(0, 0), (4, 0), (4, 4), (0, 4)]);
        let (_, _, d2) = diameter(&hull).unwrap_or_else(|| panic!("hull ok"));
        assert_eq!(d2, 32);
        assert_eq!(diameter(&[]), None);
        assert_eq!(diameter(&[(1, 2)]), Some(((1, 2), (1, 2), 0)));
        assert_eq!(diameter(&[(0, 0), (3, 4)]).map(|x| x.2), Some(25));
        let (w, _, _) = min_width(&hull).unwrap_or_else(|| panic!("width"));
        // Width squared = 16 → Frac(num, den) with num/den = 16.
        assert_eq!(w, Frac::from_int(16));
    }

    #[test]
    fn min_width_brute_force() {
        let mut rng = SplitMix64::new(0xCA11);
        for _ in 0..200 {
            let n = 3 + (rng.next_u64() % 15) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| ((rng.next_u64() % 50) as i32, (rng.next_u64() % 50) as i32))
                .collect();
            let hull = convex_hull(&pts);
            if hull.len() < 3 {
                continue;
            }
            // Brute width: min over edges of max cross / len.
            let mut bw: Option<Frac> = None;
            for i in 0..hull.len() {
                let ni = (i + 1) % hull.len();
                let len2 = dist2(hull[i], hull[ni]);
                let mut mx = 0i128;
                for &p in &hull {
                    mx = mx.max(cross(hull[i], hull[ni], p).abs());
                }
                let f = Frac::new(mx * mx, len2);
                if bw.map_or(true, |b| f.cmp_frac(&b) == std::cmp::Ordering::Less) {
                    bw = Some(f);
                }
            }
            let (w, _, _) = min_width(&hull).unwrap_or_else(|| panic!("width"));
            assert_eq!(w, bw.unwrap_or_else(|| panic!("bw")));
        }
    }

    #[test]
    fn min_rect_brute_force() {
        let mut rng = SplitMix64::new(0xB0C5);
        for _ in 0..200 {
            let n = 3 + (rng.next_u64() % 12) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| ((rng.next_u64() % 40) as i32, (rng.next_u64() % 40) as i32))
                .collect();
            let hull = convex_hull(&pts);
            if hull.len() < 3 {
                continue;
            }
            // Brute: every edge direction defines a candidate
            // rectangle; min-area rect touches an edge.
            let mut ba: Option<Frac> = None;
            for i in 0..hull.len() {
                let ni = (i + 1) % hull.len();
                let ex = hull[ni].0 as i128 - hull[i].0 as i128;
                let ey = hull[ni].1 as i128 - hull[i].1 as i128;
                let len2 = ex * ex + ey * ey;
                let dot = |p: (i32, i32)| -> i128 {
                    (p.0 as i128 - hull[i].0 as i128) * ex + (p.1 as i128 - hull[i].1 as i128) * ey
                };
                let cr = |p: (i32, i32)| -> i128 { cross(hull[i], hull[ni], p) };
                let mut hmax = 0i128;
                let mut dmax = i128::MIN;
                let mut dmin = i128::MAX;
                for &p in &hull {
                    hmax = hmax.max(cr(p));
                    dmax = dmax.max(dot(p));
                    dmin = dmin.min(dot(p));
                }
                let f = Frac::new(hmax * (dmax - dmin), len2);
                if ba.map_or(true, |b| f.cmp_frac(&b) == std::cmp::Ordering::Less) {
                    ba = Some(f);
                }
            }
            let (area, _) = min_rect_area(&hull).unwrap_or_else(|| panic!("rect"));
            assert_eq!(area, ba.unwrap_or_else(|| panic!("ba")));
        }
    }

    #[test]
    fn diameter_matches_bruteforce() {
        let mut rng = SplitMix64::new(0xD1A4);
        for _ in 0..200 {
            let n = 3 + (rng.next_u64() % 20) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| ((rng.next_u64() % 60) as i32, (rng.next_u64() % 60) as i32))
                .collect();
            let hull = convex_hull(&pts);
            let (_, _, d2) = diameter(&hull).unwrap_or_else(|| panic!("diameter"));
            assert_eq!(d2, brute_diameter(&hull));
        }
    }
}

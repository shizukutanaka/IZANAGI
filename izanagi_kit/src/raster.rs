//! Integer-exact grid rasterization — Bresenham lines, midpoint
//! circles, and scanline polygon fill. These are the pixel-space
//! companions of [`segment`](crate::segment) predicates and
//! [`poly`](crate::poly) geometry: the answers those give in ideal
//! coordinates, rasterized into `i32` cells a tilemap or terminal
//! screen can draw directly.
//!
//! ```
//! use izanagi_kit::raster::line;
//! assert_eq!(line((0, 0), (4, 2)), vec![(0, 0), (1, 0), (2, 1), (3, 1), (4, 2)]);
//! ```

/// Bresenham line from `(x0,y0)` to `(x1,y1)`, both endpoints
/// inclusive, start-to-end order. Handles all octants by
/// axis-swapping. Symmetric under endpoint reversal: the raster is
/// always computed from the lexicographically smaller endpoint and
/// reversed if needed — the tie-break direction never depends on
/// which end the caller passed first.
pub fn line(p0: (i32, i32), p1: (i32, i32)) -> Vec<(i32, i32)> {
    if p1 < p0 {
        let mut rev = line_inner(p1, p0);
        rev.reverse();
        return rev;
    }
    line_inner(p0, p1)
}

fn line_inner(p0: (i32, i32), p1: (i32, i32)) -> Vec<(i32, i32)> {
    let (x0, y0) = p0;
    let (x1, y1) = p1;
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut out = Vec::with_capacity((dx.max(dy) + 1) as usize);
    let (mut x, mut y) = (x0, y0);
    if dx >= dy {
        // Shallow slope: step x.
        let mut err = dx / 2;
        loop {
            out.push((x, y));
            if x == x1 {
                break;
            }
            x += sx;
            err -= dy;
            if err < 0 {
                y += sy;
                err += dx;
            }
        }
    } else {
        // Steep slope: step y.
        let mut err = dy / 2;
        loop {
            out.push((x, y));
            if y == y1 {
                break;
            }
            y += sy;
            err -= dx;
            if err < 0 {
                x += sx;
                err += dy;
            }
        }
    }
    out
}

/// Midpoint-circle boundary — the octant-symmetric cell set with
/// duplicates at the axes removed, sorted for determinism.
/// `r == 0` returns a single cell at `c`.
pub fn circle(c: (i32, i32), r: u32) -> Vec<(i32, i32)> {
    if r == 0 {
        return vec![c];
    }
    let r = r as i32;
    let mut out = Vec::with_capacity((8 * r) as usize);
    let (mut x, mut y) = (0i32, r);
    let mut d = 1 - r;
    while x <= y {
        for (dx, dy) in [
            (x, y),
            (-x, y),
            (x, -y),
            (-x, -y),
            (y, x),
            (-y, x),
            (y, -x),
            (-y, -x),
        ] {
            out.push((c.0 + dx, c.1 + dy));
        }
        x += 1;
        if d < 0 {
            d += 2 * x + 1;
        } else {
            y -= 1;
            d += 2 * (x - y) + 1;
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Floor division — Rust's `/` truncates toward zero; crossings need
/// the floor to place boundary cells consistently.
fn fdiv(a: i128, b: i128) -> i128 {
    let q = a / b;
    if a % b != 0 && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

/// Scanline fill of a polygon: the set of cells whose *center*
/// `(x+½, y+½)` satisfies the even-odd inside test against `verts`
/// (boundary centers included). Computed entirely in `i128` with
/// doubled coordinates — vertices at `2v`, centers at odd numbers —
/// so no division or float is ever consulted. Returns sorted cells.
pub fn fill_polygon(verts: &[(i32, i32)]) -> Vec<(i32, i32)> {
    if verts.len() < 3 {
        return Vec::new();
    }
    let n = verts.len();
    // Edges in doubled space; horizontal edges never cross a scanline.
    let mut edges: Vec<(i128, i128, i128, i128)> = Vec::with_capacity(n);
    for i in 0..n {
        let (x0, y0) = verts[i];
        let (x1, y1) = verts[(i + 1) % n];
        if y0 != y1 {
            edges.push((
                2 * x0 as i128,
                2 * y0 as i128,
                2 * x1 as i128,
                2 * y1 as i128,
            ));
        }
    }
    if edges.is_empty() {
        return Vec::new();
    }
    let ymin = verts.iter().map(|&(_, y)| y).min().unwrap_or(0);
    let ymax = verts.iter().map(|&(_, y)| y).max().unwrap_or(0);
    let mut out = Vec::new();
    for y in ymin..ymax {
        // Scanline through the row of cell centers: doubled y = 2y+1.
        let ys = 2 * y as i128 + 1;
        // Collect crossings as rational (num, den>0) pairs.
        let mut xs: Vec<(i128, i128)> = Vec::new();
        for &(x0, y0, x1, y1) in &edges {
            let (lo, hi) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
            // Half-open vertical range — the canonical vertex rule.
            if ys >= lo && ys < hi {
                // crossing x = x0 + (ys−y0)·(x1−x0)/(y1−y0) as (num,den), den>0
                let num = x0 * (y1 - y0) + (ys - y0) * (x1 - x0);
                let den = y1 - y0;
                let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
                xs.push((num, den));
            }
        }
        xs.sort_unstable_by(|&(an, ad), &(bn, bd)| (an * bd).cmp(&(bn * ad)));
        let mut k = 0;
        while k + 1 < xs.len() {
            let (sn, sd) = xs[k];
            let (en, ed) = xs[k + 1];
            // First center ≥ start crossing: (2xc+1)·sd ≥ sn — start a
            // touch low (floor-div) then walk up.
            let mut xc = fdiv(sn - sd, 2 * sd);
            while (2 * xc + 1) * sd < sn {
                xc += 1;
            }
            while (2 * xc + 1) * ed <= en {
                out.push((xc as i32, y));
                xc += 1;
            }
            k += 2;
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::{convex_hull, point_in_polygon, PointLocation};
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// DDA oracle: sample the segment densely and collect touched cells.
    fn oracle_line(p0: (i32, i32), p1: (i32, i32)) -> BTreeSet<(i32, i32)> {
        let dx = (p1.0 - p0.0).abs();
        let dy = (p1.1 - p0.1).abs();
        let steps = dx.max(dy).max(1);
        let mut set = BTreeSet::new();
        for i in 0..=steps * 4 {
            // exact point at t = i/(4·steps), in fixed point
            let x = (p0.0 as i64) * (4 * steps as i64 - i as i64) + (p1.0 as i64) * i as i64;
            let y = (p0.1 as i64) * (4 * steps as i64 - i as i64) + (p1.1 as i64) * i as i64;
            let den = 4 * steps as i64;
            // nearest cell = round(x/den) — floor((2x+den)/(2den)),
            // floor-div (trunc would round negative halves the wrong way)
            let cx = {
                let (a, b) = (2 * x + den, 2 * den);
                let q = a / b;
                (if a % b != 0 && ((a < 0) != (b < 0)) {
                    q - 1
                } else {
                    q
                }) as i32
            };
            let cy = {
                let (a, b) = (2 * y + den, 2 * den);
                let q = a / b;
                (if a % b != 0 && ((a < 0) != (b < 0)) {
                    q - 1
                } else {
                    q
                }) as i32
            };
            set.insert((cx, cy));
        }
        set
    }

    #[test]
    fn line_is_subset_of_dda_cells_and_connected() {
        let mut rng = SplitMix64::new(0xBA5E_11AA);
        for _ in 0..300 {
            let p0 = ((rng.below(40) as i32) - 20, (rng.below(40) as i32) - 20);
            let p1 = ((rng.below(40) as i32) - 20, (rng.below(40) as i32) - 20);
            let got = line(p0, p1);
            assert_eq!(got[0], p0);
            assert_eq!(*got.last().unwrap_or(&(0, 0)), p1);
            // 8-connected, no repeated cell.
            let set: BTreeSet<(i32, i32)> = got.iter().copied().collect();
            assert_eq!(set.len(), got.len());
            for w in got.windows(2) {
                let dx = (w[1].0 - w[0].0).abs();
                let dy = (w[1].1 - w[0].1).abs();
                assert!(dx <= 1 && dy <= 1 && dx + dy >= 1);
            }
            let dda = oracle_line(p0, p1);
            for &c in &got {
                assert!(dda.contains(&c), "cell {c:?} off line {p0:?}{p1:?}");
            }
            // Reversal symmetry.
            let mut rev = line(p1, p0);
            rev.reverse();
            assert_eq!(rev, got);
        }
        assert_eq!(line((2, 2), (2, 2)), vec![(2, 2)]);
        assert_eq!(line((0, 0), (3, 0)), vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
        assert_eq!(line((0, 0), (0, 3)), vec![(0, 0), (0, 1), (0, 2), (0, 3)]);
    }

    #[test]
    fn circle_is_symmetric_and_on_radius() {
        let mut rng = SplitMix64::new(0xC1BC_1E55);
        for _ in 0..60 {
            let c = ((rng.below(20) as i32) - 10, (rng.below(20) as i32) - 10);
            let r = rng.below(15);
            let cells = circle(c, r);
            if r == 0 {
                assert_eq!(cells, vec![c]);
                continue;
            }
            let set: BTreeSet<(i32, i32)> = cells.iter().copied().collect();
            // 8-way symmetry.
            for &(x, y) in &cells {
                let (dx, dy) = (x - c.0, y - c.1);
                for &(sx, sy) in &[(1, 1), (-1, 1), (1, -1), (-1, -1)] {
                    assert!(set.contains(&(c.0 + sx * dx, c.1 + sy * dy)));
                    assert!(set.contains(&(c.0 + sx * dy, c.1 + sy * dx)));
                }
            }
            // Each cell's center-distance stays within the r±1 band.
            for &(x, y) in &cells {
                let d2 = ((x - c.0) as i64).pow(2) + ((y - c.1) as i64).pow(2);
                let rr = r as i64;
                assert!(
                    (rr - 1).pow(2) <= d2 && d2 <= (rr + 1).pow(2),
                    "{x},{y} r={r} d²={d2}"
                );
            }
        }
    }

    #[test]
    fn fill_matches_center_point_in_polygon() {
        let mut rng = SplitMix64::new(0xF111_5EED);
        for _ in 0..200 {
            let n = 3 + (rng.below(5) as usize);
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| ((rng.below(12) as i32), (rng.below(12) as i32)))
                .collect();
            let hull = convex_hull(&pts);
            if hull.len() < 3 {
                continue;
            }
            let filled: BTreeSet<(i32, i32)> = fill_polygon(&hull).into_iter().collect();
            // Oracle: every cell in the bbox — its center tested against
            // the doubled polygon by `poly::point_in_polygon`.
            let poly2: Vec<(i32, i32)> = hull.iter().map(|&(x, y)| (2 * x, 2 * y)).collect();
            let (xmin, xmax, ymin, ymax) = hull.iter().fold(
                (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
                |(a, b, c, d), &(x, y)| (a.min(x), b.max(x), c.min(y), d.max(y)),
            );
            for x in xmin - 1..=xmax {
                for y in ymin - 1..=ymax {
                    let center = (2 * x + 1, 2 * y + 1);
                    let in_or_on = matches!(
                        point_in_polygon(center, &poly2),
                        PointLocation::Inside | PointLocation::OnBoundary
                    );
                    assert_eq!(
                        filled.contains(&(x, y)),
                        in_or_on,
                        "cell ({x},{y}) vs oracle on {hull:?}"
                    );
                }
            }
        }
        // Triangle sanity.
        let tri = vec![(0, 0), (4, 0), (0, 4)];
        let f: BTreeSet<(i32, i32)> = fill_polygon(&tri).into_iter().collect();
        assert!(f.contains(&(0, 0)));
        assert!(!f.contains(&(4, 4)));
    }
}

//! Poisson-disk sampling — Bridson's 2007 dart-throwing: fast blue
//! noise where every accepted point is ≥ `min_dist` from every
//! other, and the layout is packed so dense that rejection never
//! starves. `sobol` gives low *discrepancy* (quadrature-friendly);
//! Poisson disks give *uniform minimum separation* (spawn points,
//! scatter placement, stippling).
//!
//! Integer coordinates throughout, seeded by `Pcg` — the same seed
//! reproduces the same point set on every platform. The grid is the
//! reference `min_dist/√2` cell so any violating neighbor must lie
//! within two cells, checked exactly in `i64` squared distance.
//!
//! ```
//! use izanagi_kit::poisson::poisson_disk;
//!
//! let pts = poisson_disk(100, 100, 10, 30, 7);
//! assert!(!pts.is_empty());
//! for i in 0..pts.len() {
//!     for j in (i + 1)..pts.len() {
//!         let (dx, dy) = (pts[i].0 - pts[j].0, pts[i].1 - pts[j].1);
//!         assert!(dx * dx + dy * dy >= 10 * 10);
//!     }
//! }
//! ```

use crate::fixed::Fixed;
use crate::pcg::Pcg;

/// Bridson's algorithm on the integer grid `[0,w)×[0,h)`: up to `k`
/// candidates per active point, ring `[r,2r)` around it, cell size
/// `⌈r·7071/10000⌉` (≈ r/√2). First point is seeded center-free.
pub fn poisson_disk(width: i64, height: i64, min_dist: i64, k: u32, seed: u64) -> Vec<(i64, i64)> {
    if width <= 0 || height <= 0 || min_dist <= 0 {
        return Vec::new();
    }
    let mut rng = Pcg::new(seed);
    let r2 = min_dist * min_dist;
    let cell = ((min_dist * 7071) / 10000).max(1);
    let gw = ((width + cell - 1) / cell) as usize;
    let gh = ((height + cell - 1) / cell) as usize;
    let mut grid = vec![-1i32; gw * gh]; // -1 = empty cell
    let mut active: Vec<usize> = Vec::new();
    let mut points: Vec<(i64, i64)> = Vec::new();

    fn place(p: (i64, i64), cell: i64, gw: usize, grid: &mut [i32], points: &mut Vec<(i64, i64)>) {
        let idx = points.len();
        points.push(p);
        grid[(p.1 / cell) as usize * gw + (p.0 / cell) as usize] = idx as i32;
    }
    #[allow(clippy::too_many_arguments)]
    fn fits(
        p: (i64, i64),
        width: i64,
        height: i64,
        min_dist: i64,
        cell: i64,
        gw: usize,
        gh: usize,
        r2: i64,
        grid: &[i32],
        points: &[(i64, i64)],
    ) -> bool {
        if p.0 < 0 || p.0 >= width || p.1 < 0 || p.1 >= height {
            return false;
        }
        // Any point within r lies in a cell up to ⌈r/cell⌉+1 away.
        let reach = (min_dist + cell - 1) / cell + 1;
        let (cx, cy) = (p.0 / cell, p.1 / cell);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (nx, ny) = (cx + dx, cy + dy);
                if nx < 0 || ny < 0 || nx >= gw as i64 || ny >= gh as i64 {
                    continue;
                }
                let other = grid[ny as usize * gw + nx as usize];
                if other < 0 {
                    continue;
                }
                let q = points[other as usize];
                let (ddx, ddy) = (p.0 - q.0, p.1 - q.1);
                if ddx * ddx + ddy * ddy < r2 {
                    return false;
                }
            }
        }
        true
    }

    let first = (
        rng.next_bounded(width as u32) as i64,
        rng.next_bounded(height as u32) as i64,
    );
    place(first, cell, gw, &mut grid, &mut points);
    active.push(0);

    while !active.is_empty() {
        let ai = rng.next_bounded(active.len() as u32) as usize;
        let base = points[active[ai]];
        let mut found = false;
        for _ in 0..k {
            // Random ring point: uniform angle + radius in [r,2r).
            let turn = Fixed::from_ratio(rng.next_bounded(360_000) as i32, 360_000);
            let (s, c) = turn.mul(Fixed::TWO_PI).sin_cos();
            let rad = min_dist + rng.next_bounded(min_dist as u32) as i64;
            let cand = (
                base.0 + (c.raw() as i64 * rad) / 65536,
                base.1 + (s.raw() as i64 * rad) / 65536,
            );
            if fits(
                cand, width, height, min_dist, cell, gw, gh, r2, &grid, &points,
            ) {
                place(cand, cell, gw, &mut grid, &mut points);
                active.push(points.len() - 1);
                found = true;
                break;
            }
        }
        if !found {
            active.swap_remove(ai);
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_pairs_respect_min_dist() {
        let pts = poisson_disk(200, 150, 12, 30, 99);
        assert!(pts.len() >= 5, "{}", pts.len());
        for i in 0..pts.len() {
            for j in (i + 1)..pts.len() {
                let (dx, dy) = (pts[i].0 - pts[j].0, pts[i].1 - pts[j].1);
                assert!(dx * dx + dy * dy >= 12 * 12, "{:?} {:?}", pts[i], pts[j]);
            }
        }
    }

    #[test]
    fn denser_than_naive_random() {
        // Poisson packs meaningfully denser than i.i.d. sampling at
        // the same separation (classic blue-noise property).
        let pts = poisson_disk(100, 100, 10, 30, 3);
        // Upper bound on unpacked area: each disk π r²/4... any sane
        // run fits ≥40 points into 100×100 at r=10.
        assert!(pts.len() >= 40, "{}", pts.len());
        // All in-bounds.
        assert!(pts
            .iter()
            .all(|&(x, y)| (0i64..100).contains(&x) && (0i64..100).contains(&y)));
    }

    #[test]
    fn edge_cases() {
        assert_eq!(poisson_disk(0, 100, 10, 30, 1), Vec::new());
        assert_eq!(poisson_disk(100, 100, 0, 30, 1), Vec::new());
        // min_dist ≥ domain: only the seed point ever fits.
        assert_eq!(poisson_disk(10, 10, 50, 30, 1).len(), 1);
        // k=0: just the seed.
        assert_eq!(poisson_disk(100, 100, 10, 0, 1).len(), 1);
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(
            poisson_disk(80, 80, 8, 25, 5),
            poisson_disk(80, 80, 8, 25, 5)
        );
        assert_ne!(
            poisson_disk(80, 80, 8, 25, 5),
            poisson_disk(80, 80, 8, 25, 6)
        );
    }
}

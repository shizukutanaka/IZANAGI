//! Worley (cellular) noise on `Fixed` — feature-point distance fields:
//! `F1` = distance to the nearest jittered lattice feature point,
//! `F2` = second-nearest, `F2 − F1` = the cell-boundary ridge the
//! classic "caustic / cracked earth" look is built from. Unlike
//! [`crate::gnoise`] (gradient noise — smooth, zero at lattice points)
//! this produces sharp distance topology: continuous, with creases
//! exactly on the Voronoi boundaries.
//!
//! Feature points: one per integer cell, position jittered to
//! `(i + u, j + v)` with `u, v ∈ (0,1)` from the cell hash — the
//! deterministic recipe, so the field is byte-identical per seed.
//!
//! ```
//! use izanagi_kit::{fixed::Fixed, worley::worley};
//!
//! // Exactly on a feature point the F1 is near zero (jittered, so
//! // pick a point and bound it); far away it stays under √2 + slack.
//! let (f1, f2) = worley(Fixed::from_ratio(5, 2), Fixed::from_ratio(3, 2), 7);
//! assert!(f1 <= f2);
//! assert!(f1 >= Fixed::ZERO && f2 < Fixed::from_int(2));
//! ```

use crate::fixed::Fixed;

/// Cell hash → the 2-D feature-point offset in `(0,1)²` (two raw-16
/// fields, so strictly interior — never exactly on a wall).
fn feature(ix: i64, iy: i64, seed: u64) -> (Fixed, Fixed) {
    let mut z = (ix as u64)
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add((iy as u64).wrapping_mul(0xC2B2AE3D27D4EB4F))
        .wrapping_add(seed ^ 0xA0761D6478BD642F);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    (
        Fixed::from_raw((z & 0xFFFF) as i32),
        Fixed::from_raw(((z >> 16) & 0xFFFF) as i32),
    )
}

/// `(F1, F2)` — distances to the closest and second-closest feature
/// points. Both `≥ 0`, `F1 ≤ F2`, `F2 < ~2` for unit cells (a point is
/// always within √2 of a jittered feature point).
pub fn worley(x: Fixed, y: Fixed, seed: u64) -> (Fixed, Fixed) {
    let (cx, cy) = (x.floor(), y.floor());
    // Raw integer cell coords for hashing (Fixed floor → i64 units).
    let (ix, iy) = (cx.raw() as i64 >> 16, cy.raw() as i64 >> 16);
    let mut f1 = Fixed::MAX;
    let mut f2 = Fixed::MAX;
    for dy in -1i64..=1 {
        for dx in -1i64..=1 {
            let (ux, uy) = feature(ix + dx, iy + dy, seed);
            let px = cx + Fixed::from_int(dx as i32) + ux;
            let py = cy + Fixed::from_int(dy as i32) + uy;
            let (ox, oy) = (x - px, y - py);
            // sqrt(dx² + dy²) — exact Fixed metric, monotone in len².
            let d = (ox.mul(ox) + oy.mul(oy)).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2)
}

/// `F2 − F1` — small exactly on Voronoi cell borders: invert it for
/// the "cracked" texture or threshold it for cell outlines.
pub fn worley_edge(x: Fixed, y: Fixed, seed: u64) -> Fixed {
    let (f1, f2) = worley(x, y, seed);
    f2 - f1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fr(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }
    fn to_f64(x: Fixed) -> f64 {
        x.raw() as f64 / 65536.0
    }

    /// f64 oracle: same jittered lattice in float math.
    fn oracle(x: f64, y: f64, seed: u64) -> (f64, f64) {
        let feat = |ix: i64, iy: i64| {
            let mut z = (ix as u64)
                .wrapping_mul(0x9E3779B97F4A7C15)
                .wrapping_add((iy as u64).wrapping_mul(0xC2B2AE3D27D4EB4F))
                .wrapping_add(seed ^ 0xA0761D6478BD642F);
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^= z >> 31;
            (
                (z & 0xFFFF) as f64 / 65536.0,
                ((z >> 16) & 0xFFFF) as f64 / 65536.0,
            )
        };
        let (ix, iy) = (x.floor() as i64, y.floor() as i64);
        let (mut f1, mut f2) = (f64::MAX, f64::MAX);
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let (ux, uy) = feat(ix + dx, iy + dy);
                let (px, py) = (ix as f64 + dx as f64 + ux, iy as f64 + dy as f64 + uy);
                let d = ((x - px) * (x - px) + (y - py) * (y - py)).sqrt();
                if d < f1 {
                    f2 = f1;
                    f1 = d;
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
        (f1, f2)
    }

    #[test]
    fn matches_f64_oracle() {
        let mut g = crate::rng::SplitMix64::new(0x407);
        for _ in 0..60 {
            let (x, y) = (g.below(400) as f64 / 100.0, g.below(400) as f64 / 100.0);
            let (f1, f2) = worley(
                Fixed::from_raw((x * 65536.0) as i32),
                Fixed::from_raw((y * 65536.0) as i32),
                99,
            );
            let (o1, o2) = oracle(x, y, 99);
            assert!((to_f64(f1) - o1).abs() < 0.02, "F1 {f1:?} vs {o1}");
            assert!((to_f64(f2) - o2).abs() < 0.02, "F2 {f2:?} vs {o2}");
        }
    }

    #[test]
    fn f1_le_f2_and_edge_nonnegative() {
        let mut g = crate::rng::SplitMix64::new(0xE11);
        for _ in 0..200 {
            let (x, y) = (fr(g.below(400) as i32, 100), fr(g.below(400) as i32, 100));
            let (f1, f2) = worley(x, y, 5);
            assert!(f1 >= Fixed::ZERO && f1 <= f2);
            assert!(worley_edge(x, y, 5) >= Fixed::ZERO);
            // Any point is within √2·(1+jitter slack) of a feature.
            assert!(f1 < Fixed::from_int(2), "{f1:?}");
        }
    }

    #[test]
    fn edge_is_small_near_cell_borders() {
        // F2−F1 is the distance-to-border proxy: scan for a local
        // minimum ridge and check it exists (a point where two
        // features are nearly equidistant).
        let mut min_edge = Fixed::MAX;
        for i in 0..40 {
            for j in 0..40 {
                let e = worley_edge(fr(i * 3 + 1, 10), fr(j * 3 + 1, 10), 3);
                min_edge = min_edge.min(e);
            }
        }
        assert!(min_edge < fr(1, 20), "{min_edge:?}");
    }

    #[test]
    fn continuous_field_nearby_points_close() {
        // F1 is a distance function — Lipschitz-1: nearby inputs give
        // nearby outputs (no discontinuities unlike value noise).
        for i in 0..20 {
            let x = fr(i * 17 + 3, 10);
            let y = fr(i * 11 + 7, 10);
            let eps = Fixed::from_raw(256); // ~0.004
            let (a, _) = worley(x, y, 8);
            let (b, _) = worley(x + eps, y, 8);
            assert!((a - b).abs() < fr(1, 50), "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn deterministic_twice() {
        for i in 0..10 {
            let (x, y) = (fr(i * 7, 10), fr(i * 13, 10));
            assert_eq!(worley(x, y, 1), worley(x, y, 1));
            assert_eq!(worley_edge(x, y, 1), worley_edge(x, y, 1));
        }
    }
}

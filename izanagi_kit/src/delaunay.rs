//! Delaunay triangulation of a 2-D point set — the "connect nearby rooms"
//! primitive: edges of a Delaunay triangulation link each point to its
//! spatially adjacent neighbours without ever crossing, so carving corridors
//! along them (TinyKeep-style) produces organic dungeon connectivity instead
//! of a tree.
//!
//! Algorithm: incremental insertion with a super-triangle — the
//! **Bowyer–Watson** family (A. Bowyer / D. F. Watson, *Comput. J.* 24(2),
//! 1981) in its **split + Lawson-flip** form: each point splits its
//! containing triangle, then Lawson edge flips restore the Delaunay property
//! locally. The split+flip variant is used instead of the cavity-fan variant
//! because every operation is local — coverage of the super-triangle is
//! preserved exactly, so degenerate/non-star cavities can never leave hull
//! gaps (see `hull_edge_cannot_flip_to_super_vertex`).
//!
//! How it stays deterministic and integer-exact:
//!
//! * Points are inserted in **input order** — output is a pure function of
//!   the input slice.
//! * The incircle test is the exact 3×3 lifted-coordinate determinant in
//!   `i128` (safe for input coordinates up to ~10⁸ in magnitude — far past
//!   playable map sizes). A point is "inside" a circumcircle only when
//!   `det > 0` on a **counter-clockwise** triangle; `det == 0` (cocircular)
//!   is treated as outside, which is what makes diagonal choices canonical.
//! * Flips are processed from a sorted `BTreeSet` of candidate edges — a
//!   total order, never hash iteration.
//! * The super-triangle sits at distance ~`span²` (not ~`span`) so
//!   circumcircles through (hull edge, super vertex) cannot capture interior
//!   points — see the margin comment in `delaunay`.
//!
//! Degenerate inputs (fewer than 3 unique points, or all points collinear)
//! return an empty triangulation — there is no triangulation to report.
//!
//! ```
//! use izanagi_kit::delaunay::{delaunay, delaunay_edges};
//!
//! let pts = [(0, 0), (4, 0), (0, 3), (4, 3)];
//! let tris = delaunay(&pts);
//! assert_eq!(tris.len(), 2); // a quad splits into 2 triangles
//! let edges = delaunay_edges(&pts);
//! assert_eq!(edges.len(), 5); // 4 sides + 1 diagonal
//! ```

use std::collections::BTreeMap;

/// Vertex indices into the caller's `points` slice (after dedup — see below).
/// Returned in counter-clockwise order.
type Tri = [u32; 3];

/// Twice the signed area of triangle `a,b,c`; positive iff CCW.
#[inline]
fn cross2(a: (i64, i64), b: (i64, i64), c: (i64, i64)) -> i64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Incircle test: is `d` strictly inside the circumcircle of CCW triangle
/// `a,b,c`? Exact `i128` determinant of the lifted coordinates:
/// `det = ax·(by·cq − bq·cy) − ay·(bx·cq − bq·cx) + aq·(bx·cy − by·cx)`,
/// where each coordinate is translated by `−d` and `q = x² + y²`.
/// `det > 0` ⇔ inside, given `a,b,c` is CCW.
fn in_circle(a: (i64, i64), b: (i64, i64), c: (i64, i64), d: (i64, i64)) -> bool {
    let ax = (a.0 - d.0) as i128;
    let ay = (a.1 - d.1) as i128;
    let bx = (b.0 - d.0) as i128;
    let by = (b.1 - d.1) as i128;
    let cx = (c.0 - d.0) as i128;
    let cy = (c.1 - d.1) as i128;
    let aq = ax * ax + ay * ay;
    let bq = bx * bx + by * by;
    let cq = cx * cx + cy * cy;
    ax * (by * cq - bq * cy) - ay * (bx * cq - bq * cx) + aq * (bx * cy - by * cx) > 0
}

/// Delaunay triangulate `points`. Returns vertex indices into `points` —
/// each entry `[a, b, c]` is one triangle in counter-clockwise order.
///
/// Duplicate points collapse to their first occurrence. Triangles are
/// emitted in a canonical order (sorted by vertex indices) so the return
/// value is byte-identical across runs and platforms.
pub fn delaunay(points: &[(i32, i32)]) -> Vec<Tri> {
    // Dedup to unique points (first occurrence wins), preserving input order.
    let mut unique: Vec<(i64, i64)> = Vec::with_capacity(points.len());
    let mut index_of: BTreeMap<(i32, i32), u32> = BTreeMap::new();
    for &p in points {
        let next = unique.len() as u32;
        index_of.entry(p).or_insert_with(|| {
            unique.push((p.0 as i64, p.1 as i64));
            next
        });
    }
    if unique.len() < 3 {
        return Vec::new();
    }

    // Super-triangle containing every point: a triangle covering the bounding
    // box with margin. Vertices appended to `unique` as indices n..n+2.
    let (mut min_x, mut min_y, mut max_x, mut max_y) =
        (unique[0].0, unique[0].1, unique[0].0, unique[0].1);
    for &p in &unique[1..] {
        min_x = min_x.min(p.0);
        min_y = min_y.min(p.1);
        max_x = max_x.max(p.0);
        max_y = max_y.max(p.1);
    }
    let span = (max_x - min_x).max(max_y - min_y).max(1);
    let mid_x = (min_x + max_x) / 2;
    let n = unique.len() as u32;
    // Super-triangle vertices sit at distance ~span² from the data, not ~span.
    // Correctness requires that no circumcircle through (hull edge, super
    // vertex) capture an interior point: with super distance D the circle's
    // far-side cap (sagitta) is ≈ edge²/(8D) — below one unit whenever
    // D ≥ span² — so integer interior points can never be inside it and hull
    // edges can never flip away. A tight super-triangle (D ~ span) lets those
    // circles reach inside the hull and produces holes after super-touching
    // triangles are dropped. `saturating_mul` + a 10^8 cap keeps the i128
    // incircle determinant far from overflow (det ≈ D⁴ ≤ 10³² ≪ 2¹²⁷).
    let margin = span.saturating_mul(span).clamp(4 * span, 100_000_000);
    unique.push((mid_x, min_y - margin)); // apex above
    unique.push((mid_x + margin, max_y + margin)); // bottom-right
    unique.push((mid_x - margin, max_y + margin)); // bottom-left
                                                   // Order apex → bottom-right → bottom-left is counter-clockwise (cross2 > 0),
                                                   // which the incircle sign convention requires for every triangle.
    let super_lo = n; // smallest super-vertex index

    // Active triangle list; every entry is CCW by construction.
    let mut tris: Vec<[u32; 3]> = vec![[n, n + 1, n + 2]];
    let verts = unique.as_slice();

    for pi in 0..n {
        let p = verts[pi as usize];
        // Insert via split + Lawson flips (Guibas–Stolfi incremental form):
        // locate the containing triangle, split it around p, then restore the
        // Delaunay property by flipping any interior edge whose neighbouring
        // circumcircle contains p. All operations are local — coverage of the
        // super-triangle is preserved exactly, so hull gaps can't form even
        // for cocircular/degenerate inputs (a non-star "cavity" never arises).
        let Some(ti) = tris.iter().position(|t| {
            let (a, b, c) = (
                verts[t[0] as usize],
                verts[t[1] as usize],
                verts[t[2] as usize],
            );
            cross2(a, b, p) >= 0 && cross2(b, c, p) >= 0 && cross2(c, a, p) >= 0
        }) else {
            continue; // coverage is always complete; unreachable defensively
        };
        // The pocket = containing triangle, plus the neighbour of any edge p
        // lies exactly on (an on-edge insertion splits both sides).
        let t0 = tris.remove(ti);
        let mut pocket = vec![t0];
        for ei in 0..3 {
            let (u, v) = (t0[ei], t0[(ei + 1) % 3]);
            if cross2(verts[u as usize], verts[v as usize], p) == 0 {
                // p on edge (u,v): find the other triangle sharing it.
                if let Some(ni) = tris.iter().position(|t| t.contains(&u) && t.contains(&v)) {
                    pocket.push(tris.remove(ni));
                }
            }
        }
        // Worklist = pocket boundary edges (edges whose p-fan triangle is
        // non-degenerate and that aren't the shared on-edge cut).
        let mut worklist: std::collections::BTreeSet<(u32, u32)> =
            std::collections::BTreeSet::new();
        for &t in &pocket {
            for ei in 0..3 {
                let (u, v) = (t[ei], t[(ei + 1) % 3]);
                if cross2(verts[u as usize], verts[v as usize], p) != 0 {
                    worklist.insert((u.min(v), u.max(v)));
                }
            }
        }
        // Split: fan each removed triangle's edges to p, skipping degenerates.
        for &t in &pocket {
            for ei in 0..3 {
                let (u, v) = (t[ei], t[(ei + 1) % 3]);
                let s = cross2(verts[u as usize], verts[v as usize], p);
                if s > 0 {
                    tris.push([u, v, pi]);
                } else if s < 0 {
                    tris.push([v, u, pi]);
                }
            }
        }
        // Lawson flips: any pocket-boundary edge whose far triangle's
        // circumcircle contains p gets flipped — deterministic in sorted
        // edge order. Cocircular neighbours (det == 0) never flip, which is
        // what makes the diagonal choice canonical.
        let mut flips = 0u32;
        while let Some(&(a, b)) = worklist.iter().next() {
            worklist.remove(&(a, b));
            // Tri sharing (a,b) containing pi, and the other across it.
            let mut tp: Option<usize> = None;
            let mut to: Option<usize> = None;
            for (i, t) in tris.iter().enumerate() {
                let has =
                    (t[0] == a || t[1] == a || t[2] == a) && (t[0] == b || t[1] == b || t[2] == b);
                if !has {
                    continue;
                }
                if t[0] == pi || t[1] == pi || t[2] == pi {
                    tp = Some(i);
                } else {
                    to = Some(i);
                }
            }
            let (Some(i_p), Some(i_o)) = (tp, to) else {
                continue;
            };
            let t_o = tris[i_o];
            let c = t_o
                .iter()
                .copied()
                .find(|&v| v != a && v != b)
                .unwrap_or(pi);
            if !in_circle(
                verts[t_o[0] as usize],
                verts[t_o[1] as usize],
                verts[t_o[2] as usize],
                p,
            ) {
                continue;
            }
            // Flip is legal only on a strictly convex quad {a,c,b,p}: all
            // four boundary turns must share one sign.
            let (va, vb, vc, vp) = (verts[a as usize], verts[b as usize], verts[c as usize], p);
            let turns = [
                cross2(va, vc, vb),
                cross2(vc, vb, vp),
                cross2(vb, vp, va),
                cross2(vp, va, vc),
            ];
            let convex = turns.iter().all(|&s| s > 0) || turns.iter().all(|&s| s < 0);
            if !convex {
                continue;
            }
            // Flip diagonal (a,b) → (c,p): new tris {a,c,p} and {b,c,p} in
            // CCW orientation. Remove the two sharers first (highest index
            // first to keep positions valid).
            tris.remove(i_p.max(i_o));
            tris.remove(i_p.min(i_o));
            for (u, v) in [(a, c), (b, c)] {
                let s = cross2(verts[u as usize], verts[v as usize], p);
                if s > 0 {
                    tris.push([u, v, pi]);
                } else if s < 0 {
                    tris.push([v, u, pi]);
                }
            }
            // The quad's other two outer edges may now violate Delaunay.
            worklist.insert((a.min(c), a.max(c)));
            worklist.insert((b.min(c), b.max(c)));
            flips += 1;
            if flips > 100_000 {
                break; // defensive cap; exact predicates always terminate
            }
        }
    }

    // Drop triangles that touch a super vertex; drop zero-area degenerates.
    let mut out: Vec<Tri> = tris
        .into_iter()
        .filter(|t| t[0] < super_lo && t[1] < super_lo && t[2] < super_lo)
        .filter(|t| {
            cross2(
                unique[t[0] as usize],
                unique[t[1] as usize],
                unique[t[2] as usize],
            ) > 0
        })
        .map(|mut t| {
            // Rotate so the smallest vertex is first — canonical form (still
            // CCW, rotations preserve orientation).
            if t[1] < t[0] && t[1] < t[2] {
                t.rotate_left(1);
            } else if t[2] < t[0] && t[2] < t[1] {
                t.rotate_left(2);
            }
            t
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The unique undirected edges of [`delaunay`], sorted as `(min, max)` vertex
/// pairs — ready for corridor carving or MST restriction
/// ([`crate::voronoi::mst_edges_over`]).
pub fn delaunay_edges(points: &[(i32, i32)]) -> Vec<(u32, u32)> {
    let mut set: std::collections::BTreeSet<(u32, u32)> = std::collections::BTreeSet::new();
    for t in delaunay(points) {
        for i in 0..3 {
            let (u, v) = (t[i], t[(i + 1) % 3]);
            set.insert((u.min(v), u.max(v)));
        }
    }
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle 1: every returned triangle's circumcircle is empty of all other
    /// input points (the defining Delaunay property).
    fn assert_empty_circumcircles(points: &[(i32, i32)], tris: &[Tri]) {
        // Rebuild the dedup map the way `delaunay` does.
        let mut unique: Vec<(i64, i64)> = Vec::new();
        let mut index_of: BTreeMap<(i32, i32), u32> = BTreeMap::new();
        for &p in points {
            let next = unique.len() as u32;
            index_of.entry(p).or_insert_with(|| {
                unique.push((p.0 as i64, p.1 as i64));
                next
            });
        }
        for t in tris {
            let (a, b, c) = (t[0], t[1], t[2]);
            assert!(cross2(unique[a as usize], unique[b as usize], unique[c as usize]) > 0);
            for (p, &idx) in index_of.iter() {
                if idx == a || idx == b || idx == c {
                    continue;
                }
                let p64 = (p.0 as i64, p.1 as i64);
                assert!(
                    !in_circle(
                        unique[a as usize],
                        unique[b as usize],
                        unique[c as usize],
                        p64
                    ),
                    "point {idx} inside circumcircle of {t:?}"
                );
            }
        }
    }

    /// Andrew monotone-chain convex hull; returns hull vertices CCW.
    fn convex_hull(points: &[(i32, i32)]) -> Vec<(i64, i64)> {
        let mut pts: Vec<(i64, i64)> = points.iter().map(|p| (p.0 as i64, p.1 as i64)).collect();
        pts.sort_unstable();
        pts.dedup();
        if pts.len() <= 2 {
            return pts;
        }
        // Standard two-half construction: lower hull then upper hull.
        let mut lower: Vec<(i64, i64)> = Vec::new();
        for &p in &pts {
            while lower.len() >= 2 && cross2(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0
            {
                lower.pop();
            }
            lower.push(p);
        }
        let mut upper: Vec<(i64, i64)> = Vec::new();
        for &p in pts.iter().rev() {
            while upper.len() >= 2 && cross2(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0
            {
                upper.pop();
            }
            upper.push(p);
        }
        lower.pop();
        upper.pop();
        lower.extend(upper);
        lower
    }

    /// Oracle 2: twice the union area of the triangulation equals twice the
    /// convex-hull area (shoelace) — catches missing or doubled triangles.
    fn assert_area_matches_hull(points: &[(i32, i32)], tris: &[Tri]) {
        let mut unique: Vec<(i64, i64)> = Vec::new();
        let mut index_of: BTreeMap<(i32, i32), u32> = BTreeMap::new();
        for &p in points {
            let next = unique.len() as u32;
            index_of.entry(p).or_insert_with(|| {
                unique.push((p.0 as i64, p.1 as i64));
                next
            });
        }
        let tri_area: i64 = tris
            .iter()
            .map(|t| {
                cross2(
                    unique[t[0] as usize],
                    unique[t[1] as usize],
                    unique[t[2] as usize],
                )
            })
            .sum();
        let hull = convex_hull(points);
        if hull.len() < 3 {
            assert!(tris.is_empty(), "degenerate input must yield no triangles");
            return;
        }
        let mut hull_area: i64 = 0;
        for i in 0..hull.len() {
            let (x1, y1) = hull[i];
            let (x2, y2) = hull[(i + 1) % hull.len()];
            hull_area += x1 * y2 - x2 * y1;
        }
        assert_eq!(tri_area, hull_area, "Σ|tri area| must equal hull area");
    }

    /// Oracle 3: Euler's formula V − E + F = 2 (F counts the outer face).
    fn assert_euler(points: &[(i32, i32)], tris: &[Tri]) {
        if tris.is_empty() {
            return;
        }
        let mut unique_count = 0usize;
        let mut seen: std::collections::BTreeSet<(i32, i32)> = std::collections::BTreeSet::new();
        for &p in points {
            if seen.insert(p) {
                unique_count += 1;
            }
        }
        let edges: std::collections::BTreeSet<(u32, u32)> = tris
            .iter()
            .flat_map(|t| {
                [
                    (t[0].min(t[1]), t[0].max(t[1])),
                    (t[1].min(t[2]), t[1].max(t[2])),
                    (t[2].min(t[0]), t[2].max(t[0])),
                ]
            })
            .collect();
        let v = unique_count as i64;
        let e = edges.len() as i64;
        let f = tris.len() as i64 + 1; // +1 outer face
        assert_eq!(v - e + f, 2, "Euler characteristic violated");
    }

    /// Regression: this cloud produced a hull gap under a tight super-
    /// triangle — a circumcircle through (hull edge, super vertex) captured
    /// interior point 13, flipped hull edge (8,2) into super-touching
    /// triangles, and dropping them left an interior hole of area 43.
    #[test]
    fn hull_edge_cannot_flip_to_super_vertex() {
        let pts = [
            (-32, 25),
            (-24, -34),
            (36, 19),
            (34, -5),
            (4, -42),
            (39, -38),
            (-38, 14),
            (19, 13),
            (40, -30),
            (6, -28),
            (-27, -2),
            (-2, 8),
            (-29, -13),
            (37, -4),
        ];
        let tris = delaunay(&pts);
        assert_empty_circumcircles(&pts, &tris);
        assert_area_matches_hull(&pts, &tris);
        assert_euler(&pts, &tris);
    }

    #[test]
    fn quad_splits_into_two_triangles() {
        let pts = [(0, 0), (4, 0), (0, 3), (4, 3)];
        let tris = delaunay(&pts);
        assert_eq!(tris.len(), 2);
        assert_eq!(delaunay_edges(&pts).len(), 5);
        assert_empty_circumcircles(&pts, &tris);
        assert_area_matches_hull(&pts, &tris);
        assert_euler(&pts, &tris);
    }

    #[test]
    fn single_triangle() {
        let pts = [(0, 0), (5, 0), (0, 5)];
        // Canonical form: smallest vertex first, CCW preserved.
        assert_eq!(delaunay(&pts), vec![[0, 1, 2]]);
        assert_empty_circumcircles(&pts, &delaunay(&pts));
    }

    #[test]
    fn degenerate_inputs_return_empty() {
        assert!(delaunay(&[]).is_empty());
        assert!(delaunay(&[(1, 2)]).is_empty());
        assert!(delaunay(&[(0, 0), (1, 1)]).is_empty());
        assert!(delaunay(&[(0, 0), (0, 0), (0, 0)]).is_empty());
        // All collinear.
        assert!(delaunay(&[(0, 0), (1, 0), (2, 0), (3, 0)]).is_empty());
    }

    #[test]
    fn duplicates_collapse() {
        let pts = [(0, 0), (4, 0), (0, 3), (4, 3), (0, 0), (4, 0)];
        assert_eq!(delaunay(&pts).len(), 2);
    }

    #[test]
    fn square_center_point() {
        // Center point: all 4 outer triangles, 4 spokes + 4 rim edges.
        let pts = [(0, 0), (4, 0), (0, 4), (4, 4), (2, 2)];
        let tris = delaunay(&pts);
        assert_eq!(tris.len(), 4);
        assert_eq!(delaunay_edges(&pts).len(), 8);
        assert_empty_circumcircles(&pts, &tris);
        assert_area_matches_hull(&pts, &tris);
        assert_euler(&pts, &tris);
    }

    #[test]
    fn random_point_clouds_satisfy_all_oracles() {
        let mut rng = SplitMix64::new(0xDE1A);
        for _ in 0..40 {
            let n = rng.range(4, 26) as usize;
            let mut pts = Vec::with_capacity(n);
            for _ in 0..n {
                pts.push((rng.range(-50, 51), rng.range(-50, 51)));
            }
            let tris = delaunay(&pts);
            assert_empty_circumcircles(&pts, &tris);
            assert_area_matches_hull(&pts, &tris);
            assert_euler(&pts, &tris);
        }
    }

    #[test]
    fn cocircular_points_pick_a_canonical_diagonal() {
        // Square corners are cocircular — det == 0 is "outside", so the
        // diagonal chosen is deterministic for this exact input order.
        let pts = [(0, 0), (1, 0), (1, 1), (0, 1)];
        let tris = delaunay(&pts);
        assert_eq!(tris.len(), 2);
        let edges = delaunay_edges(&pts);
        assert_eq!(edges.len(), 5);
        // Deterministic: same call, same bytes.
        assert_eq!(tris, delaunay(&pts));
    }

    #[test]
    fn output_is_sorted_and_ccw() {
        let pts = [(0, 0), (4, 0), (0, 3), (4, 3), (2, 1)];
        let tris = delaunay(&pts);
        let mut sorted = tris.clone();
        sorted.sort_unstable();
        assert_eq!(tris, sorted);
        let pts64: Vec<(i64, i64)> = pts.iter().map(|p| (p.0 as i64, p.1 as i64)).collect();
        for t in &tris {
            assert!(
                cross2(
                    pts64[t[0] as usize],
                    pts64[t[1] as usize],
                    pts64[t[2] as usize]
                ) > 0
            );
        }
    }
}

//! Exact Voronoi-style spatial queries on the integer grid: which seed is
//! closest to each cell, how far, and how the seed set connects into one
//! minimal graph.
//!
//! [`pathfinding::DijkstraMap`](crate::pathfinding::DijkstraMap) answers
//! "how far is the nearest source, and which way is downhill" for pathing
//! units; it does **not** say *which* source that is. Territory assignment —
//! which of the scattered points owns this cell — is the procgen primitive
//! underneath biome maps, region borders and spawn-catchment areas, and it is
//! what this module computes. Three pieces:
//!
//! - [`voronoi_partition`] — an *exact* nearest-seed assignment under a chosen
//!   [`Distance`] metric, `O(w·h·seeds)`. Exactness is deliberate: the jump
//!   flooding algorithm (Rong & Tan, I3D 2006 — the standard fast route, GPU
//!   constant-time) only *approximates* the diagram, and this crate's promise
//!   is that results are provable, not approximate. Brute force is the correct
//!   algorithm at game-map scale.
//! - [`voronoi_flood`] — the same partition but measured through passable
//!   terrain (multi-source BFS), so walls split territories instead of the
//!   distance metric ignoring them.
//! - [`mst_edges`] — Kruskal minimum spanning tree over a point set: the
//!   canonical way to turn scattered seeds into a connected graph (the
//!   TinyKeep-style dungeon pipeline runs scatter → partition → connect).
//!
//! All three are pure integer arithmetic in fixed iteration orders; ties are
//! broken by lowest seed index, so results are bit-identical across platforms
//! and inputs.
//!
//! ```
//! use izanagi_kit::voronoi::{voronoi_partition, mst_edges};
//! use izanagi_kit::geometry::Distance;
//!
//! let seeds = [(2, 2), (14, 4), (8, 12)];
//! let grid = voronoi_partition(&seeds, 16, 16, Distance::EuclideanSquared);
//! assert_eq!(grid.get(2, 2), Some(0));
//! assert_eq!(grid.get(15, 0), Some(1));
//! let edges = mst_edges(&seeds);
//! assert_eq!(edges.len(), seeds.len() - 1);
//! ```

use crate::geometry::Distance;
use crate::world_hash::{DetHash, Fnv1a};
use std::collections::VecDeque;

/// Sentinel stored in cells that own no seed — unreachable cells under
/// [`voronoi_flood`], or every cell when `seeds` is empty.
pub const NO_SEED: u32 = u32::MAX;

/// The per-cell result of a Voronoi partition: which seed owns each cell and
/// how far away it is.
///
/// Determinism: `cells` is a row-major `Vec<u32>` — a canonical memory order
/// with no iteration-order dependence — and `dist` the same. [`DetHash`] folds
/// both in that order, so two `VoronoiGrid`s are equal iff their hashes are.
#[derive(Clone, Debug)]
pub struct VoronoiGrid {
    width: i32,
    height: i32,
    seed_count: u32,
    /// Seed index per cell, row-major. [`NO_SEED`] where unassigned.
    cells: Vec<u32>,
    /// Distance to the owning seed, row-major. `i32::MAX` where unassigned.
    dist: Vec<i32>,
}

impl VoronoiGrid {
    /// Grid width in cells.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Grid height in cells.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Number of seeds the partition was built from.
    pub fn seed_count(&self) -> u32 {
        self.seed_count
    }

    /// The seed index owning cell `(x, y)`, or `None` for out-of-bounds and
    /// unassigned (unreachable / no seeds) cells.
    pub fn get(&self, x: i32, y: i32) -> Option<u32> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let v = self.cells[(y * self.width + x) as usize];
        if v == NO_SEED {
            None
        } else {
            Some(v)
        }
    }

    /// Distance from `(x, y)` to the seed that owns it (the metric/BFS step
    /// count the partition was built with), or `None` where [`get`](Self::get)
    /// is `None`.
    pub fn dist_at(&self, x: i32, y: i32) -> Option<i32> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let v = self.dist[(y * self.width + x) as usize];
        if v == i32::MAX {
            None
        } else {
            Some(v)
        }
    }

    /// Cells owned by each seed, in seed order. Unassigned cells are not
    /// counted, so the sum can be less than `width * height` (flood partition
    /// with unreachable terrain).
    pub fn region_sizes(&self) -> Vec<u32> {
        let mut sizes = vec![0u32; self.seed_count as usize];
        for &c in &self.cells {
            if c != NO_SEED {
                sizes[c as usize] += 1;
            }
        }
        sizes
    }

    /// Cells sharing an edge with a different owner's cell — the region
    /// borders. Useful for drawing region outlines or placing walls/borders
    /// between territories.
    pub fn iter_borders(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let (w, h) = (self.width, self.height);
        self.cells.iter().enumerate().filter_map(move |(i, &c)| {
            if c == NO_SEED {
                return None;
            }
            let x = i as i32 % w;
            let y = i as i32 / w;
            // Right and bottom neighbours only — a border edge is reported
            // once, from the cell that sees it.
            let right = x + 1 < w && self.cells[i + 1] != c;
            let down = y + 1 < h && self.cells[i + w as usize] != c;
            if right || down {
                Some((x, y))
            } else {
                None
            }
        })
    }

    /// `true` when every cell owns a seed (empty for a `0×0` grid).
    pub fn is_total(&self) -> bool {
        self.cells.iter().all(|&c| c != NO_SEED)
    }
}

impl DetHash for VoronoiGrid {
    fn det_hash(&self, h: &mut Fnv1a) {
        h.write_i32(self.width);
        h.write_i32(self.height);
        h.write_u32(self.seed_count);
        for &c in &self.cells {
            h.write_u32(c);
        }
        for &d in &self.dist {
            h.write_i32(d);
        }
    }
}

/// Exact Voronoi partition: every cell is assigned to the nearest seed under
/// `metric`, with ties broken by lowest seed index.
///
/// `O(width·height·seeds)` — the honest cost of exactness. At dungeon scale
/// (200×200 cells, dozens of seeds) that is a millisecond-class computation,
/// and the result needs no "approximately Voronoi" caveat (contrast: the GPU
/// jump-flooding approach of Rong & Tan, I3D 2006, which trades exactness for
/// constant-round speed this module does not need).
///
/// An empty `seeds` slice yields a grid where every cell is [`NO_SEED`].
/// Seed coordinates outside the grid are legal (a seed outside the boundary
/// still wins cells near the edge — the semantics "nearest seed, wherever it
/// is" stay total).
pub fn voronoi_partition(
    seeds: &[(i32, i32)],
    width: i32,
    height: i32,
    metric: Distance,
) -> VoronoiGrid {
    let (w, h) = (width.max(0), height.max(0));
    let n = (w as usize) * (h as usize);
    let mut cells = vec![NO_SEED; n];
    let mut dist = vec![i32::MAX; n];
    if seeds.is_empty() {
        return VoronoiGrid {
            width: w,
            height: h,
            seed_count: 0,
            cells,
            dist,
        };
    }
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let mut best_d = i32::MAX;
            let mut best_s = NO_SEED;
            for (i, &s) in seeds.iter().enumerate() {
                let d = metric.between((x, y), s);
                // Strict `<` keeps the earliest (lowest-indexed) seed on ties —
                // the canonical, deterministic tie-break.
                if d < best_d {
                    best_d = d;
                    best_s = i as u32;
                }
            }
            cells[idx] = best_s;
            dist[idx] = best_d;
        }
    }
    VoronoiGrid {
        width: w,
        height: h,
        seed_count: seeds.len() as u32,
        cells,
        dist,
    }
}

/// Flood-fill Voronoi: multi-source BFS where `is_blocked(x, y)` marks walls
/// (matching the [`pathfinding`](crate::pathfinding) convention — out-of-bounds
/// counts as blocked). Each passable cell is assigned to the seed whose
/// 4-directional step distance is smallest; blocked cells stay [`NO_SEED`].
///
/// Region growing is 4-directional (orthogonal spread): a territory may not
/// leak through a diagonal corner, which is the semantics "influence spreading
/// through a dungeon" wants — if two seeds' regions met only at a wall
/// corner, the wall is part of neither.
///
/// Determinism: cells pop from a FIFO in strictly increasing distance and
/// seeds are enqueued in index order, so the first claim on a cell is the
/// canonical one; a later equal-distance claim never displaces it. Seeds on
/// blocked or out-of-bounds cells participate vacuously (they claim nothing).
///
/// `dist_at` returns the BFS step distance, not a metric value.
pub fn voronoi_flood<B>(
    seeds: &[(i32, i32)],
    width: i32,
    height: i32,
    mut is_blocked: B,
) -> VoronoiGrid
where
    B: FnMut(i32, i32) -> bool,
{
    let (w, h) = (width.max(0), height.max(0));
    let n = (w as usize) * (h as usize);
    let mut cells = vec![NO_SEED; n];
    let mut dist = vec![i32::MAX; n];
    let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
    for (i, &s) in seeds.iter().enumerate() {
        if s.0 >= 0 && s.1 >= 0 && s.0 < w && s.1 < h && !is_blocked(s.0, s.1) {
            let idx = (s.1 * w + s.0) as usize;
            cells[idx] = i as u32;
            dist[idx] = 0;
            queue.push_back(s);
        }
    }
    const CARDINALS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    while let Some((x, y)) = queue.pop_front() {
        let cur_idx = (y * w + x) as usize;
        let owner = cells[cur_idx];
        let d = dist[cur_idx];
        for (dx, dy) in CARDINALS {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h || is_blocked(nx, ny) {
                continue;
            }
            let nidx = (ny * w + nx) as usize;
            // Claim only unclaimed cells: a claimed cell is by construction at
            // distance <= d + 1 already, so an equal-distance rival claim can
            // never improve the answer — the first claim is canonical.
            if cells[nidx] == NO_SEED {
                cells[nidx] = owner;
                dist[nidx] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }
    VoronoiGrid {
        width: w,
        height: h,
        seed_count: seeds.len() as u32,
        cells,
        dist,
    }
}

/// Kruskal's minimum spanning tree over the complete graph on `points` —
/// returns the `len - 1` edges `(i, j)` (point indices, `i < j`) connecting
/// every point at minimum total Euclidean-squared edge cost.
///
/// The canonical turn from scattered sites into a connected region graph —
/// the backbone of TinyKeep-style dungeon generation (Poisson-scatter the
/// room centers, then wire them with the MST) and the deterministic answer
/// whenever "connect all the points cheaply" is needed.
///
/// Determinism: edges are selected in `(distance, i, j)` order — a total,
/// canonical order — so equal-length candidate sets resolve identically on
/// every run. `O(p² log p)` in edges. Fewer than two points yield no edges.
pub fn mst_edges(points: &[(i32, i32)]) -> Vec<(u32, u32)> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    // All candidate edges, sorted by (cost, i, j) — total and canonical.
    let mut edges: Vec<(i64, u32, u32)> = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            let d = Distance::EuclideanSquared.between(points[i], points[j]) as i64;
            edges.push((d, i as u32, j as u32));
        }
    }
    edges.sort_unstable();

    // Union-find over point indices.
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(parent: &mut [u32], x: u32) -> u32 {
        let mut r = x;
        while parent[r as usize] != r {
            r = parent[r as usize];
        }
        // Path halving keeps the structure shallow deterministically.
        let mut c = x;
        while parent[c as usize] != r {
            let next = parent[c as usize];
            parent[c as usize] = r;
            c = next;
        }
        r
    }

    let mut out = Vec::with_capacity(n - 1);
    for (_, i, j) in edges {
        let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
        if ri != rj {
            parent[ri as usize] = rj;
            out.push((i, j));
            if out.len() == n - 1 {
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use crate::world_hash::hash_state;
    use std::collections::BTreeSet;

    #[test]
    fn partition_assigns_to_nearest_seed() {
        let seeds = [(2, 2), (14, 4), (8, 12)];
        let g = voronoi_partition(&seeds, 16, 16, Distance::EuclideanSquared);
        assert_eq!(g.get(2, 2), Some(0));
        assert_eq!(g.get(14, 4), Some(1));
        assert_eq!(g.get(8, 12), Some(2));
        assert_eq!(g.get(0, 0), Some(0));
        assert_eq!(g.get(15, 15), Some(2));
        assert!(g.is_total());
        assert_eq!(g.seed_count(), 3);
        assert_eq!(g.width(), 16);
        assert_eq!(g.height(), 16);
        // Out-of-bounds reads are None, not panics.
        assert_eq!(g.get(-1, 0), None);
        assert_eq!(g.get(16, 0), None);
        assert_eq!(g.dist_at(2, 2), Some(0));
    }

    #[test]
    fn partition_matches_bruteforce_oracle() {
        // Independent oracle: nearest seed computed per-cell by a straight
        // double loop with Manhattan distance — a different code path than the
        // implementation's parameterised metric.
        let mut rng = SplitMix64::new(42);
        let seeds: Vec<(i32, i32)> = (0..7)
            .map(|_| (rng.range(0, 32), rng.range(0, 24)))
            .collect();
        let g = voronoi_partition(&seeds, 32, 24, Distance::Manhattan);
        for y in 0..24 {
            for x in 0..32 {
                let mut best = (i32::MAX, NO_SEED);
                for (i, &s) in seeds.iter().enumerate() {
                    let d = (x - s.0).abs() + (y - s.1).abs();
                    if d < best.0 {
                        best = (d, i as u32);
                    }
                }
                assert_eq!(g.get(x, y), Some(best.1));
                assert_eq!(g.dist_at(x, y), Some(best.0));
            }
        }
    }

    #[test]
    fn tie_goes_to_lowest_seed_index() {
        // (0,0) is equidistant to seeds (0,2) and (0,-2).
        let g = voronoi_partition(&[(0, 2), (0, -2)], 4, 4, Distance::EuclideanSquared);
        assert_eq!(g.get(0, 0), Some(0));
        let g2 = voronoi_partition(&[(0, -2), (0, 2)], 4, 4, Distance::EuclideanSquared);
        assert_eq!(g2.get(0, 0), Some(0));
    }

    #[test]
    fn empty_and_degenerate_inputs() {
        let g = voronoi_partition(&[], 8, 8, Distance::Manhattan);
        assert!(!g.is_total());
        assert_eq!(g.get(0, 0), None);
        assert_eq!(g.dist_at(0, 0), None);
        assert_eq!(g.region_sizes(), Vec::<u32>::new());
        let g0 = voronoi_partition(&[(0, 0)], 0, 0, Distance::Manhattan);
        assert!(g0.is_total()); // empty range: vacuously all assigned
        let neg = voronoi_partition(&[(0, 0)], -4, 8, Distance::Manhattan);
        assert_eq!(neg.width(), 0);
    }

    #[test]
    fn region_sizes_sum_to_cells() {
        let mut rng = SplitMix64::new(7);
        let seeds: Vec<(i32, i32)> = (0..9)
            .map(|_| (rng.range(0, 40), rng.range(0, 40)))
            .collect();
        let g = voronoi_partition(&seeds, 40, 40, Distance::Chebyshev);
        let sizes = g.region_sizes();
        assert_eq!(sizes.len(), 9);
        assert_eq!(sizes.iter().sum::<u32>(), 1600);
        for &s in &sizes {
            assert!(s > 0); // every seed owns its own cell at least
        }
    }

    #[test]
    fn flood_respects_walls() {
        // Two seeds split by a wall of blocked cells with one gap.
        let seeds = [(1, 1), (14, 4)];
        let wall_col = 8;
        let g = voronoi_flood(&seeds, 16, 9, |x, y| x == wall_col && y != 4);
        // Left territory is all seed 0, right is all seed 1 — the only route
        // between regions runs through the gap at (8, 4).
        assert_eq!(g.get(0, 0), Some(0));
        assert_eq!(g.get(15, 8), Some(1));
        // Wall cells are unclaimed.
        assert_eq!(g.get(8, 0), None);
        assert_eq!(g.dist_at(8, 0), None);
        // The gap cell is 10 steps from seed 0 but only 6 from seed 1 — the
        // flood claims it for seed 1.
        assert_eq!(g.get(8, 4), Some(1));
        assert!(!g.is_total()); // wall column unowned
    }

    #[test]
    fn flood_unreachable_pocket_stays_unowned() {
        // A cell fully walled off claims nothing.
        let seeds = [(0, 0)];
        let g = voronoi_flood(&seeds, 5, 5, |x, y| {
            x == 2 || y == 2 // sealed cross walls — (4,4) unreachable
        });
        assert_eq!(g.get(0, 0), Some(0));
        assert_eq!(g.get(4, 4), None);
        assert_eq!(g.get(2, 0), None); // wall itself
        let sizes = g.region_sizes();
        assert!(sizes[0] < 25);
    }

    #[test]
    fn flood_distances_are_step_counts() {
        let g = voronoi_flood(&[(0, 0)], 10, 10, |_, _| false);
        assert_eq!(g.dist_at(9, 9), Some(18)); // Manhattan walk
        assert_eq!(g.dist_at(9, 0), Some(9));
    }

    #[test]
    fn flood_empty_and_blocked_seeds() {
        let g = voronoi_flood(&[], 4, 4, |_, _| false);
        assert!(!g.is_total());
        // Seed standing inside a wall claims nothing.
        let g2 = voronoi_flood(&[(1, 1)], 4, 4, |x, y| (x, y) == (1, 1));
        assert!(!g2.is_total());
        // Out-of-bounds seeds participate vacuously.
        let g3 = voronoi_flood(&[(-5, 99)], 4, 4, |_, _| false);
        assert!(!g3.is_total());
    }

    #[test]
    fn mst_connects_all_points_minimally() {
        // Square corners: MST must pick 3 of the 4 sides (length 10 each),
        // never the diagonals (length ~14.14 -> sq 200).
        let pts = [(0, 0), (10, 0), (0, 10), (10, 10)];
        let edges = mst_edges(&pts);
        assert_eq!(edges.len(), 3);
        for &(i, j) in &edges {
            let d2 = Distance::EuclideanSquared.between(pts[i as usize], pts[j as usize]);
            assert_eq!(d2, 100, "MST picked a diagonal");
        }
        // All four points covered.
        let mut covered = BTreeSet::new();
        for &(i, j) in &edges {
            covered.insert(i);
            covered.insert(j);
        }
        assert_eq!(covered.len(), 4);
    }

    #[test]
    fn mst_is_deterministic_and_total() {
        let mut rng = SplitMix64::new(99);
        let pts: Vec<(i32, i32)> = (0..30)
            .map(|_| (rng.range(0, 100), rng.range(0, 100)))
            .collect();
        let a = mst_edges(&pts);
        let b = mst_edges(&pts);
        assert_eq!(a, b);
        assert_eq!(a.len(), 29);
        // Oracle: total weight equals an independent Prim's computation.
        let mut in_tree = vec![false; pts.len()];
        in_tree[0] = true;
        let mut total = 0i64;
        for _ in 1..pts.len() {
            let mut best = (i64::MAX, 0usize, 0usize);
            for (i, &p) in pts.iter().enumerate().filter(|(i, _)| in_tree[*i]) {
                for (j, &q) in pts.iter().enumerate().filter(|(j, _)| !in_tree[*j]) {
                    let d = Distance::EuclideanSquared.between(p, q) as i64;
                    if d < best.0 {
                        best = (d, i, j);
                    }
                }
            }
            total += best.0;
            in_tree[best.2] = true;
        }
        let mst_total: i64 = a
            .iter()
            .map(|&(i, j)| {
                Distance::EuclideanSquared.between(pts[i as usize], pts[j as usize]) as i64
            })
            .sum();
        assert_eq!(mst_total, total);
    }

    #[test]
    fn mst_edges_trivial_inputs() {
        assert!(mst_edges(&[]).is_empty());
        assert!(mst_edges(&[(5, 5)]).is_empty());
        assert_eq!(mst_edges(&[(0, 0), (3, 4)]), vec![(0, 1)]);
        // Duplicate points are legal: zero-length edges still connect them.
        let e = mst_edges(&[(1, 1), (1, 1), (5, 5)]);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn borders_mark_only_boundaries() {
        // Two seeds side by side: the border column must separate them.
        let g = voronoi_partition(&[(0, 0), (9, 0)], 10, 4, Distance::Manhattan);
        let borders: BTreeSet<(i32, i32)> = g.iter_borders().collect();
        // Every border cell sits adjacent to the other region.
        for &(x, _y) in &borders {
            assert!((4..=5).contains(&x), "border outside the seam: {x}");
        }
        assert!(!borders.is_empty());
    }

    #[test]
    fn voronoi_grid_dethash_is_canonical() {
        let seeds = [(1, 1), (6, 3)];
        let a = voronoi_partition(&seeds, 8, 8, Distance::Manhattan);
        let b = voronoi_partition(&seeds, 8, 8, Distance::Manhattan);
        assert_eq!(hash_state(&a), hash_state(&b));
        let c = voronoi_partition(&seeds, 8, 8, Distance::Chebyshev);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "different partition must hash differently"
        );
    }
}

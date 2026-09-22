//! Axial-coordinate hex-grid math — distance, lines, rings, and offset
//! conversion for hexagonal maps (hex-Civ boards, six-neighbor dungeons,
//! hex WFC grids). Pure integer arithmetic throughout.
//!
//! Reference implementation: Amit Patel's *Hexagonal Grids* guide
//! (redblobgames.com/grids/hexagons) — the canonical formulation. Coordinates
//! are **axial** (`q`, `r`) with the third cube coordinate derived as
//! `s = -q - r`, so every hex satisfies `q + r + s = 0`.
//!
//! Two conversions need care to stay integer-exact:
//!
//! * [`line()`] draws hex lines by cube-coordinate linear interpolation.
//!   Rather than fractionally interpolating and rounding (the float recipe),
//!   it lerps on *numerators* — each coordinate is `num / N` with `N` the
//!   line length — and rounds with [`cube_round`], so the result is exact for
//!   every denominator.
//! * [`cube_round`] picks the cube cell nearest to a fractional coordinate
//!   the way the float version does (round all three, then fix up the
//!   coordinate whose rounding moved it furthest), except all comparisons
//!   happen on `|rounded * N - num|`, never on floats.
//!
//! Everything here is deterministic: no iteration over hash containers, no
//! environment reads, no floats.
//!
//! ```
//! use izanagi_kit::hexgrid::{line, distance, Hex};
//!
//! let a = Hex::new(0, 0);
//! let b = Hex::new(3, -1);
//! assert_eq!(distance(a, b), 3);
//! let cells = line(a, b);
//! assert_eq!(cells.len(), 4); // endpoints + 2 interior hexes
//! ```
//!
//! [`cube_round`]: fn@cube_round

use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};

/// A hex cell in axial coordinates. `q` is the column-ish axis, `r` the
/// row-ish axis, and the third cube coordinate is `s = -q - r` (kept implicit
/// so `q + r + s = 0` can never drift).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hex {
    /// Axial `q` coordinate.
    pub q: i32,
    /// Axial `r` coordinate.
    pub r: i32,
}

impl Hex {
    /// The origin hex `(0, 0)`.
    pub const ORIGIN: Hex = Hex { q: 0, r: 0 };

    /// Construct a hex at axial `(q, r)`.
    #[inline]
    pub const fn new(q: i32, r: i32) -> Hex {
        Hex { q, r }
    }

    /// The third cube coordinate, `s = -q - r` (derived — always consistent).
    #[inline]
    pub fn s(self) -> i32 {
        -self.q - self.r
    }

    /// `self * k` — scale both coordinates.
    #[inline]
    pub fn scale(self, k: i32) -> Hex {
        Hex::new(self.q * k, self.r * k)
    }

    /// Hex distance from the origin — `( |q| + |r| + |s| ) / 2`.
    #[inline]
    pub fn distance_from_origin(self) -> i32 {
        distance(Hex::ORIGIN, self)
    }

    /// The neighbor in direction `dir` (`0..6`, wraps mod 6) — see
    /// [`DIRECTIONS`].
    #[inline]
    pub fn neighbor(self, dir: u32) -> Hex {
        self + (DIRECTIONS[(dir % 6) as usize])
    }

    /// All six neighbors, in [`DIRECTIONS`] order.
    pub fn neighbors(self) -> [Hex; 6] {
        DIRECTIONS.map(|d| self + (d))
    }

    /// Rotate 60° counter-clockwise around the origin: `(q, r, s) → (-s, -q, -r)`.
    #[inline]
    pub fn rotate_left(self) -> Hex {
        Hex::new(-self.s(), -self.q)
    }

    /// Rotate 60° clockwise around the origin: `(q, r, s) → (-r, -s, -q)`.
    #[inline]
    pub fn rotate_right(self) -> Hex {
        Hex::new(-self.r, -self.s())
    }
}

impl std::ops::Add for Hex {
    type Output = Hex;
    #[inline]
    fn add(self, other: Hex) -> Hex {
        Hex::new(self.q + other.q, self.r + other.r)
    }
}

impl std::ops::Sub for Hex {
    type Output = Hex;
    #[inline]
    fn sub(self, other: Hex) -> Hex {
        Hex::new(self.q - other.q, self.r - other.r)
    }
}

impl DetHash for Hex {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.q);
        hasher.write_i32(self.r);
    }
}

/// The six axial directions, counter-clockwise starting east.
/// `DIRECTIONS[i]` steps to neighbor `i`.
pub const DIRECTIONS: [Hex; 6] = [
    Hex { q: 1, r: 0 },
    Hex { q: 1, r: -1 },
    Hex { q: 0, r: -1 },
    Hex { q: -1, r: 0 },
    Hex { q: -1, r: 1 },
    Hex { q: 0, r: 1 },
];

/// Hex (Manhattan-on-cube) distance between `a` and `b`:
/// `( |Δq| + |Δr| + |Δs| ) / 2`. Equivalent to the number of hex steps on the
/// shortest path.
#[inline]
pub fn distance(a: Hex, b: Hex) -> i32 {
    let dq = (a.q - b.q) as i64;
    let dr = (a.r - b.r) as i64;
    let ds = dq + dr;
    ((dq.abs() + dr.abs() + ds.abs()) / 2) as i32
}

/// Round a fractional cube coordinate to the nearest hex. The coordinate is
/// given as three numerators over a common denominator: the hex is
/// `(qn / d, rn / d, sn / d)`. `d` must be positive and the numerators must
/// satisfy `qn + rn + sn == 0` (the caller's responsibility for exactness —
/// arbitrary triples still produce *a* nearest hex).
///
/// Rounds all three components to the nearest integer, then recomputes the
/// component whose rounding error was largest as `-q - r` of the other two —
/// restoring the `q + r + s = 0` invariant. Ties round away from zero.
pub fn cube_round(qn: i64, rn: i64, sn: i64, d: i64) -> Hex {
    let rx = round_div(qn, d);
    let ry = round_div(rn, d);
    let rz = round_div(sn, d);
    let dq = (rx * d - qn).abs();
    let dr = (ry * d - rn).abs();
    let ds = (rz * d - sn).abs();
    let (mut q, mut r) = (rx, ry);
    if dq > dr && dq > ds {
        q = -ry - rz;
    } else if dr > ds {
        r = -rx - rz;
    }
    // Clamp to i32 — inputs this large are far past playable map sizes.
    Hex {
        q: q.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        r: r.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
    }
}

/// `n / d` rounded to the nearest integer, ties away from zero. `d > 0`.
fn round_div(n: i64, d: i64) -> i64 {
    let q = n / d;
    let rem = n % d;
    if 2 * rem.abs() >= d {
        q + if n < 0 { -1 } else { 1 }
    } else {
        q
    }
}

/// Draw a hex line from `a` to `b` inclusive, using cube linear
/// interpolation with integer-exact rounding. Returns `distance + 1` hexes;
/// consecutive hexes are always neighbors, and every returned hex lies on a
/// shortest `a → b` path (`d(a, h) + d(h, b) == d(a, b)`).
pub fn line(a: Hex, b: Hex) -> Vec<Hex> {
    let n = distance(a, b) as i64;
    if n == 0 {
        return vec![a];
    }
    let mut out = Vec::with_capacity(n as usize + 1);
    for i in 0..=n {
        // Cube lerp on numerators: coord = a + (b - a) * i / n.
        let qn = a.q as i64 * n + (b.q - a.q) as i64 * i;
        let rn = a.r as i64 * n + (b.r - a.r) as i64 * i;
        let sn = -(qn + rn);
        out.push(cube_round(qn, rn, sn, n));
    }
    out
}

/// The ring of hexes exactly `radius` steps from `center` — `6 * radius`
/// cells, in `DIRECTIONS` order starting at the southwest corner. A
/// `radius` of 0 returns just `center`.
pub fn ring(center: Hex, radius: i32) -> Vec<Hex> {
    if radius <= 0 {
        return vec![center];
    }
    let mut out = Vec::with_capacity(6 * radius as usize);
    // Start one step past the last direction so the walk spirals inward to
    // the ring's start corner (matches the guide's ring construction).
    let mut h = center + (DIRECTIONS[4].scale(radius));
    for dir in 0..6 {
        for _ in 0..radius {
            out.push(h);
            h = h.neighbor(dir);
        }
    }
    out
}

/// Every hex within `radius` steps of `center`, inclusive — `1 + 3·r·(r+1)`
/// cells, ordered by `q` band then `r`. Equivalent to the hex-shaped range
/// scan `|dq| ≤ r ∧ |dr| ≤ r ∧ |dq + dr| ≤ r`.
pub fn spiral(center: Hex, radius: i32) -> Vec<Hex> {
    if radius < 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for dq in -radius..=radius {
        let lo = (-radius).max(-dq - radius);
        let hi = radius.min(-dq + radius);
        for dr in lo..=hi {
            out.push(center + (Hex::new(dq, dr)));
        }
    }
    out
}

/// Convert an **odd-r** offset coordinate (pointy-top layout: odd rows are
/// shifted half a column right) to axial.
#[inline]
pub fn from_offset_odd_r(col: i32, row: i32) -> Hex {
    Hex::new(col - (row - row.rem_euclid(2)) / 2, row)
}

/// Convert axial to **odd-r** offset coordinates, `(col, row)`.
#[inline]
pub fn to_offset_odd_r(h: Hex) -> (i32, i32) {
    (h.q + (h.r - h.r.rem_euclid(2)) / 2, h.r)
}

/// Convert an **even-r** offset coordinate to axial.
#[inline]
pub fn from_offset_even_r(col: i32, row: i32) -> Hex {
    Hex::new(col - (row + row.rem_euclid(2)) / 2, row)
}

/// Convert axial to **even-r** offset coordinates, `(col, row)`.
#[inline]
pub fn to_offset_even_r(h: Hex) -> (i32, i32) {
    (h.q + (h.r + h.r.rem_euclid(2)) / 2, h.r)
}

/// A random hex within `radius` steps of `center`, drawn uniformly by
/// rejection from the axial bounding box. Deterministic under `rng`.
pub fn random_in_range(center: Hex, radius: i32, rng: &mut SplitMix64) -> Hex {
    if radius <= 0 {
        return center;
    }
    let dq = rng.range(-radius, radius + 1);
    // Valid dr for this dq: |dr| <= radius AND |dq + dr| <= radius.
    let dr_lo = (-radius).max(-dq - radius);
    let dr_hi = radius.min(-dq + radius);
    center + (Hex::new(dq, rng.range(dr_lo, dr_hi + 1)))
}

/// A* on the hex grid (redblobgames formulation): unit-cost steps over the
/// six [`DIRECTIONS`], with the exact hex [`distance`] as the admissible —
/// and consistent — heuristic, so every node is expanded at most once and
/// the returned path is always a true shortest path.
///
/// `passable` decides whether a cell may be entered (the start and goal are
/// always allowed regardless). `max_steps` bounds the search: at most that
/// many cells are relaxed before giving up — required because the hex
/// lattice is unbounded and a walled-in goal would otherwise search
/// forever. The map scale sets a sane bound; the hex-disc of radius
/// `distance(start, goal)` holds `1 + 3d(d+1)` cells, so twice that is a
/// generous default for open maps.
///
/// Returns `Some(path)` — start-inclusive, goal-inclusive — or `None` when
/// the goal is unreachable or the step budget runs out. Deterministic: the
/// open set pops lowest `f`, breaking ties by lowest `h` then by
/// `(q, r)` order, and all bookkeeping lives in ordered maps.
///
/// ```
/// use izanagi_kit::hexgrid::{hex_astar, distance, Hex};
/// let path = hex_astar(Hex::new(0, 0), Hex::new(3, -1), |_| true, 1000).unwrap();
/// assert_eq!(path.len() as i32, distance(Hex::new(0, 0), Hex::new(3, -1)) + 1);
/// ```
pub fn hex_astar(
    start: Hex,
    goal: Hex,
    mut passable: impl FnMut(Hex) -> bool,
    max_steps: u32,
) -> Option<Vec<Hex>> {
    use std::cmp::Reverse;
    use std::collections::{BTreeMap, BinaryHeap};

    if start == goal {
        return Some(vec![start]);
    }
    // g-score and came_from keyed by (q, r); open heap by (f, h, q, r).
    let mut g: BTreeMap<(i32, i32), u32> = BTreeMap::new();
    let mut came_from: BTreeMap<(i32, i32), Hex> = BTreeMap::new();
    let mut open: BinaryHeap<Reverse<(u32, u32, i32, i32)>> = BinaryHeap::new();
    let mut closed = std::collections::BTreeSet::new();

    g.insert((start.q, start.r), 0);
    let h0 = distance(start, goal) as u32;
    open.push(Reverse((h0, h0, start.q, start.r)));
    let mut relaxed = 0u32;

    while let Some(Reverse((_f, _h, q, r))) = open.pop() {
        if !closed.insert((q, r)) {
            continue; // stale heap entry
        }
        let cur = Hex::new(q, r);
        if cur == goal {
            // Reconstruct.
            let mut path = vec![goal];
            let mut at = goal;
            while at != start {
                at = came_from[&(at.q, at.r)];
                path.push(at);
            }
            path.reverse();
            return Some(path);
        }
        relaxed += 1;
        if relaxed > max_steps {
            return None;
        }
        let g_cur = g[&(q, r)];
        for n in cur.neighbors() {
            if !passable(n) && n != goal {
                continue;
            }
            let key = (n.q, n.r);
            if closed.contains(&key) {
                continue;
            }
            let g_new = g_cur + 1;
            let better = match g.get(&key) {
                Some(&old) => g_new < old,
                None => true,
            };
            if better {
                g.insert(key, g_new);
                came_from.insert(key, cur);
                let hn = distance(n, goal) as u32;
                open.push(Reverse((g_new + hn, hn, n.q, n.r)));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BFS shortest-path oracle on a bounded hex region — the ground truth
    /// `distance` and `line` must agree with.
    fn bfs_distance(a: Hex, b: Hex) -> i64 {
        let mut frontier = vec![a];
        let mut seen = std::collections::BTreeSet::from([(a.q, a.r)]);
        let mut steps = 0;
        loop {
            if frontier.contains(&b) {
                return steps;
            }
            let mut next = Vec::new();
            for &h in &frontier {
                for n in h.neighbors() {
                    if seen.insert((n.q, n.r)) {
                        next.push(n);
                    }
                }
            }
            frontier = next;
            steps += 1;
            assert!(steps < 100, "BFS should reach any hex quickly");
        }
    }

    #[test]
    fn distance_matches_bfs_oracle() {
        let mut rng = SplitMix64::new(0x5EED);
        for _ in 0..500 {
            let a = Hex::new(rng.range(-20, 21), rng.range(-20, 21));
            let b = Hex::new(rng.range(-20, 21), rng.range(-20, 21));
            assert_eq!(distance(a, b) as i64, bfs_distance(a, b));
        }
    }

    #[test]
    fn distance_is_a_metric() {
        let a = Hex::new(3, -5);
        let b = Hex::new(-2, 7);
        let c = Hex::new(0, 0);
        assert_eq!(distance(a, a), 0);
        assert_eq!(distance(a, b), distance(b, a));
        assert!(distance(a, b) <= distance(a, c) + distance(c, b));
        assert_eq!(a.s(), -a.q - a.r);
        // (2,3) has s = -5 → distance (2+3+5)/2 = 5.
        assert_eq!(Hex::new(2, 3).distance_from_origin(), 5);
    }

    #[test]
    fn line_is_a_shortest_path() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..300 {
            let a = Hex::new(rng.range(-15, 16), rng.range(-15, 16));
            let b = Hex::new(rng.range(-15, 16), rng.range(-15, 16));
            let path = line(a, b);
            assert_eq!(path.len() as i32, distance(a, b) + 1);
            assert_eq!(path[0], a);
            assert_eq!(path[path.len() - 1], b);
            for w in path.windows(2) {
                assert_eq!(distance(w[0], w[1]), 1, "consecutive hexes adjacent");
            }
            for &h in &path {
                assert_eq!(
                    distance(a, h) + distance(h, b),
                    distance(a, b),
                    "every line hex lies on a shortest path"
                );
            }
        }
    }

    #[test]
    fn ring_geometry() {
        let c = Hex::new(2, -3);
        assert_eq!(ring(c, 0), vec![c]);
        for radius in 1..8 {
            let r = ring(c, radius);
            assert_eq!(r.len(), 6 * radius as usize);
            for &h in &r {
                assert_eq!(distance(c, h), radius);
            }
        }
    }

    #[test]
    fn spiral_covers_the_hex_disc() {
        let c = Hex::new(-1, 2);
        for radius in 0..=5 {
            let cells = spiral(c, radius);
            assert_eq!(cells.len(), 1 + 3 * radius as usize * (radius as usize + 1));
            for &h in &cells {
                assert!(distance(c, h) <= radius);
            }
            // Every in-range hex is present exactly once.
            let unique: std::collections::BTreeSet<_> = cells.iter().map(|h| (h.q, h.r)).collect();
            assert_eq!(unique.len(), cells.len());
        }
        assert!(spiral(c, -1).is_empty());
    }

    #[test]
    fn offset_conversions_round_trip() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..1000 {
            let h = Hex::new(rng.range(-30, 31), rng.range(-30, 31));
            let (col, row) = to_offset_odd_r(h);
            assert_eq!(from_offset_odd_r(col, row), h, "odd-r round trip");
            let (col, row) = to_offset_even_r(h);
            assert_eq!(from_offset_even_r(col, row), h, "even-r round trip");
        }
        // Sanity: offset (0,0) maps to axial (0,0) in both layouts.
        assert_eq!(from_offset_odd_r(0, 0), Hex::ORIGIN);
        assert_eq!(from_offset_even_r(0, 0), Hex::ORIGIN);
    }

    #[test]
    fn neighbors_and_rotations() {
        let h = Hex::new(4, -2);
        for (i, n) in h.neighbors().iter().enumerate() {
            assert_eq!(*n, h + (DIRECTIONS[i]));
            assert_eq!(h.neighbor(i as u32), *n);
            assert_eq!(distance(h, *n), 1);
        }
        let mut rotated = h;
        for _ in 0..6 {
            rotated = rotated.rotate_left();
        }
        assert_eq!(rotated, h, "six 60° rotations return to start");
        assert_eq!(h.rotate_left().rotate_right(), h);
    }

    #[test]
    fn cube_round_picks_the_largest_error_component() {
        // Numerators (qn, rn, sn) must sum to 0.
        let h = cube_round(5, -3, -2, 2); // (2.5, -1.5, -1.0) → fixes q
        assert_eq!(h.q + h.r + h.s(), 0);
        // Exact integer input passes through.
        assert_eq!(cube_round(6, -4, -2, 2), Hex::new(3, -2));
    }

    #[test]
    fn random_in_range_stays_in_range_and_deterministic() {
        let c = Hex::new(1, 1);
        let mut rng = SplitMix64::new(99);
        for _ in 0..200 {
            let h = random_in_range(c, 5, &mut rng);
            assert!(distance(c, h) <= 5);
        }
        assert_eq!(random_in_range(c, 0, &mut rng), c);
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(1);
        assert_eq!(random_in_range(c, 7, &mut a), random_in_range(c, 7, &mut b));
    }

    #[test]
    fn hex_det_hash_is_stable() {
        // Cross-run pin: same hex, same bytes, same hash.
        assert_eq!(hash_of(Hex::new(3, -5)), hash_of(Hex::new(3, -5)));
        assert_ne!(hash_of(Hex::new(3, -5)), hash_of(Hex::new(-5, 3)));
    }

    fn hash_of(h: Hex) -> u64 {
        crate::world_hash::hash_state(&h)
    }

    /// BFS oracle on an obstacle field: the true shortest-path length.
    fn bfs_len(
        start: Hex,
        goal: Hex,
        blocked: &std::collections::BTreeSet<(i32, i32)>,
    ) -> Option<usize> {
        let mut frontier = vec![start];
        let mut seen = std::collections::BTreeSet::from([(start.q, start.r)]);
        let mut steps = 0usize;
        loop {
            if frontier.contains(&goal) {
                return Some(steps);
            }
            if seen.len() > 5000 {
                return None; // bounded region — unreachable
            }
            let mut next = Vec::new();
            for &h in &frontier {
                for n in h.neighbors() {
                    if !blocked.contains(&(n.q, n.r)) && seen.insert((n.q, n.r)) {
                        next.push(n);
                    }
                }
            }
            if next.is_empty() {
                return None;
            }
            frontier = next;
            steps += 1;
        }
    }

    #[test]
    fn hex_astar_matches_bfs_oracle() {
        let mut rng = SplitMix64::new(0xA57A);
        for _ in 0..200 {
            // Random obstacle field around the origin.
            let mut blocked = std::collections::BTreeSet::new();
            for h in spiral(Hex::ORIGIN, 8) {
                if rng.below(100) < 25 {
                    blocked.insert((h.q, h.r));
                }
            }
            let a = Hex::new(rng.range(-6, 7), rng.range(-6, 7));
            let b = Hex::new(rng.range(-6, 7), rng.range(-6, 7));
            blocked.remove(&(a.q, a.r));
            blocked.remove(&(b.q, b.r));
            let path = hex_astar(a, b, |h| !blocked.contains(&(h.q, h.r)), 5000);
            match (path, bfs_len(a, b, &blocked)) {
                (None, None) => {}
                (Some(p), Some(d)) => {
                    assert_eq!(p.len() - 1, d, "path is shortest");
                    assert_eq!(p[0], a);
                    assert_eq!(p[p.len() - 1], b);
                    for w in p.windows(2) {
                        assert_eq!(distance(w[0], w[1]), 1);
                        assert!(!blocked.contains(&(w[1].q, w[1].r)));
                    }
                }
                (got, oracle) => panic!(
                    "mismatch: astar={:?} bfs={:?}",
                    got.map(|p| p.len()),
                    oracle
                ),
            }
        }
    }

    #[test]
    fn hex_astar_walled_goal_and_budget() {
        // Goal fully walled off by its own neighbors → unreachable.
        let goal = Hex::new(2, -1);
        let walls: std::collections::BTreeSet<(i32, i32)> =
            goal.neighbors().iter().map(|h| (h.q, h.r)).collect();
        assert!(hex_astar(Hex::ORIGIN, goal, |h| !walls.contains(&(h.q, h.r)), 10_000).is_none());
        // Tiny budget cannot finish even a clear run.
        assert!(hex_astar(Hex::ORIGIN, Hex::new(50, 0), |_| true, 10).is_none());
        // Same start and goal.
        assert_eq!(
            hex_astar(Hex::ORIGIN, Hex::ORIGIN, |_| false, 1),
            Some(vec![Hex::ORIGIN])
        );
    }

    #[test]
    fn hex_astar_is_deterministic() {
        let run = || {
            hex_astar(
                Hex::new(-3, 1),
                Hex::new(4, -2),
                |h| !(h.q == 0 && h.r == 0),
                10_000,
            )
        };
        assert_eq!(run(), run());
    }
}

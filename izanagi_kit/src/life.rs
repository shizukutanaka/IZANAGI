//! Sparse, deterministic Conway's Game of Life over an unbounded
//! `i64` lattice (B3/S23). The live set is a `BTreeSet<(i64, i64)>` —
//! the state IS its own canonical form: same cells ⇒ same future
//! trace, no ordering noise, no floating boundary to normalize.
//! HashLife-style quadtree memoization is deliberately skipped; this
//! is the verified sparse reference implementation, matching the
//! module's role across the kit.
//!
//! ```
//! use izanagi_kit::life::World;
//! // Blinker: period-2 oscillator.
//! let mut w = World::from_cells(&[(0, -1), (0, 0), (0, 1)]);
//! w.step();
//! assert_eq!(w.population(), 3);
//! w.step();
//! assert_eq!(w.population(), 3);
//! ```
//!
//! Reference: Gardner (1970) for B3/S23; Gosper (1984) for the sparse
//! neighbor-count update.

/// Sparse B3/S23 world.
#[derive(Clone, Default)]
pub struct World {
    cells: std::collections::BTreeSet<(i64, i64)>,
}

impl World {
    /// Empty world.
    pub fn new() -> World {
        World {
            cells: std::collections::BTreeSet::new(),
        }
    }

    /// World from an explicit live-cell list.
    pub fn from_cells(cells: &[(i64, i64)]) -> World {
        World {
            cells: cells.iter().copied().collect(),
        }
    }

    /// Number of live cells.
    pub fn population(&self) -> usize {
        self.cells.len()
    }

    /// Whether `(x, y)` is alive.
    pub fn alive(&self, x: i64, y: i64) -> bool {
        self.cells.contains(&(x, y))
    }

    /// Set a cell alive.
    pub fn set(&mut self, x: i64, y: i64) {
        self.cells.insert((x, y));
    }

    /// Kill a cell.
    pub fn kill(&mut self, x: i64, y: i64) {
        self.cells.remove(&(x, y));
    }

    /// Bounding box `(min_x, min_y, max_x, max_y)` of the live set —
    /// `None` when empty.
    pub fn bounds(&self) -> Option<(i64, i64, i64, i64)> {
        let mut it = self.cells.iter();
        let &(x0, y0) = it.next()?;
        let (mut xa, mut ya, mut xb, mut yb) = (x0, y0, x0, y0);
        for &(x, y) in it {
            xa = xa.min(x);
            xb = xb.max(x);
            ya = ya.min(y);
            yb = yb.max(y);
        }
        Some((xa, ya, xb, yb))
    }

    /// Live cells in canonical sorted order.
    pub fn cells(&self) -> Vec<(i64, i64)> {
        self.cells.iter().copied().collect()
    }

    /// One B3/S23 generation.
    ///
    /// Neighbor counts are accumulated over a `BTreeMap` so the
    /// birth/survival scan is a single sorted pass — output is a pure
    /// function of the input cell set.
    pub fn step(&mut self) {
        let mut counts: std::collections::BTreeMap<(i64, i64), u8> =
            std::collections::BTreeMap::new();
        for &(x, y) in &self.cells {
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    if dx != 0 || dy != 0 {
                        *counts.entry((x + dx, y + dy)).or_insert(0) += 1;
                    }
                }
            }
        }
        let mut next: std::collections::BTreeSet<(i64, i64)> = std::collections::BTreeSet::new();
        for (cell, n) in counts {
            let alive = self.cells.contains(&cell);
            if n == 3 || (alive && n == 2) {
                next.insert(cell);
            }
        }
        self.cells = next;
    }

    /// Advance `generations` steps.
    pub fn run(&mut self, generations: usize) {
        for _ in 0..generations {
            self.step();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn still_lifes_invariant() {
        // Block and beehive are stable.
        for cells in [
            vec![(0, 0), (1, 0), (0, 1), (1, 1)],
            vec![(1, 0), (2, 0), (0, 1), (3, 1), (1, 2), (2, 2)],
        ] {
            let mut w = World::from_cells(&cells);
            w.step();
            assert_eq!(w.cells(), {
                let mut v = cells;
                v.sort();
                v
            });
        }
    }

    #[test]
    fn glider_translates() {
        // Glider returns to its shape, shifted (+1, +1), every 4 steps.
        let base = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];
        let mut w = World::from_cells(&base);
        w.run(4);
        let mut want: Vec<(i64, i64)> = base.iter().map(|&(x, y)| (x + 1, y + 1)).collect();
        want.sort();
        assert_eq!(w.cells(), want);
    }

    #[test]
    fn dense_grid_oracle() {
        // Sparse world vs a dense W×H simulator with hard walls on a
        // random soup, 12 generations — states must agree exactly on
        // the bounded region (soup kept away from walls).
        const W: i64 = 24;
        const H: i64 = 24;
        let mut rng = SplitMix64::new(0x11fe_11fe_11fe_11fe);
        for trial in 0..30 {
            let mut sparse = World::new();
            let mut dense: BTreeSet<(i64, i64)> = BTreeSet::new();
            // Spawn a random 8x8 soup centered in the grid.
            for _ in 0..20 {
                let x = 8 + rng.below(8) as i64;
                let y = 8 + rng.below(8) as i64;
                sparse.set(x, y);
                dense.insert((x, y));
            }
            for gen in 0..12 {
                sparse.step();
                // Dense step within [0, W) x [0, H).
                let mut counts: std::collections::BTreeMap<(i64, i64), u8> =
                    std::collections::BTreeMap::new();
                for &(x, y) in &dense {
                    for dy in -1i64..=1 {
                        for dx in -1i64..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let (nx, ny) = (x + dx, y + dy);
                            if (0..W).contains(&nx) && (0..H).contains(&ny) {
                                *counts.entry((nx, ny)).or_insert(0) += 1;
                            }
                        }
                    }
                }
                let mut next = BTreeSet::new();
                for (cell, n) in counts {
                    if n == 3 || (dense.contains(&cell) && n == 2) {
                        next.insert(cell);
                    }
                }
                dense = next;
                assert_eq!(
                    sparse.cells(),
                    dense.iter().copied().collect::<Vec<_>>(),
                    "trial {trial} generation {gen}"
                );
                if dense.is_empty() {
                    break;
                }
            }
        }
    }

    #[test]
    fn determinism() {
        // Same seed ⇒ identical trace across rebuilds.
        let build = || {
            let mut rng = SplitMix64::new(99);
            let mut w = World::new();
            for _ in 0..50 {
                w.set(rng.below(30) as i64, rng.below(30) as i64);
            }
            let mut trace = Vec::new();
            for _ in 0..10 {
                w.step();
                trace.push(w.cells());
            }
            trace
        };
        assert_eq!(build(), build());
    }

    #[test]
    fn bounds_and_membership() {
        let mut w = World::from_cells(&[(-5, 2), (7, -9), (0, 0)]);
        assert_eq!(w.bounds(), Some((-5, -9, 7, 2)));
        assert!(w.alive(-5, 2));
        w.kill(-5, 2);
        assert!(!w.alive(-5, 2));
        assert_eq!(w.population(), 2);
        w.kill(7, -9);
        w.kill(0, 0);
        assert_eq!(w.bounds(), None);
    }
}

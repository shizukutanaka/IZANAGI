//! 2-D Fenwick tree (binary indexed tree) — point updates and
//! prefix/rectangle sum queries in `O(log w · log h)` over `i64`
//! weights.
//!
//! The 1-D [`crate::fenwick`] answers prefix sums on a line; this is
//! the grid version for influence maps, resource fields, and damage
//! heat-maps: `add` a cell, `prefix`/`rect_sum`/`total` read back.
//! All arithmetic is `i64` — bit-identical across platforms.
//!
//! ```
//! use izanagi_kit::fenwick2d::Fenwick2d;
//! let mut f = Fenwick2d::new(4, 4);
//! f.add(1, 1, 5);
//! f.add(2, 3, 7);
//! assert_eq!(f.prefix(2, 2), 5);        // cells [0..2) x [0..2)
//! assert_eq!(f.rect_sum(1, 1, 3, 4), 12);
//! assert_eq!(f.total(), 12);
//! ```

/// 2-D BIT over a `w × h` cell grid. Row-major storage,
/// `O(w·h)` cells.
#[derive(Clone, Debug)]
pub struct Fenwick2d {
    w: usize,
    h: usize,
    /// BIT storage, row-major `y * w + x` with 1-based indexing inside.
    t: Vec<i64>,
}

impl Fenwick2d {
    /// A `w × h` zeroed tree.
    pub fn new(w: usize, h: usize) -> Fenwick2d {
        Fenwick2d {
            w,
            h,
            t: vec![0; w * h],
        }
    }

    /// `grid[x][y] += delta`. Out-of-range cells are ignored.
    pub fn add(&mut self, x: usize, y: usize, delta: i64) {
        if x >= self.w || y >= self.h {
            return;
        }
        let mut i = x + 1;
        while i <= self.w {
            let mut j = y + 1;
            while j <= self.h {
                // 1-based BIT indices mapped into row-major storage.
                let idx = (i - 1) * self.h + (j - 1);
                self.t[idx] += delta;
                j += j & j.wrapping_neg();
            }
            i += i & i.wrapping_neg();
        }
    }

    /// Sum over `[0..x) × [0..y)`.
    pub fn prefix(&self, x: usize, y: usize) -> i64 {
        let mut s = 0i64;
        let mut i = x.min(self.w);
        while i > 0 {
            let mut j = y.min(self.h);
            while j > 0 {
                s += self.t[(i - 1) * self.h + (j - 1)];
                j -= j & j.wrapping_neg();
            }
            i -= i & i.wrapping_neg();
        }
        s
    }

    /// Sum over `[x0..x1) × [y0..y1)` — clamps to the grid and returns
    /// 0 for empty or inverted ranges.
    pub fn rect_sum(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {
        let (x1, y1) = (x1.min(self.w), y1.min(self.h));
        let (x0, y0) = (x0.min(x1), y0.min(y1));
        self.prefix(x1, y1) - self.prefix(x0, y1) - self.prefix(x1, y0) + self.prefix(x0, y0)
    }

    /// Sum of every cell.
    pub fn total(&self) -> i64 {
        self.prefix(self.w, self.h)
    }

    /// Value of a single cell — `rect_sum(x, y, x+1, y+1)`; `None`
    /// out of range.
    pub fn get(&self, x: usize, y: usize) -> Option<i64> {
        if x >= self.w || y >= self.h {
            return None;
        }
        Some(self.rect_sum(x, y, x + 1, y + 1))
    }

    /// Grid width.
    pub fn width(&self) -> usize {
        self.w
    }

    /// Grid height.
    pub fn height(&self) -> usize {
        self.h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: dense grid of cells, brute-force scans.
    struct Dense {
        g: Vec<i64>,
        w: usize,
        h: usize,
    }
    impl Dense {
        fn new(w: usize, h: usize) -> Dense {
            Dense {
                g: vec![0; w * h],
                w,
                h,
            }
        }
        fn add(&mut self, x: usize, y: usize, d: i64) {
            if x < self.w && y < self.h {
                self.g[y * self.w + x] += d;
            }
        }
        fn rect(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> i64 {
            let (x1, y1) = (x1.min(self.w), y1.min(self.h));
            let (x0, y0) = (x0.min(x1), y0.min(y1));
            (y0..y1)
                .flat_map(|y| (x0..x1).map(move |x| self.g[y * self.w + x]))
                .sum()
        }
    }

    #[test]
    fn matches_dense_oracle_on_random_ops() {
        let mut rng = SplitMix64::new(0xFE2D);
        for _ in 0..60 {
            let w = (rng.below(9) + 1) as usize;
            let h = (rng.below(9) + 1) as usize;
            let mut f = Fenwick2d::new(w, h);
            let mut d = Dense::new(w, h);
            for _ in 0..60 {
                let x = rng.below(w as u32 + 2) as usize;
                let y = rng.below(h as u32 + 2) as usize;
                let v = rng.below(41) as i64 - 20;
                f.add(x, y, v);
                d.add(x, y, v);
                // Prefix, rect, and point reads — including out-of-range.
                let (ax, ay) = (
                    rng.below(w as u32 + 2) as usize,
                    rng.below(h as u32 + 2) as usize,
                );
                let (bx, by) = (
                    rng.below(w as u32 + 2) as usize,
                    rng.below(h as u32 + 2) as usize,
                );
                let (x0, x1) = (ax.min(bx), ax.max(bx));
                let (y0, y1) = (ay.min(by), ay.max(by));
                assert_eq!(f.rect_sum(x0, y0, x1, y1), d.rect(x0, y0, x1, y1));
                assert_eq!(f.prefix(x0, y0), d.rect(0, 0, x0, y0));
                let pt = if x0 < w && y0 < h {
                    Some(d.rect(x0, y0, x0 + 1, y0 + 1))
                } else {
                    None
                };
                assert_eq!(f.get(x0, y0), pt);
            }
            assert_eq!(f.total(), d.rect(0, 0, w, h));
        }
    }

    #[test]
    fn negative_deltas_and_empties() {
        let mut f = Fenwick2d::new(3, 3);
        f.add(0, 0, -7);
        f.add(2, 2, 7);
        assert_eq!(f.prefix(1, 1), -7);
        assert_eq!(f.total(), 0);
        assert_eq!(f.rect_sum(1, 1, 2, 2), 0);
        // Empty / inverted ranges and degenerate trees.
        assert_eq!(f.rect_sum(2, 2, 1, 1), 0);
        assert_eq!(Fenwick2d::new(0, 0).total(), 0);
        assert_eq!(Fenwick2d::new(5, 0).prefix(5, 5), 0);
        assert_eq!(Fenwick2d::new(1, 1).get(0, 0), Some(0));
        assert_eq!(Fenwick2d::new(1, 1).get(1, 0), None);
    }

    #[test]
    fn full_grid_coverage() {
        let mut f = Fenwick2d::new(4, 3);
        for y in 0..3 {
            for x in 0..4 {
                f.add(x, y, (x + y * 4) as i64 + 1);
            }
        }
        assert_eq!(f.total(), 78);
        assert_eq!(f.rect_sum(1, 1, 3, 2), 13);
        assert_eq!(f.get(3, 2), Some(12));
    }
}

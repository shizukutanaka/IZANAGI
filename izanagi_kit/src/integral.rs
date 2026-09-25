//! Integral image (summed-area table) — Viola–Jones's `O(w·h)` build,
//! `O(1)` rectangle-sum answer. Cell `(x,y)` of the table holds the
//! sum of every source pixel in the rectangle `[(0,0),(x,y)]`; any
//! axis-aligned sum is then three lookups and a subtraction. The
//! raster companion to `ccl`/`worley`: mean filters, adaptive
//! thresholds, and box statistics all collapse into `sum_rect`.
//!
//! ```
//! use izanagi_kit::integral::Sat;
//!
//! // 4×4 all-ones image.
//! let sat = Sat::new(4, 4, &[1u32; 16]);
//! assert_eq!(sat.sum_rect(1, 1, 2, 2), 4);
//! assert_eq!(sat.sum_rect(0, 0, 4, 4), 16);
//! assert_eq!(sat.sum_rect(3, 3, 1, 1), 1);
//! ```

/// Summed-area table over `u32` pixels (cells accumulate to `u64`,
/// so a 256×255-max image never overflows `i64` math either).
/// Indexing: `cell(x,y)` covers the source rectangle `[0..x]×[0..y]`
/// — an `(w+1)×(h+1)` table with a zero border, so every query is
/// branch-free.
pub struct Sat {
    width: usize,
    height: usize,
    cells: Vec<u64>,
}

impl Sat {
    /// Build from a `width×height` row-major pixel slice. A
    /// length mismatch yields an empty `0×0` table rather than a
    /// panic (the G7 contract: bad input degrades, never faults).
    pub fn new(width: usize, height: usize, data: &[u32]) -> Self {
        if data.len() != width * height {
            return Sat {
                width: 0,
                height: 0,
                cells: vec![0],
            };
        }
        let stride = width + 1;
        let mut cells = vec![0u64; stride * (height + 1)];
        for y in 0..height {
            let mut row_sum = 0u64;
            for x in 0..width {
                row_sum += data[y * width + x] as u64;
                cells[(y + 1) * stride + (x + 1)] = cells[y * stride + (x + 1)] + row_sum;
            }
        }
        Sat {
            width,
            height,
            cells,
        }
    }

    /// Source dimensions `(width, height)`.
    pub fn dims(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn stride(&self) -> usize {
        self.width + 1
    }

    /// Sum of the source rectangle `[x, x+w) × [y, y+h)`, clamped to
    /// the image bounds (an out-of-bounds edge contributes nothing).
    pub fn sum_rect(&self, x: usize, y: usize, w: usize, h: usize) -> u64 {
        let x1 = (x + w).min(self.width);
        let y1 = (y + h).min(self.height);
        if x >= x1 || y >= y1 {
            return 0;
        }
        let s = self.stride();
        // A + D − B − C, not A − B − C + D: the latter underflows
        // when the top-left corner D is nonzero.
        self.cells[y1 * s + x1] + self.cells[y * s + x]
            - self.cells[y * s + x1]
            - self.cells[y1 * s + x]
    }

    /// Sum of every pixel — `sum_rect` over the whole image.
    pub fn total(&self) -> u64 {
        self.sum_rect(0, 0, self.width, self.height)
    }

    /// `⌊sum/w·h⌋` mean of the rectangle (0 when the clamped area
    /// is empty).
    pub fn mean_rect(&self, x: usize, y: usize, w: usize, h: usize) -> u64 {
        let x1 = (x + w).min(self.width);
        let y1 = (y + h).min(self.height);
        if x >= x1 || y >= y1 {
            return 0;
        }
        let area = ((x1 - x) * (y1 - y)) as u64;
        self.sum_rect(x, y, w, h) / area
    }

    /// Box-filtered image: each output pixel is the mean of the
    /// `(2r+1)²` square clamped around it — the O(1)-per-pixel blur.
    pub fn box_mean(&self, r: usize) -> Vec<u64> {
        let mut out = Vec::with_capacity(self.width * self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let (x0, y0) = (x.saturating_sub(r), y.saturating_sub(r));
                out.push(self.mean_rect(x0, y0, x + r + 1 - x0, y + r + 1 - y0));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O(w·h) brute rectangle sum.
    fn brute(data: &[u32], w: usize, x: usize, y: usize, rw: usize, rh: usize, h: usize) -> u64 {
        let mut s = 0u64;
        for yy in y..(y + rh).min(h) {
            for xx in x..(x + rw).min(w) {
                s += data[yy * w + xx] as u64;
            }
        }
        s
    }

    #[test]
    fn matches_brute_everywhere() {
        let (w, h) = (7usize, 5usize);
        let data: Vec<u32> = (0..w * h).map(|i| (i * 37 + 11) as u32 % 251).collect();
        let sat = Sat::new(w, h, &data);
        for y in 0..h {
            for x in 0..w {
                for rh in 0..=(h - y) {
                    for rw in 0..=(w - x) {
                        assert_eq!(
                            sat.sum_rect(x, y, rw, rh),
                            brute(&data, w, x, y, rw, rh, h),
                            "{x},{y} {rw}x{rh}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn sums_and_means() {
        let data = [1u32, 2, 3, 4, 5, 6]; // 3×2
        let sat = Sat::new(3, 2, &data);
        assert_eq!(sat.dims(), (3, 2));
        assert_eq!(sat.total(), 21);
        assert_eq!(sat.sum_rect(1, 0, 2, 2), 2 + 3 + 5 + 6);
        assert_eq!(sat.sum_rect(0, 1, 3, 1), 4 + 5 + 6);
        assert_eq!(sat.mean_rect(0, 0, 2, 1), 1); // (1+2)/2 floor
                                                  // Clamped beyond the edge contributes nothing.
        assert_eq!(sat.sum_rect(2, 1, 9, 9), 6);
        // Empty rects are 0, never panic.
        assert_eq!(sat.sum_rect(0, 0, 0, 3), 0);
        assert_eq!(sat.mean_rect(9, 9, 1, 1), 0);
    }

    #[test]
    fn box_mean_matches_naive() {
        let (w, h, r) = (6usize, 4usize, 2usize);
        let data: Vec<u32> = (0..w * h).map(|i| (i * 53 + 7) as u32 % 97).collect();
        let sat = Sat::new(w, h, &data);
        let blur = sat.box_mean(r);
        for y in 0..h {
            for x in 0..w {
                let (x0, y0) = (x.saturating_sub(r), y.saturating_sub(r));
                let (x1, y1) = ((x + r + 1).min(w), (y + r + 1).min(h));
                let n: u64 = (0..y1 - y0)
                    .map(|dy| {
                        (0..x1 - x0)
                            .map(|dx| data[(y0 + dy) * w + x0 + dx] as u64)
                            .sum::<u64>()
                    })
                    .sum();
                assert_eq!(
                    blur[y * w + x],
                    n / ((x1 - x0) * (y1 - y0)) as u64,
                    "{x},{y}"
                );
            }
        }
    }

    #[test]
    fn deterministic_twice() {
        let data: Vec<u32> = (0..64).map(|i| i as u32).collect();
        let a = Sat::new(8, 8, &data);
        let b = Sat::new(8, 8, &data);
        assert_eq!(a.sum_rect(1, 2, 3, 4), b.sum_rect(1, 2, 3, 4));
        assert_eq!(a.box_mean(1), b.box_mean(1));
    }
}

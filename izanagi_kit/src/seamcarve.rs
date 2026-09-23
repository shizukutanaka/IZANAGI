//! Seam carving (Avidan–Shamir) on integer pixels — find the
//! minimum-energy 8-connected vertical/horizontal seam and remove it.
//!
//! Energy uses squared gradients: `e(x,y) = dx² + dy²` where `dx` is
//! `p[x+1] − p[x−1]` (one-sided at the borders, `0` when the image is
//! 1-wide — all `u32`, no float, no sign juggling). The seam DP walks
//! rows accumulating `e + min(prev[j−1..=j+1])`, picks the smallest
//! argmin end, and backtracks; ties resolve to the *leftmost* column,
//! which is what makes both the returned seam and the carved image
//! canonical.
//!
//! ```
//! use izanagi_kit::seamcarve::{energy, find_vseam, remove_vseam};
//! // A bright vertical stripe — carving removes a low-energy seam
//! // around it, never through it.
//! let img = vec![
//!     vec![0, 0, 200, 0, 0],
//!     vec![0, 0, 200, 0, 0],
//!     vec![0, 0, 200, 0, 0],
//! ];
//! let e = energy(&img);
//! // Gradient lives at the stripe edges, not inside it.
//! assert_eq!(e[0][1], 200 * 200);
//! assert_eq!(e[0][2], 0);
//! let seam = find_vseam(&e).unwrap();
//! assert_eq!(seam.len(), 3);
//! let carved = remove_vseam(&img, &seam).unwrap();
//! assert_eq!(carved[0].len(), 4);
//! ```

/// Squared-gradient energy map of a row-major image — same shape as
/// `img`. Empty rows or an empty image return an empty map; ragged
/// rows are padded with zeros to the widest row's width.
pub fn energy(img: &[Vec<i32>]) -> Vec<Vec<u32>> {
    let h = img.len();
    let w = img.iter().map(Vec::len).max().unwrap_or(0);
    if h == 0 || w == 0 {
        return Vec::new();
    }
    let at = |y: isize, x: isize| -> i64 {
        if x < 0 || y < 0 {
            return 0;
        }
        img.get(y as usize)
            .and_then(|r| r.get(x as usize))
            .copied()
            .unwrap_or(0) as i64
    };
    let mut out = vec![vec![0u32; w]; h];
    for (y, orow) in out.iter_mut().enumerate() {
        for (x, ocell) in orow.iter_mut().enumerate() {
            let dx = if w == 1 {
                0
            } else if x == 0 {
                at(y as isize, 1) - at(y as isize, 0)
            } else if x == w - 1 {
                at(y as isize, x as isize) - at(y as isize, (x - 1) as isize)
            } else {
                at(y as isize, (x + 1) as isize) - at(y as isize, (x - 1) as isize)
            };
            let dy = if h == 1 {
                0
            } else if y == 0 {
                at(1, x as isize) - at(0, x as isize)
            } else if y == h - 1 {
                at(y as isize, x as isize) - at((y - 1) as isize, x as isize)
            } else {
                at((y + 1) as isize, x as isize) - at((y - 1) as isize, x as isize)
            };
            let sq = |v: i64| (v as i128 * v as i128).min(i128::from(u32::MAX)) as u32;
            *ocell = sq(dx).saturating_add(sq(dy));
        }
    }
    out
}

/// Minimum-total-energy vertical seam: one column index per row,
/// consecutive columns differing by at most 1. `None` when the map is
/// empty. Leftmost-argmin tie-break keeps the result canonical.
pub fn find_vseam(e: &[Vec<u32>]) -> Option<Vec<usize>> {
    let h = e.len();
    let w = e.iter().map(Vec::len).max().unwrap_or(0);
    if h == 0 || w == 0 {
        return None;
    }
    let at = |y: usize, x: usize| -> u64 {
        e.get(y)
            .and_then(|r| r.get(x))
            .copied()
            .map(u64::from)
            .unwrap_or(0)
    };
    let mut dp = vec![vec![0u64; w]; h];
    let mut par = vec![vec![0usize; w]; h];
    for (x, cell) in dp[0].iter_mut().enumerate() {
        *cell = at(0, x);
    }
    for y in 1..h {
        let (head, tail) = dp.split_at_mut(y);
        let prev = &head[y - 1];
        let cur = &mut tail[0];
        for (x, cell) in cur.iter_mut().enumerate() {
            let lo = x.saturating_sub(1);
            let hi = (x + 1).min(w - 1);
            let mut bp = lo;
            let mut bv = prev[lo];
            for (p, &pv) in prev.iter().enumerate().take(hi + 1).skip(lo) {
                if pv < bv {
                    bv = pv;
                    bp = p;
                }
            }
            *cell = bv + at(y, x);
            par[y][x] = bp;
        }
    }
    let mut x = 0usize;
    for c in 1..w {
        if dp[h - 1][c] < dp[h - 1][x] {
            x = c;
        }
    }
    let mut seam = vec![0usize; h];
    for y in (0..h).rev() {
        seam[y] = x;
        x = par[y][x];
    }
    Some(seam)
}

/// The image with `seam[y]` removed from row `y` — `None` when the
/// seam is malformed (wrong length, column out of bounds, or a column
/// jump `> 1`). Rows narrower than the widest keep their own length.
pub fn remove_vseam(img: &[Vec<i32>], seam: &[usize]) -> Option<Vec<Vec<i32>>> {
    if seam.len() != img.len() {
        return None;
    }
    let w = img.iter().map(Vec::len).max().unwrap_or(0);
    let mut prev: Option<usize> = None;
    for &s in seam {
        if s >= w || prev.is_some_and(|p| s.abs_diff(p) > 1) {
            return None;
        }
        prev = Some(s);
    }
    let mut out = Vec::with_capacity(img.len());
    for (y, row) in img.iter().enumerate() {
        let mut nr = Vec::with_capacity(row.len().saturating_sub(1));
        for (x, &v) in row.iter().enumerate() {
            if x != seam[y] {
                nr.push(v);
            }
        }
        out.push(nr);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    /// Enumerate every legal seam by DP-free recursion, take min total.
    fn brute_seam(e: &[Vec<u32>], w: usize, h: usize) -> Option<(u64, Vec<usize>)> {
        if h == 0 || w == 0 {
            return None;
        }
        let at = |y: usize, x: usize| u64::from(e[y][x]);
        struct Go<'a> {
            w: usize,
            h: usize,
            at: &'a dyn Fn(usize, usize) -> u64,
            memo: BTreeMap<(usize, usize), (u64, Vec<usize>)>,
        }
        impl Go<'_> {
            fn run(&mut self, y: usize, x: usize) -> (u64, Vec<usize>) {
                if let Some(r) = self.memo.get(&(y, x)) {
                    return r.clone();
                }
                // A partial path (stop mid-image) is not a seam — the
                // base value is only the leaf row.
                let mut best = (u64::MAX, Vec::new());
                if y + 1 == self.h {
                    best = ((self.at)(y, x), vec![x]);
                } else {
                    let lo = x.saturating_sub(1);
                    let hi = (x + 1).min(self.w - 1);
                    for p in lo..=hi {
                        let (c, mut s) = self.run(y + 1, p);
                        let c = c + (self.at)(y, x);
                        if c < best.0 {
                            best = (c, {
                                let mut v = vec![x];
                                v.append(&mut s);
                                v
                            });
                        }
                    }
                }
                self.memo.insert((y, x), best.clone());
                best
            }
        }
        let mut g = Go {
            w,
            h,
            at: &at,
            memo: BTreeMap::new(),
        };
        let mut best: Option<(u64, Vec<usize>)> = None;
        for x in 0..w {
            let r = g.run(0, x);
            if best.as_ref().map_or(true, |(c, _)| r.0 < *c) {
                best = Some(r);
            }
        }
        best
    }

    #[test]
    fn known() {
        // Uniform image: every seam ties → leftmost column.
        let e = energy(&[vec![5; 4], vec![5; 4], vec![5; 4]]);
        let seam = find_vseam(&e).unwrap();
        assert_eq!(seam, vec![0, 0, 0]);
        // Malformed seams rejected: wrong length, out of bounds, column jump > 1.
        assert!(remove_vseam(&[vec![1, 2]], &[0, 0]).is_none());
        assert!(remove_vseam(&[vec![1, 2]], &[5]).is_none());
        assert!(remove_vseam(&[vec![1, 2, 3], vec![4, 5, 6]], &[0, 2]).is_none());
        assert!(remove_vseam(&[vec![1, 2], vec![3, 4]], &[0, 1]).is_some());
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0x5ea9_ca4e_7ace);
        for _ in 0..200 {
            let h = 1 + rng.below(6) as usize;
            let w = 1 + rng.below(6) as usize;
            let img: Vec<Vec<i32>> = (0..h)
                .map(|_| (0..w).map(|_| rng.below(30) as i32).collect())
                .collect();
            let e = energy(&img);
            assert_eq!(e.len(), h);
            assert_eq!(e[0].len(), w);
            // Energy oracle: recompute one pixel directly.
            let (y, x) = (rng.below(h as u32) as usize, rng.below(w as u32) as usize);
            let dx = match (x == 0, x + 1 == w) {
                (true, true) => 0i64,
                (true, false) => i64::from(img[y][x + 1]) - i64::from(img[y][x]),
                (false, true) => i64::from(img[y][x]) - i64::from(img[y][x - 1]),
                (false, false) => i64::from(img[y][x + 1]) - i64::from(img[y][x - 1]),
            };
            let dy = match (y == 0, y + 1 == h) {
                (true, true) => 0i64,
                (true, false) => i64::from(img[y + 1][x]) - i64::from(img[y][x]),
                (false, true) => i64::from(img[y][x]) - i64::from(img[y - 1][x]),
                (false, false) => i64::from(img[y + 1][x]) - i64::from(img[y - 1][x]),
            };
            let sq = |v: i64| (v as i128 * v as i128).min(i128::from(u32::MAX)) as u32;
            assert_eq!(e[y][x], sq(dx).saturating_add(sq(dy)));
            // Seam oracle: minimal total and valid steps.
            let seam = find_vseam(&e).unwrap();
            let (bcost, _) = brute_seam(&e, w, h).unwrap();
            let got: u64 = seam
                .iter()
                .enumerate()
                .map(|(y, &x)| u64::from(e[y][x]))
                .sum();
            assert_eq!(got, bcost);
            for wnd in seam.windows(2) {
                assert!(wnd[0].abs_diff(wnd[1]) <= 1);
            }
            // Removal keeps every row one cell shorter.
            let carved = remove_vseam(&img, &seam).unwrap();
            for (y, row) in carved.iter().enumerate() {
                assert_eq!(row.len(), img[y].len() - 1);
            }
        }
    }
}

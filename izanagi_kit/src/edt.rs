//! Squared Euclidean distance transform —
//! Felzenszwalb–Huttenlocher, exact over integers.
//!
//! The transform of a binary grid gives each cell the
//! squared distance to the nearest *set* cell. The classic
//! algorithm factors the problem into two 1-D passes over
//! lower envelopes of parabolas `f_i(x) = (x − i)² + g(i)`:
//!
//! - maintain candidate sites `v[k]` and rational
//!   breakpoints `z[k]` (start of the interval where
//!   `v[k]` wins);
//! - inserting parabola `q`: its intersection `s` with
//!   the current top `v[k]` is
//!   `s = ((g(q) + q²) − (g(v_k) + v_k²)) / (2·(q − v_k))`.
//!   If `s ≤ z[k]`, the top parabola is dominated — pop
//!   and retry. Comparisons stay in `i128` by
//!   cross-multiplying the rationals;
//! - then each `x` takes its value from the parabola whose
//!   interval contains it.
//!
//! Every quantity is integral except the breakpoints,
//! which are kept as `(num, den)` pairs — no floats.
//!
//! `BIG` marks cells with no reachable feature (the whole
//! grid unset on a pass, or `g = BIG` sites); `BIG =
//! i64::MAX / 4` so additions never overflow.
//!
//! ```
//! use izanagi_kit::edt::edt2;
//! let grid = vec![
//!     vec![false, false, false],
//!     vec![false, true, false],
//!     vec![false, false, false],
//! ];
//! let d = edt2(&grid);
//! assert_eq!(d[1][1], 0); // the site itself
//! assert_eq!(d[0][0], 2); // diagonal: 1² + 1²
//! assert_eq!(d[0][1], 1);
//! ```

/// Sentinel for "no feature reachable" — `i64::MAX/4` so
/// `v² + g(v)` never overflows.
pub const BIG: i64 = i64::MAX / 4;

/// 1-D squared EDT. `g[i]` is the initial height of
/// parabola `i` (`0` for feature sites, `BIG` for empty);
/// the result is `min_i (x − i)² + g(i)`.
pub fn edt1(g: &[i64]) -> Vec<i64> {
    let n = g.len();
    if n == 0 {
        return Vec::new();
    }
    let mut v: Vec<i64> = Vec::with_capacity(n); // site indices
                                                 // breakpoints as (num, den): interval for v[k] starts at
                                                 // z[k]; z[0] = −inf represented as num<0 den=1 with num=-BIG
    let mut z: Vec<(i128, i128)> = Vec::with_capacity(n + 1);
    // −∞ breakpoint: keep |num| small enough that
    // `zn · den` stays well inside i128 for den ≤ 2n
    let neg_inf: (i128, i128) = (i64::MIN as i128, 1);
    for (qq, &gq) in g.iter().enumerate() {
        let q = qq as i64;
        // skip unusable sites early: g = BIG can never win
        // against any finite parabola — but it CAN win when
        // everything is BIG, so only drop when an envelope
        // already exists. (Dropping all-BIG sites just
        // defers to the BIG default below.)
        if gq >= BIG && v.is_empty() {
            continue;
        }
        let s_num: i128;
        let s_den: i128;
        loop {
            let k = v.len();
            if k == 0 {
                s_num = neg_inf.0;
                s_den = neg_inf.1;
                break;
            }
            let p = v[k - 1];
            let gp = g[p as usize];
            // s = ((gq + q²) − (gp + p²)) / (2(q − p))
            let num = i128::from(gq - gp) + i128::from(q - p) * i128::from(q + p);
            let den = 2i128 * i128::from(q - p);
            // compare s <= z[k] ?  (z[k-1] is start of v[k-1]'s interval)
            let (zn, zd) = z[k - 1];
            if num * zd <= zn * den {
                v.pop(); // dominated
                z.pop();
                continue;
            }
            s_num = num;
            s_den = den;
            break;
        }
        v.push(q);
        z.push((s_num, s_den));
    }
    if v.is_empty() {
        return vec![BIG; n];
    }
    let mut out = vec![0i64; n];
    let mut k = 0usize;
    for (x, o) in out.iter_mut().enumerate() {
        let xi = x as i128;
        // advance while breakpoint z[k+1] <= x
        while k + 1 < v.len() {
            let (zn, zd) = z[k + 1];
            if zn <= xi * zd {
                k += 1;
            } else {
                break;
            }
        }
        let dx = x as i64 - v[k];
        let gv = g[v[k] as usize];
        *o = if gv >= BIG { BIG } else { dx * dx + gv };
    }
    out
}

/// 2-D squared EDT over a boolean grid (`true` = feature).
///
/// Two passes: 1-D EDT down each column (feature sites
/// get `g = 0`, others `BIG`), then 1-D EDT along each row
/// using the column results as parabola heights.
/// `min_{site} (x−sx)² + (y−sy)²` factors exactly.
pub fn edt2(grid: &[Vec<bool>]) -> Vec<Vec<i64>> {
    let h = grid.len();
    if h == 0 {
        return Vec::new();
    }
    let w = grid[0].len();
    // column pass
    let mut cols = Vec::with_capacity(w);
    for x in 0..w {
        let col: Vec<i64> = (0..h)
            .map(|y| {
                if grid[y].get(x).copied().unwrap_or(false) {
                    0
                } else {
                    BIG
                }
            })
            .collect();
        cols.push(edt1(&col));
    }
    let mut out: Vec<Vec<i64>> = (0..h)
        .map(|y| (0..w).map(|x| cols[x][y]).collect())
        .collect();
    // row pass — cells of unequal width read as BIG
    for (y, orow) in out.iter_mut().enumerate() {
        let row = edt1(orow);
        for (x, o) in orow.iter_mut().enumerate() {
            if x < grid[y].len() {
                *o = row[x];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute-force squared EDT for oracle comparison.
    fn brute(grid: &[Vec<bool>]) -> Vec<Vec<i64>> {
        let h = grid.len();
        if h == 0 {
            return Vec::new();
        }
        let w = grid[0].len();
        let mut sites = Vec::new();
        for (y, row) in grid.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                if c {
                    sites.push((x as i64, y as i64));
                }
            }
        }
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| {
                        sites
                            .iter()
                            .map(|&(sx, sy)| (x as i64 - sx).pow(2) + (y as i64 - sy).pow(2))
                            .min()
                            .unwrap_or(BIG)
                    })
                    .collect()
            })
            .collect()
    }

    /// Hand-verified shapes.
    #[test]
    fn basics() {
        // single site at center
        let d = edt2(&[vec![true]]);
        assert_eq!(d[0][0], 0);
        let d2 = edt2(&[
            vec![false, false, false],
            vec![false, true, false],
            vec![false, false, false],
        ]);
        let expected = [[2i64, 1, 2], [1, 0, 1], [2, 1, 2]];
        assert_eq!(
            d2.iter().map(|r| r.as_slice()).collect::<Vec<_>>(),
            expected.iter().map(|r| r.as_slice()).collect::<Vec<_>>()
        );
        // all set → all zero
        let d3 = edt2(&[vec![true, true], vec![true, true]]);
        assert!(d3.iter().flatten().all(|&v| v == 0));
        // none set → all BIG
        let d4 = edt2(&[vec![false, false], vec![false, false]]);
        assert!(d4.iter().flatten().all(|&v| v == BIG));
        // empty
        assert_eq!(edt2(&[]), Vec::<Vec<i64>>::new());
        assert_eq!(edt1(&[]), Vec::<i64>::new());
        assert_eq!(edt1(&[BIG]), vec![BIG]);
        assert_eq!(edt1(&[0, BIG, BIG]), vec![0, 1, 4]);
    }

    /// Brute-force oracle over random small grids.
    #[test]
    fn oracle_bruteforce() {
        let mut rng = SplitMix64::new(0xED7);
        for _ in 0..120 {
            let h = 1 + rng.below(10) as usize;
            let w = 1 + rng.below(10) as usize;
            let grid: Vec<Vec<bool>> = (0..h)
                .map(|_| (0..w).map(|_| rng.below(4) == 0).collect())
                .collect();
            assert_eq!(edt2(&grid), brute(&grid), "{grid:?}");
        }
        // also 1-D oracle
        for _ in 0..120 {
            let n = 1 + rng.below(12) as usize;
            let g: Vec<i64> = (0..n)
                .map(|_| {
                    if rng.below(3) == 0 {
                        rng.below(20) as i64
                    } else {
                        BIG
                    }
                })
                .collect();
            let got = edt1(&g);
            for (x, &gx) in got.iter().enumerate() {
                let want = (0..n)
                    .map(|i| {
                        if g[i] >= BIG {
                            BIG
                        } else {
                            (x as i64 - i as i64).pow(2) + g[i]
                        }
                    })
                    .min()
                    .unwrap_or(BIG);
                assert_eq!(gx, want, "x={x} g={g:?}");
            }
        }
    }

    /// Ragged grids: missing cells read as no feature,
    /// and row width beyond the ragged edge is left BIG.
    #[test]
    fn ragged() {
        let d = edt2(&[vec![true, false, false], vec![false]]);
        assert_eq!(d[0][0], 0);
        assert_eq!(d[1][0], 1);
        // column 2 has no second row — no panic
    }

    /// Determinism.
    #[test]
    fn deterministic() {
        let g = vec![vec![false, true, false], vec![true, false, true]];
        assert_eq!(edt2(&g), edt2(&g));
    }
}

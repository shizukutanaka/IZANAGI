//! Exact integer linear algebra — Bareiss fraction-free Gaussian
//! elimination over `i64` matrices with `i128` intermediates:
//! `det`, `solve` (exact rational answers as reduced `(num, den)`
//! pairs), and `rank`. Because no division ever loses precision,
//! the answers are *exact* — a locked-door constraint solver can
//! trust `solve`'s `None` = provably singular, not "pivot too small".
//!
//! ```
//! use izanagi_kit::gauss::{det, solve};
//! let m = vec![vec![2, 1], vec![1, 3]];
//! assert_eq!(det(&m), Some(5));
//! // 2x + y = 5, x + 3y = 10  →  x = 1, y = 3
//! assert_eq!(solve(&m, &[5, 10]), Some(vec![(1, 1), (3, 1)]));
//! ```

/// Bareiss elimination on a square `i128` copy; returns `None` when
/// the matrix isn't square.
pub fn det(a: &[Vec<i64>]) -> Option<i128> {
    let n = a.len();
    if a.iter().any(|row| row.len() != n) {
        return None;
    }
    if n == 0 {
        return Some(1);
    }
    let mut m: Vec<Vec<i128>> = a
        .iter()
        .map(|r| r.iter().map(|&v| v as i128).collect())
        .collect();
    let (mut sign, mut prev) = (1i128, 1i128);
    for k in 0..n - 1 {
        if m[k][k] == 0 {
            let swap = (k + 1..n).find(|&p| m[p][k] != 0);
            match swap {
                Some(p) => {
                    m.swap(k, p);
                    sign = -sign;
                }
                None => return Some(0),
            }
        }
        let pivot = m[k][k];
        for i in k + 1..n {
            for j in k + 1..n {
                m[i][j] = (m[i][j] * pivot - m[i][k] * m[k][j]) / prev;
            }
            m[i][k] = 0;
        }
        prev = pivot;
    }
    Some(sign * m[n - 1][n - 1])
}

fn gcd_i128(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b > 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a as i128
}

fn reduced(n: i128, d: i128) -> (i128, i128) {
    let g = gcd_i128(n, d).max(1);
    let (n, d) = (n / g, d / g);
    if d < 0 {
        (-n, -d)
    } else {
        (n, d)
    }
}

/// Solve `A x = b` for a square nonsingular `A`; each component is a
/// reduced `(numerator, denominator)`. `None` when `A` is singular or
/// not square / `b` length mismatched.
pub fn solve(a: &[Vec<i64>], b: &[i64]) -> Option<Vec<(i128, i128)>> {
    let n = a.len();
    if a.iter().any(|r| r.len() != n) || b.len() != n {
        return None;
    }
    if n == 0 {
        return Some(Vec::new());
    }
    // Bareiss on the augmented matrix [A|b].
    let mut m: Vec<Vec<i128>> = a
        .iter()
        .enumerate()
        .map(|(i, r)| {
            r.iter()
                .map(|&v| v as i128)
                .chain(std::iter::once(b[i] as i128))
                .collect()
        })
        .collect();
    let mut prev = 1i128;
    for k in 0..n {
        if m[k][k] == 0 {
            let p = (k + 1..n).find(|&p| m[p][k] != 0)?;
            m.swap(k, p);
        }
        let pivot = m[k][k];
        for i in k + 1..n {
            for j in k + 1..=n {
                m[i][j] = (m[i][j] * pivot - m[i][k] * m[k][j]) / prev;
            }
            m[i][k] = 0;
        }
        prev = pivot;
    }
    if m[n - 1][n - 1] == 0 {
        return None;
    }
    // Back-substitution with exact rationals.
    let mut x: Vec<(i128, i128)> = vec![(0, 1); n];
    for i in (0..n).rev() {
        let mut num = m[i][n];
        let mut den = 1i128;
        for j in i + 1..n {
            // num/den -= m[i][j] * x[j]
            let p = m[i][j] * x[j].0;
            num = num * x[j].1 - p * den;
            den *= x[j].1;
            let g = gcd_i128(num, den).max(1);
            num /= g;
            den /= g;
        }
        x[i] = reduced(num, den * m[i][i]);
    }
    Some(x)
}

/// Row rank over the rationals — via exact cross-multiplied
/// elimination (distinct code path from [`det`]'s Bareiss so the two
/// check each other).
pub fn rank(a: &[Vec<i64>]) -> usize {
    let rows = a.len();
    if rows == 0 {
        return 0;
    }
    let cols = a.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut m: Vec<Vec<i128>> = (0..rows)
        .map(|i| {
            let mut r: Vec<i128> = a[i].iter().map(|&v| v as i128).collect();
            r.resize(cols, 0);
            r
        })
        .collect();
    let mut rank = 0;
    let mut col = 0;
    while rank < rows && col < cols {
        let Some(p) = (rank..rows).find(|&i| m[i][col] != 0) else {
            col += 1;
            continue;
        };
        m.swap(rank, p);
        let pivot = m[rank][col];
        for i in rank + 1..rows {
            if m[i][col] == 0 {
                continue;
            }
            let f = m[i][col];
            let (lo, hi) = m.split_at_mut(i);
            for (dst, &pv) in hi[0][col..].iter_mut().zip(lo[rank][col..].iter()) {
                *dst = *dst * pivot - f * pv;
            }
        }
        rank += 1;
        col += 1;
    }
    rank
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perm::{cycles, sign};
    use crate::rng::SplitMix64;

    fn det_brute(a: &[Vec<i64>]) -> i128 {
        let n = a.len();
        let mut total = 0i128;
        // Enumerate all n! permutations by swap-recursion.
        fn go(k: usize, p: &mut Vec<u32>, a: &[Vec<i64>], total: &mut i128) {
            let n = p.len();
            if k == n {
                let mut term = sign(p) as i128;
                for (row, &col) in p.iter().enumerate() {
                    term *= a[row][col as usize] as i128;
                }
                *total += term;
                return;
            }
            for i in k..n {
                p.swap(k, i);
                go(k + 1, p, a, total);
                p.swap(k, i);
            }
        }
        if n == 0 {
            return 1;
        }
        let mut p: Vec<u32> = (0..n as u32).collect();
        go(0, &mut p, a, &mut total);
        total
    }

    /// Independent rank oracle: rational-pair Gaussian elimination.
    fn rank_oracle(a: &[Vec<i64>]) -> usize {
        let rows = a.len();
        if rows == 0 {
            return 0;
        }
        let cols = a.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut m: Vec<Vec<(i128, i128)>> = (0..rows)
            .map(|i| {
                let mut r: Vec<(i128, i128)> = a[i].iter().map(|&v| (v as i128, 1)).collect();
                r.resize(cols, (0, 1));
                r
            })
            .collect();
        let mut rk = 0;
        for c in 0..cols {
            let Some(p) = (rk..rows).find(|&i| m[i][c].0 != 0) else {
                continue;
            };
            m.swap(rk, p);
            for i in rk + 1..rows {
                if m[i][c].0 == 0 {
                    continue;
                }
                // m[i][*] -= m[i][c]/m[rk][c] * m[rk][*]
                let f = reduced(m[i][c].0 * m[rk][c].1, m[i][c].1 * m[rk][c].0);
                let prow = m[rk].clone();
                for (dst, &src) in m[i].iter_mut().zip(prow.iter()) {
                    let t = reduced(src.0 * f.0, src.1 * f.1);
                    let num = dst.0 * t.1 - t.0 * dst.1;
                    *dst = reduced(num, dst.1 * t.1);
                }
            }
            rk += 1;
            if rk == rows {
                break;
            }
        }
        rk
    }

    #[test]
    fn det_matches_permutation_expansion() {
        let mut rng = SplitMix64::new(0x6A055);
        for _ in 0..300 {
            let n = (rng.below(5) + 1) as usize;
            let m: Vec<Vec<i64>> = (0..n)
                .map(|_| (0..n).map(|_| (rng.next_u64() % 21) as i64 - 10).collect())
                .collect();
            assert_eq!(det(&m), Some(det_brute(&m)));
            assert_eq!(rank(&m), rank_oracle(&m));
            // det is a homomorphism-ish sanity: det = 0 ⟺ rank < n.
            assert_eq!(det(&m) == Some(0), rank(&m) < n);
        }
        assert_eq!(det(&[]), Some(1));
        assert_eq!(det(&[vec![1, 2], vec![3]]), None);
    }

    #[test]
    fn solve_is_exact_and_verified() {
        let mut rng = SplitMix64::new(0x5017E);
        for _ in 0..300 {
            let n = (rng.below(4) + 1) as usize;
            let a: Vec<Vec<i64>> = (0..n)
                .map(|_| (0..n).map(|_| (rng.next_u64() % 13) as i64 - 6).collect())
                .collect();
            let b: Vec<i64> = (0..n).map(|_| (rng.next_u64() % 9) as i64).collect();
            match solve(&a, &b) {
                None => assert_eq!(det(&a), Some(0)),
                Some(x) => {
                    // A·x == b as exact rationals.
                    for i in 0..n {
                        let mut num = 0i128;
                        let mut den = 1i128;
                        for j in 0..n {
                            let t = reduced(a[i][j] as i128 * x[j].0, x[j].1);
                            num = num * t.1 + t.0 * den;
                            den *= t.1;
                            let g = gcd_i128(num, den).max(1);
                            num /= g;
                            den /= g;
                        }
                        assert_eq!(reduced(num, den), (b[i] as i128, 1));
                    }
                }
            }
        }
        // Singular system → None.
        assert_eq!(solve(&[vec![1, 2], vec![2, 4]], &[3, 6]), None);
    }

    #[test]
    fn perm_helpers_exercise() {
        // cycles/sign used by det_brute — also guards the oracle itself.
        assert_eq!(sign(&[1, 2, 0]), 1);
        assert_eq!(cycles(&[1, 2, 0]), vec![vec![0u32, 1, 2]]);
    }
}

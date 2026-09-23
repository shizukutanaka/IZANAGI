//! Linear algebra over a prime field `GF(p)` — Gaussian
//! elimination giving `solve`, `rank`, and a nullspace basis.
//! Integer-only, exact, and deterministic (row pivots chosen by
//! first-nonzero scan), complementing [`crate::xorbasis`]
//! (GF(2) span tests) and [`crate::ntheory::mod_pow`].
//!
//! ```
//! use izanagi_kit::modlin;
//! // 2x + y = 5, x + 3y = 5 (mod 7) → x=2, y=1
//! let sol = modlin::solve(&[&[2, 1], &[1, 3]], &[5, 5], 7);
//! assert_eq!(sol, Some(vec![2, 1]));
//! ```

/// `a·b mod p` without overflow worry — inputs are already
/// reduced mod p so `a·b ≤ (p−1)²`; for `p < 2^63` the product
/// fits `u128`.
fn mul_mod(a: u64, b: u64, p: u64) -> u64 {
    ((a as u128 * b as u128) % p as u128) as u64
}

fn pow_mod(mut a: u64, mut e: u64, p: u64) -> u64 {
    let mut r = 1u64 % p;
    a %= p;
    while e > 0 {
        if e & 1 == 1 {
            r = mul_mod(r, a, p);
        }
        a = mul_mod(a, a, p);
        e >>= 1;
    }
    r
}

fn inv_mod(a: u64, p: u64) -> Option<u64> {
    if a % p == 0 {
        return None;
    }
    Some(pow_mod(a % p, p - 2, p))
}

/// Row-reduce `a` (m×n, row-major) to reduced row echelon form
/// mod `p`. Returns `(rref, pivots)` where `pivots[i]` is the
/// column of the i-th pivot row.
///
/// ```
/// use izanagi_kit::modlin;
/// let (e, pivots) = modlin::rref(&[&[2, 1], &[1, 3]], 7);
/// assert_eq!(pivots, vec![0, 1]);
/// assert_eq!(e, vec![vec![1, 0], vec![0, 1]]);
/// ```
pub fn rref(a: &[&[u64]], p: u64) -> (Vec<Vec<u64>>, Vec<usize>) {
    let m = a.len();
    let n = if m == 0 { 0 } else { a[0].len() };
    let mut a: Vec<Vec<u64>> = a
        .iter()
        .map(|r| r.iter().map(|&x| x % p).collect())
        .collect();
    let mut pivots = Vec::new();
    let mut row = 0usize;
    let mut col = 0usize;
    while row < m && col < n {
        // find first row ≥ `row` with nonzero in `col`
        let mut piv = !0usize;
        for (i, r) in a.iter().enumerate().skip(row) {
            if r[col] % p != 0 {
                piv = i;
                break;
            }
        }
        if piv == !0usize {
            col += 1;
            continue;
        }
        a.swap(row, piv);
        let inv = inv_mod(a[row][col], p);
        let inv = match inv {
            Some(v) => v,
            None => {
                col += 1;
                continue;
            }
        };
        for v in a[row].iter_mut() {
            *v = mul_mod(*v, inv, p);
        }
        let row_vals = a[row].clone();
        for (i, a_i) in a.iter_mut().enumerate() {
            if i != row && a_i[col] != 0 {
                let f = a_i[col];
                for (v, &r) in a_i.iter_mut().zip(row_vals.iter()) {
                    let sub = mul_mod(f, r, p);
                    *v = (*v + p - sub % p) % p;
                }
            }
        }
        pivots.push(col);
        row += 1;
        col += 1;
    }
    (a, pivots)
}

/// Rank of `a` mod `p` — the pivot count of [`rref`].
pub fn rank(a: &[&[u64]], p: u64) -> usize {
    rref(a, p).1.len()
}

/// Solve `A·x = b` over `GF(p)`. `a` is m rows × n cols.
/// `Some(x)` gives one solution (free variables = 0); `None`
/// when inconsistent.
pub fn solve(a: &[&[u64]], b: &[u64], p: u64) -> Option<Vec<u64>> {
    let m = a.len();
    let n = if m == 0 { 0 } else { a[0].len() };
    if m != b.len() {
        return None;
    }
    let aug: Vec<Vec<u64>> = a
        .iter()
        .zip(b.iter())
        .map(|(r, &bi)| r.iter().chain([bi].iter()).map(|&x| x % p).collect())
        .collect();
    let refs: Vec<&[u64]> = aug.iter().map(|r| r.as_slice()).collect();
    let (e, pivots) = rref(&refs, p);
    // inconsistency: a zero row with nonzero rhs
    for r in &e {
        let all_zero = r[..n].iter().all(|&x| x % p == 0);
        if all_zero && r[n] % p != 0 {
            return None;
        }
    }
    let mut x = vec![0u64; n];
    for (i, &c) in pivots.iter().enumerate() {
        if c < n {
            x[c] = e[i][n] % p;
        }
    }
    Some(x)
}

/// Nullspace basis of `A` mod `p` — columns `v` with `A·v = 0`.
/// Returned as one vector per free variable (pivot positions
/// carry the negated RREF coefficient).
pub fn nullspace(a: &[&[u64]], p: u64) -> Vec<Vec<u64>> {
    let n = if a.is_empty() { 0 } else { a[0].len() };
    let (e, pivots) = rref(a, p);
    let mut is_pivot = vec![false; n];
    for &c in &pivots {
        if c < n {
            is_pivot[c] = true;
        }
    }
    let mut basis = Vec::new();
    for f in 0..n {
        if is_pivot[f] {
            continue;
        }
        let mut v = vec![0u64; n];
        v[f] = 1;
        for (i, &c) in pivots.iter().enumerate() {
            if c < n {
                v[c] = (p - e[i][f] % p) % p;
            }
        }
        basis.push(v);
    }
    basis
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn eval_row(row: &[u64], x: &[u64], p: u64) -> u64 {
        row.iter().zip(x.iter()).fold(0u64, |s, (&a, &v)| {
            (s + (a as u128 * v as u128) as u64 % p) % p
        })
    }

    fn brute_rank(a: &[&[u64]], p: u64) -> usize {
        let n = a[0].len();
        // count distinct row-span size via GF(p) span enumeration for tiny dims
        let m = a.len();
        let mut span = std::collections::BTreeSet::new();
        span.insert(vec![0u64; n]);
        for r in a.iter() {
            let cur: Vec<Vec<u64>> = span.iter().cloned().collect();
            for c in 1..p.min(8) {
                for v in &cur {
                    let nv: Vec<u64> = v
                        .iter()
                        .zip(r.iter())
                        .map(|(&x, &y)| (x + c * y) % p)
                        .collect();
                    span.insert(nv);
                }
            }
            if span.len() > 1 << 16 || m > 6 {
                return usize::MAX; // too big — skip oracle
            }
        }
        let mut k = 0usize;
        let mut sz = span.len() as u64;
        while sz > 1 {
            sz /= p;
            k += 1;
        }
        k
    }

    #[test]
    fn solve_small_systems() {
        let (e, pivots) = rref(&[&[2, 1], &[1, 3]], 7);
        assert_eq!(pivots, vec![0, 1]);
        assert_eq!(e, vec![vec![1, 0], vec![0, 1]]);
        let sol = solve(&[&[2, 1], &[1, 3]], &[5, 5], 7);
        assert_eq!(sol, Some(vec![2, 1]));
        // underdetermined: x + y = 1 mod 5 → x=1,y=0 (free=0 convention)
        let sol = solve(&[&[1, 1]], &[1], 5);
        assert_eq!(sol, Some(vec![1, 0]));
        // inconsistent
        assert_eq!(solve(&[&[1, 1], &[1, 1]], &[1, 2], 5), None);
        // singular square
        assert_eq!(solve(&[&[2, 2], &[4, 4]], &[1, 1], 5), None);
        let sol = solve(&[&[2, 2], &[4, 4]], &[1, 2], 5);
        assert!(sol.is_some());
        let x = sol.unwrap();
        assert_eq!(eval_row(&[2, 2], &x, 5), 1);
        assert_eq!(eval_row(&[4, 4], &x, 5), 2);
    }

    #[test]
    fn solve_oracle_random() {
        let mut rng = SplitMix64::new(11);
        for _ in 0..300 {
            let p = [5u64, 7, 11, 97][rng.below(4) as usize];
            let m = (rng.below(4) + 1) as usize;
            let n = (rng.below(4) + 1) as usize;
            let a: Vec<Vec<u64>> = (0..m)
                .map(|_| (0..n).map(|_| rng.below(p as u32) as u64).collect())
                .collect();
            let b: Vec<u64> = (0..m).map(|_| rng.below(p as u32) as u64).collect();
            let refs: Vec<&[u64]> = a.iter().map(|r| r.as_slice()).collect();
            if let Some(x) = solve(&refs, &b, p) {
                for (i, r) in a.iter().enumerate() {
                    assert_eq!(eval_row(r, &x, p), b[i] % p);
                }
            } else {
                // brute: no x satisfies all rows (p tiny → enumerate)
                let mut any = false;
                let total = p.pow(n as u32);
                if total <= 200_000 {
                    'outer: for mask in 0..total {
                        let mut x = vec![0u64; n];
                        let mut t = mask;
                        for xi in x.iter_mut() {
                            *xi = t % p;
                            t /= p;
                        }
                        if a.iter()
                            .enumerate()
                            .all(|(i, r)| eval_row(r, &x, p) == b[i] % p)
                        {
                            any = true;
                            break 'outer;
                        }
                    }
                    assert!(!any);
                }
            }
        }
    }

    #[test]
    fn rank_and_nullspace_oracle() {
        let mut rng = SplitMix64::new(12);
        for _ in 0..200 {
            let p = [3u64, 5][rng.below(2) as usize];
            let m = (rng.below(3) + 1) as usize;
            let n = (rng.below(3) + 1) as usize;
            let a: Vec<Vec<u64>> = (0..m)
                .map(|_| (0..n).map(|_| rng.below(p as u32) as u64).collect())
                .collect();
            let refs: Vec<&[u64]> = a.iter().map(|r| r.as_slice()).collect();
            let r = rank(&refs, p);
            let want = brute_rank(&refs, p);
            if want != usize::MAX {
                assert_eq!(r, want);
            }
            // nullity = n - rank
            let ns = nullspace(&refs, p);
            assert_eq!(ns.len(), n - r);
            for v in &ns {
                for row in &a {
                    assert_eq!(eval_row(row, v, p), 0);
                }
            }
            // basis independence: basis-matrix rank equals its size
            let brefs: Vec<&[u64]> = ns.iter().map(|v| v.as_slice()).collect();
            if !brefs.is_empty() {
                assert_eq!(rank(&brefs, p), ns.len());
            }
        }
    }

    #[test]
    fn prime_power_free_var() {
        // A = [1 1; 0 0] mod 7: rank 1, nullity 1, null vector (p-1,1)∝(-1,1)
        let ns = nullspace(&[&[1, 1], &[0, 0]], 7);
        assert_eq!(ns.len(), 1);
        assert_eq!(eval_row(&[1, 1], &ns[0], 7), 0);
        assert!(ns[0][1] == 1);
    }
}

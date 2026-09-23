//! Exact Gram–Schmidt orthogonalization and QR decomposition
//! over [`crate::frac::Frac`] — rational linear algebra with
//! no rounding anywhere.
//!
//! Columns of `A` (row-major `m × n`) are orthogonalized in
//! order: `qⱼ = aⱼ − Σₖ<ⱼ (⟨aⱼ,qₖ⟩/⟨qₖ,qₖ⟩)·qₖ`. The `q` are
//! *orthogonal but unnormalized* — exact `ℚ` cannot carry
//! `√`-normalization — so `Q` satisfies `QᵀQ = diag(‖qⱼ‖²)`,
//! and `R` is unit-diagonal upper triangular with
//! `A = Q·R` holding **exactly**. A linearly dependent column
//! yields the zero vector for its `q` (documented, not an
//! error) and `R`'s row still satisfies the product.
//!
//! ```
//! use izanagi_kit::frac::Frac;
//! use izanagi_kit::ortho::qr;
//! // 3×2: columns (1,0,1) and (1,1,0)
//! let a = vec![
//!     vec![Frac::from_int(1), Frac::from_int(1)],
//!     vec![Frac::from_int(0), Frac::from_int(1)],
//!     vec![Frac::from_int(1), Frac::from_int(0)],
//! ];
//! let (q, r) = qr(&a).unwrap();
//! assert_eq!(r[0][0], Frac::from_int(1));
//! // q₂ = a₂ − (⟨a₂,q₁⟩/⟨q₁,q₁⟩)·q₁ = (1/2, 1, −1/2)
//! assert_eq!(q[0][1] * Frac::from_int(2), Frac::from_int(1));
//! ```
//!
//! References: Golub & Van Loan, *Matrix Computations* §5.2;
//! classical Gram–Schmidt over `ℚ`.

use crate::frac::Frac;

fn dot(a: &[Frac], b: &[Frac]) -> Frac {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| x * y)
        .fold(Frac::from_int(0), |s, t| s + t)
}

/// Row-major matrix of exact rationals.
pub type Mat = Vec<Vec<Frac>>;

/// QR decompose `a` (row-major `m × n`, `m ≥ 1`) into
/// `(q, r)`: `q` is row-major `m × n` with pairwise-orthogonal
/// columns, `r` is row-major `n × n` unit-diagonal upper
/// triangular, and `a == q · r` exactly.
///
/// `None` when `a` is empty, ragged, or has `n == 0`.
/// Dependent columns are allowed: their `q` column comes back
/// as the zero vector.
pub fn qr(a: &[Vec<Frac>]) -> Option<(Mat, Mat)> {
    let m = a.len();
    let n = a.first()?.len();
    if n == 0 || a.iter().any(|row| row.len() != n) {
        return None;
    }
    let col = |j: usize| -> Vec<Frac> { (0..m).map(|i| a[i][j]).collect() };
    let mut q = vec![vec![Frac::from_int(0); n]; m];
    let mut r = vec![vec![Frac::from_int(0); n]; n];
    let qcol = |k: usize, q: &Vec<Vec<Frac>>| -> Vec<Frac> { (0..m).map(|i| q[i][k]).collect() };
    for j in 0..n {
        let aj = col(j);
        let mut v = aj.clone();
        for k in 0..j {
            let qk = qcol(k, &q);
            let denom = dot(&qk, &qk);
            let c = if denom == Frac::from_int(0) {
                // qₖ is the zero vector (dependent column) —
                // nothing to project against.
                Frac::from_int(0)
            } else {
                match dot(&aj, &qk).checked_div(denom) {
                    Some(c) => c,
                    None => Frac::from_int(0),
                }
            };
            r[k][j] = c;
            for i in 0..m {
                v[i] = v[i] - c * q[i][k];
            }
        }
        r[j][j] = Frac::from_int(1);
        for i in 0..m {
            q[i][j] = v[i];
        }
    }
    Some((q, r))
}

/// Rank proxy: number of nonzero orthogonal columns in `q`
/// (from [`qr`]) — equals `rank(A)` exactly over `ℚ`.
pub fn rank(q: &[Vec<Frac>]) -> usize {
    if q.is_empty() {
        return 0;
    }
    let n = q[0].len();
    (0..n)
        .filter(|&j| q.iter().any(|row| row[j] != Frac::from_int(0)))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn matmul(a: &[Vec<Frac>], b: &[Vec<Frac>]) -> Vec<Vec<Frac>> {
        let (m, n, p) = (a.len(), b[0].len(), b.len());
        let mut out = vec![vec![Frac::from_int(0); n]; m];
        for i in 0..m {
            for j in 0..n {
                for k in 0..p {
                    out[i][j] = out[i][j] + a[i][k] * b[k][j];
                }
            }
        }
        out
    }

    fn coldot(q: &[Vec<Frac>], i: usize, j: usize) -> Frac {
        dot(
            &q.iter().map(|r| r[i]).collect::<Vec<_>>(),
            &q.iter().map(|r| r[j]).collect::<Vec<_>>(),
        )
    }

    #[test]
    fn basics() {
        let a = vec![
            vec![Frac::from_int(1), Frac::from_int(1)],
            vec![Frac::from_int(0), Frac::from_int(1)],
            vec![Frac::from_int(1), Frac::from_int(0)],
        ];
        let (q, r) = qr(&a).unwrap();
        assert_eq!(matmul(&q, &r), a);
        assert_eq!(coldot(&q, 0, 1), Frac::from_int(0));
        assert!(qr(&[]).is_none());
        assert!(qr(&[vec![]]).is_none());
        // ragged input rejected
        assert!(qr(&[vec![Frac::from_int(1)], vec![]]).is_none());
    }

    /// The defining exact properties on random inputs:
    /// `A = QR`, `R` unit upper-triangular, `q` columns
    /// pairwise orthogonal, `rank` matches the truth.
    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0xA44);
        for _ in 0..150 {
            let m = 1 + rng.below(5) as usize;
            let n = 1 + rng.below(4) as usize;
            let mut a: Vec<Vec<Frac>> = (0..m)
                .map(|_| {
                    (0..n)
                        .map(|_| {
                            Frac::new(i128::from(rng.below(11)) - 5, 1 + i128::from(rng.below(4)))
                        })
                        .collect()
                })
                .collect();
            // inject a dependent column half the time
            if n > 1 && rng.coin(1, 2) {
                for row in a.iter_mut() {
                    row[n - 1] = row[0] * Frac::from_int(2);
                }
            }
            let (q, r) = qr(&a).unwrap();
            assert_eq!(matmul(&q, &r), a);
            for (i, ri) in r.iter().enumerate().take(n) {
                assert_eq!(ri[i], Frac::from_int(1));
                for (j, rj) in r.iter().enumerate().take(n).skip(i + 1) {
                    assert_eq!(rj[i], Frac::from_int(0)); // lower = 0
                    assert_eq!(coldot(&q, i, j), Frac::from_int(0));
                }
            }
            // brute rank oracle: Gaussian elimination over Frac
            let mut mat = a.clone();
            let mut rk = 0usize;
            for j in 0..n {
                if let Some(p) = (rk..m).find(|&i| mat[i][j] != Frac::from_int(0)) {
                    mat.swap(rk, p);
                    let piv = mat[rk][j];
                    let (top, bot) = mat.split_at_mut(rk + 1);
                    for row in bot.iter_mut() {
                        let f = row[j].checked_div(piv).unwrap();
                        for (v, &pv) in row.iter_mut().zip(top[rk].iter()).skip(j) {
                            *v = *v - f * pv;
                        }
                    }
                    rk += 1;
                }
            }
            assert_eq!(rank(&q), rk);
        }
    }
}

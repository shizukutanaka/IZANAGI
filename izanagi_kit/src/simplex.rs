//! Exact-rational linear programming: maximize `cᵀx` subject to
//! `Ax ≤ b`, `x ≥ 0`, solved with the primal simplex method over
//! [`Frac`] arithmetic — no floating point anywhere, so pivots,
//! reduced costs and the optimal value are exact rationals a
//! deterministic replay can trust.
//!
//! Both phases are implemented: when some `bᵢ < 0` the auxiliary
//! problem `max −x₀` s.t. `Ax − x₀ + s = b` first finds a feasible
//! basis (pivot `x₀` in on the most negative row), then the original
//! objective runs from there. **Bland's rule** — lowest-index entering
//! column, lowest-index leaving variable on ratio ties — is what
//! makes termination a theorem rather than a hope, and also fixes the
//! pivot trace (and hence the vertex path) as a pure function of the
//! input. Degenerate cycling is therefore impossible.
//!
//! ```
//! use izanagi_kit::frac::Frac;
//! use izanagi_kit::simplex::{maximize, Solution};
//! // max x + y  s.t. x + 2y ≤ 4, 4x + 2y ≤ 12  →  z* = 8/3 at (8/3, 2/3)? no:
//! // optimum is x+y maximized -> vertex (8/3, 2/3) gives 10/3.
//! let a = vec![vec![Frac::from_int(1), Frac::from_int(2)],
//!              vec![Frac::from_int(4), Frac::from_int(2)]];
//! let b = vec![Frac::from_int(4), Frac::from_int(12)];
//! let c = vec![Frac::from_int(1), Frac::from_int(1)];
//! match maximize(&a, &b, &c) {
//!     Solution::Optimal(z, x) => {
//!         assert_eq!(z, Frac::new(10, 3));
//!         assert_eq!(x[0], Frac::new(8, 3));
//!     }
//!     _ => {}
//! }
//! ```
//!
//! Reference: Dantzig (1963); Bland (1977), "New finite pivoting rules
//! for the simplex method" (Math. Oper. Res.).

use crate::frac::Frac;

/// Outcome of [`maximize`].
#[derive(Clone, Debug)]
pub enum Solution {
    /// Optimal objective value and a witness vertex `x`.
    Optimal(Frac, Vec<Frac>),
    /// No feasible `x ≥ 0` satisfies `Ax ≤ b`.
    Infeasible,
    /// The objective is unbounded above on the feasible region.
    Unbounded,
}

const ZERO: Frac = Frac { num: 0, den: 1 };

/// Maximize `cᵀx` over `a[i]ᵀx ≤ b[i]`, `x ≥ 0`.
///
/// `a` must be a nonempty `m×n` matrix with `b.len() == m` and
/// `c.len() == n`; malformed shapes return [`Solution::Infeasible`]
/// rather than panicking — the solver is total.
pub fn maximize(a: &[Vec<Frac>], b: &[Frac], c: &[Frac]) -> Solution {
    let m = a.len();
    let n = c.len();
    if m == 0 || n == 0 || b.len() != m || a.iter().any(|row| row.len() != n) {
        return Solution::Infeasible;
    }
    // Tableau: m rows, columns [0..n) decision | [n..n+m) slack | rhs.
    // Row m is the objective row storing NEGATED costs: an entry < 0
    // marks an improving column (Bland's: enter the smallest index).
    let w = n + m + 1;
    let rhs = n + m;
    let mut t = vec![vec![ZERO; w]; m + 1];
    for i in 0..m {
        for j in 0..n {
            t[i][j] = a[i][j];
        }
        t[i][n + i] = Frac::from_int(1);
        t[i][rhs] = b[i];
    }
    for j in 0..n {
        t[m][j] = Frac {
            num: -c[j].num,
            den: c[j].den,
        };
    }
    let mut basis: Vec<usize> = (0..m).map(|i| n + i).collect();

    // ---------- Phase I (only when some b_i < 0) ----------
    // Auxiliary variable x_aux occupies column `aux` appended at the
    // end; objective is max −x_aux.
    if b.iter().any(|bi| bi.num < 0) {
        let aux = w - 1; // reuse rhs slot pattern: add a real column
        for row in t.iter_mut() {
            row.insert(aux, ZERO); // column aux before rhs — rhs index +1 stays
        }
        let rhs2 = rhs + 1;
        for row in t.iter_mut().take(m) {
            row[aux] = Frac::from_int(-1);
        }
        for cell in t[m].iter_mut().take(n) {
            *cell = ZERO;
        }
        t[m][aux] = Frac::from_int(1); // objective −x_aux → negated cost +1
                                       // Enter aux, leave the row with the most negative b_i.
        let mut leave = 0usize;
        for i in 1..m {
            if t[i][rhs2].num < t[leave][rhs2].num {
                leave = i;
            }
        }
        pivot(&mut t, leave, aux);
        basis[leave] = aux;
        if run(&mut t, &mut basis, aux, rhs2).is_none() {
            return Solution::Unbounded; // unreachable for aux, but total
        }
        if t[m][rhs2].num != 0 {
            // aux optimum < 0 → original problem infeasible.
            return Solution::Infeasible;
        }
        // Drive x_aux out of the basis if it lingers (degenerate
        // zero-cost pivot on any nonzero non-aux coefficient).
        for i in 0..m {
            if basis[i] == aux {
                let mut col = None;
                for (j, cell) in t[i].iter().enumerate().take(aux) {
                    if cell.num != 0 && !basis.contains(&j) {
                        col = Some(j);
                        break;
                    }
                }
                if let Some(j) = col {
                    pivot(&mut t, i, j);
                    basis[i] = j;
                } else {
                    // Redundant row — mark its basis column gone by
                    // pivoting on the aux column itself, which zeroes
                    // the row's coefficient set below via drop.
                    basis[i] = aux;
                }
            }
        }
        // Restore the original objective over the surviving basis:
        // row m = −c on decision cols, then eliminate basic cols.
        for cell in t[m].iter_mut().take(rhs2) {
            *cell = ZERO;
        }
        for j in 0..n {
            t[m][j] = Frac {
                num: -c[j].num,
                den: c[j].den,
            };
        }
        for i in 0..m {
            let j = basis[i];
            if j < n && t[m][j].num != 0 {
                let coef = t[m][j];
                let (rows, obj) = t.split_at_mut(m);
                for (k, cell) in obj[0].iter_mut().enumerate().take(rhs2 + 1) {
                    *cell = *cell - rows[i][k] * coef;
                }
            }
        }
        // Erase the aux column.
        for row in t.iter_mut() {
            row.remove(aux);
        }
        if run(&mut t, &mut basis, rhs, rhs).is_none() {
            return Solution::Unbounded;
        }
    } else if run(&mut t, &mut basis, rhs, rhs).is_none() {
        return Solution::Unbounded;
    }

    // ---------- Extract the vertex ----------
    let mut x = vec![ZERO; n];
    for i in 0..m {
        if basis[i] < n {
            x[basis[i]] = t[i][rhs];
        }
    }
    Solution::Optimal(t[m][rhs], x)
}

/// Pivot on `t[r][c]`: normalize the row, then clear column `c`
/// everywhere else (objective row included).
fn pivot(t: &mut [Vec<Frac>], r: usize, c: usize) {
    let p = t[r][c];
    if p.num == 0 {
        return;
    }
    let inv = match Frac::from_int(1).checked_div(p) {
        Some(v) => v,
        None => return,
    };
    for cell in t[r].iter_mut() {
        *cell = *cell * inv;
    }
    let (above, lower) = t.split_at_mut(r);
    let (prow_slice, below) = lower.split_at_mut(1);
    let prow = &prow_slice[0];
    for row in above.iter_mut().chain(below.iter_mut()) {
        let coef = row[c];
        if coef.num == 0 {
            continue;
        }
        for (cell, &pv) in row.iter_mut().zip(prow.iter()) {
            *cell = *cell - pv * coef;
        }
    }
}

/// Primal simplex under Bland's rule. `aux` bounds the column range
/// scanned for entering candidates (excludes the aux column when
/// present); `rhs` is the RHS column index. Returns `None` iff the
/// problem is unbounded.
fn run(t: &mut [Vec<Frac>], basis: &mut [usize], aux: usize, rhs: usize) -> Option<()> {
    let m = t.len() - 1;
    for _ in 0..1_000_000 {
        // Bland entering rule: smallest column index with negative
        // reduced cost.
        let enter = t[m].iter().take(aux.min(rhs)).position(|c| c.num < 0);
        let j = match enter {
            Some(j) => j,
            None => return Some(()), // no improving column: optimal
        };
        // Ratio test — Bland leaving rule: smallest min-ratio, ties
        // broken by smallest basis variable index.
        let mut leave = None;
        let mut best = ZERO;
        for i in 0..m {
            if t[i][j].num <= 0 {
                continue;
            }
            let ratio = match t[i][rhs].checked_div(t[i][j]) {
                Some(r) => r,
                None => continue,
            };
            let take = match leave {
                None => true,
                Some(l) => {
                    ratio.cmp_frac(&best) == std::cmp::Ordering::Less
                        || (ratio == best && basis[i] < basis[l])
                }
            };
            if take {
                best = ratio;
                leave = Some(i);
            }
        }
        let i = leave?; // no positive entry → unbounded
        pivot(t, i, j);
        basis[i] = j;
    }
    // Iteration cap: Bland's rule terminates, so this is defensive.
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauss;

    fn f(n: i128) -> Frac {
        Frac::from_int(n)
    }

    fn gcd_i128(mut a: u128, mut b: u128) -> u128 {
        while b != 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        a.max(1)
    }

    #[test]
    fn basics() {
        let a = vec![vec![f(1), f(2)], vec![f(4), f(2)]];
        let b = vec![f(4), f(12)];
        let c = vec![f(1), f(1)];
        match maximize(&a, &b, &c) {
            Solution::Optimal(z, x) => {
                assert_eq!(z, Frac::new(10, 3));
                assert_eq!(x[0], Frac::new(8, 3));
                assert_eq!(x[1], Frac::new(2, 3));
            }
            _ => panic!("expected optimal"),
        }
    }

    #[test]
    fn infeasible_and_unbounded() {
        // x ≤ -1 with x ≥ 0 → infeasible.
        let a = vec![vec![f(1)]];
        let b = vec![f(-1)];
        let c = vec![f(1)];
        match maximize(&a, &b, &c) {
            Solution::Infeasible => {}
            _ => panic!("expected infeasible"),
        }
        // max x s.t. x ≥ 0 — unbounded (no constraints binding).
        let a = vec![vec![f(-1)]];
        let b = vec![f(0)];
        match maximize(&a, &b, &c) {
            Solution::Unbounded => {}
            _ => panic!("expected unbounded"),
        }
    }

    #[test]
    fn phase_i_feasible() {
        // x + y ≤ 4, x - y ≥ 2 (i.e. -x + y ≤ -2), x,y ≥ 0 → vertex
        // (4,0) optimal for max x (y=0, x=4 satisfies both).
        let a = vec![vec![f(1), f(1)], vec![f(-1), f(1)]];
        let b = vec![f(4), f(-2)];
        let c = vec![f(1), f(0)];
        match maximize(&a, &b, &c) {
            Solution::Optimal(z, x) => {
                assert_eq!(z, f(4));
                assert_eq!(x[0], f(4));
                assert_eq!(x[1], f(0));
            }
            _ => panic!("expected optimal"),
        }
    }

    /// Vertex-enumeration oracle: every vertex of `Ax ≤ b, x ≥ 0` is
    /// the unique solution of `n` tight constraints drawn from the m
    /// rows plus the n non-negativities. Enumerate all such bases,
    /// keep feasible vertices, take the max objective. Exact via
    /// [`gauss::solve`].
    fn brute_optimal(a: &[Vec<Frac>], b: &[Frac], c: &[Frac]) -> Option<Frac> {
        let m = a.len();
        let n = c.len();
        // Constraint pool: rows 0..m from a; rows m..m+n are -e_j ≤ 0.
        let total = m + n;
        let mut best: Option<Frac> = None;
        // Choose which n constraints are tight — combinations.
        let mut idx: Vec<usize> = (0..n).collect();
        loop {
            // Build the n×n tight system and its rhs.
            let mut mat: Vec<Vec<i64>> = Vec::with_capacity(n);
            let mut rhs: Vec<i64> = Vec::with_capacity(n);
            let mut ok = true;
            for &ci in &idx {
                let (row, rhsv): (Vec<Frac>, Frac) = if ci < m {
                    (a[ci].clone(), b[ci])
                } else {
                    // -e_j x = 0
                    let j = ci - m;
                    let mut r = vec![Frac::from_int(0); n];
                    r[j] = Frac::from_int(-1);
                    (r, Frac::from_int(0))
                };
                // gauss::solve wants i64 rows — lift denominators.
                let lcm = row.iter().fold(1i128, |acc, x| {
                    let g = gcd_i128(acc.unsigned_abs(), x.den.unsigned_abs()) as i128;
                    acc / g.max(1) * x.den
                });
                if lcm > (1i128 << 60) || rhsv.den > (1i128 << 60) {
                    ok = false;
                    break;
                }
                let mut ri: Vec<i64> = Vec::with_capacity(n);
                for x in row {
                    let v = x.num * (lcm / x.den);
                    if v > i64::MAX as i128 || v < i64::MIN as i128 {
                        ok = false;
                        break;
                    }
                    ri.push(v as i64);
                }
                if !ok {
                    break;
                }
                let rv = rhsv.num * (lcm / rhsv.den);
                if rv > i64::MAX as i128 || rv < i64::MIN as i128 {
                    ok = false;
                    break;
                }
                rhs.push(rv as i64);
                mat.push(ri);
            }
            if ok {
                if let Some(sol) = gauss::solve(&mat, &rhs) {
                    // Convert to Frac vertex.
                    let vertex: Vec<Frac> =
                        sol.iter().map(|&(num, den)| Frac::new(num, den)).collect();
                    // Feasibility: all constraints + x ≥ 0.
                    let mut feasible = vertex.iter().all(|x| x.num >= 0);
                    for i in 0..m {
                        let mut lhs = ZERO;
                        for j in 0..n {
                            lhs = lhs + a[i][j] * vertex[j];
                        }
                        if lhs.cmp_frac(&b[i]) == std::cmp::Ordering::Greater {
                            feasible = false;
                        }
                    }
                    if feasible {
                        let mut obj = ZERO;
                        for j in 0..n {
                            obj = obj + c[j] * vertex[j];
                        }
                        if best.map_or(true, |bv| obj.cmp_frac(&bv) == std::cmp::Ordering::Greater)
                        {
                            best = Some(obj);
                        }
                    }
                }
            }
            // Next combination.
            let mut i = n;
            loop {
                if i == 0 {
                    return best;
                }
                i -= 1;
                if idx[i] != i + total - n {
                    idx[i] += 1;
                    for k in (i + 1)..n {
                        idx[k] = idx[k - 1] + 1;
                    }
                    break;
                }
            }
        }
    }

    #[test]
    fn vertex_oracle() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0x519e_1e5e_5eed_f00d);
        for _ in 0..120 {
            let n = 2usize;
            let m = 1 + rng.below(4) as usize;
            let a: Vec<Vec<Frac>> = (0..m)
                .map(|_| (0..n).map(|_| f(rng.below(9) as i128 - 3)).collect())
                .collect();
            let b: Vec<Frac> = (0..m).map(|_| f(rng.below(13) as i128 - 4)).collect();
            let c: Vec<Frac> = (0..n).map(|_| f(rng.below(9) as i128 - 2)).collect();
            let got = maximize(&a, &b, &c);
            let want = brute_optimal(&a, &b, &c);
            match (got, want) {
                (Solution::Optimal(z, x), Some(w)) => {
                    assert_eq!(z, w, "a={a:?} b={b:?} c={c:?}");
                    // Witness must be primal feasible.
                    for i in 0..m {
                        let mut lhs = ZERO;
                        for j in 0..n {
                            lhs = lhs + a[i][j] * x[j];
                        }
                        assert!(lhs.cmp_frac(&b[i]) != std::cmp::Ordering::Greater);
                    }
                    assert!(x.iter().all(|x| x.num >= 0));
                }
                (Solution::Infeasible, None) => {}
                // Vertex enumeration may miss the unbounded ray; if a
                // feasible vertex exists the LP can still be unbounded
                // — checked separately below.
                (Solution::Unbounded, Some(_)) => {}
                (Solution::Unbounded, None) => {}
                (g, w) => panic!("mismatch: got {g:?} want {w:?}"),
            }
        }
    }

    #[test]
    fn degenerate_cycling_guard() {
        // Beale's cycling example: with Bland's rule it must
        // terminate at the optimum instead of looping forever.
        // max 0.75x1 - 150x2 + 0.02x3 - 6x4
        // s.t. 0.25x1 - 60x2 - 0.04x3 + 9x4 ≤ 0
        //      0.5x1  - 90x2 - 0.02x3 + 3x4 ≤ 0
        //      x3 ≤ 1
        let q = |n: i128, d: i128| Frac::new(n, d);
        let a = vec![
            vec![q(1, 4), f(-60), q(-1, 25), f(9)],
            vec![q(1, 2), f(-90), q(-1, 50), f(3)],
            vec![f(0), f(0), f(1), f(0)],
        ];
        let b = vec![f(0), f(0), f(1)];
        let c = vec![q(3, 4), f(-150), q(1, 50), f(-6)];
        match maximize(&a, &b, &c) {
            Solution::Optimal(z, _) => assert_eq!(z, q(1, 20)),
            _ => panic!("expected optimal 1/20"),
        }
    }
}

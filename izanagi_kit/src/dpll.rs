//! DPLL SAT solver — Davis–Putnam–Logemann–Loveland backtracking
//! search over CNF formulas.
//!
//! Input is a formula in conjunctive normal form: a slice of clauses,
//! each clause a `Vec` of non-zero literals where literal `i` means
//! "variable `i`" and `−i` means "not variable `i`" (variables are
//! 1-indexed). [`solve`] returns a satisfying assignment as
//! `Some(model)` with `model[v − 1]` = value of variable `v`, or
//! `None` when the formula is unsatisfiable.
//!
//! The search is the classical DPLL loop — unit propagation and pure
//! literal elimination to a fixpoint, then a deterministic split on
//! the smallest remaining variable, `true` branch first. Returned
//! models are canonical: unset variables default to `false`, so
//! `solve` is a pure function of the input formula (same clauses in,
//! same model out — a property replay machinery can rely on).
//!
//! Not a CDCL solver — no clause learning, no VSIDS. For game-scale
//! problems (puzzle rules, placement constraints, desync forensics
//! over bounded formulas) plain DPLL is exact and fast enough.
//!
//! ```
//! use izanagi_kit::dpll::solve;
//! // (x1 ∨ x2) ∧ (¬x1 ∨ x2) ∧ (x1 ∨ ¬x2)
//! let model = solve(&[vec![1, 2], vec![-1, 2], vec![1, -2]]).unwrap();
//! assert!(model.iter().all(|&v| v));
//! assert_eq!(solve(&[vec![1], vec![-1]]), None); // contradiction
//! ```

use std::collections::BTreeSet;

/// Remove clauses satisfied by `lit` and delete `−lit` from the rest.
/// `None` when a clause is reduced to empty (conflict).
fn simplify(f: &[Vec<i32>], lit: i32) -> Option<Vec<Vec<i32>>> {
    let mut out = Vec::with_capacity(f.len());
    for c in f {
        if c.contains(&lit) {
            continue;
        }
        let nc: Vec<i32> = c.iter().copied().filter(|&l| l != -lit).collect();
        if nc.is_empty() {
            return None;
        }
        out.push(nc);
    }
    Some(out)
}

/// DPLL recursion over `f`; `assign[v]` records the forced value of
/// variable `v` (index 0 unused). Returns `true` on satisfiable.
fn dpll(f: &[Vec<i32>], assign: &mut Vec<Option<bool>>) -> bool {
    // Unit propagation — repeat until no unit clause remains.
    let mut f = f.to_vec();
    loop {
        let unit = f.iter().find_map(|c| (c.len() == 1).then_some(c[0]));
        let Some(lit) = unit else { break };
        let v = lit.unsigned_abs() as usize;
        match assign[v] {
            Some(a) if a != (lit > 0) => return false, // contradicts a prior force
            _ => assign[v] = Some(lit > 0),
        }
        match simplify(&f, lit) {
            Some(nf) => f = nf,
            None => return false,
        }
    }
    // Pure literal elimination: a variable appearing in only one
    // polarity can be fixed without branching.
    let mut pos = BTreeSet::new();
    let mut neg = BTreeSet::new();
    for c in &f {
        for &l in c {
            if l > 0 {
                pos.insert(l as usize);
            } else {
                neg.insert((-l) as usize);
            }
        }
    }
    for v in 1..assign.len() {
        let (pure, phase) = (pos.contains(&v) && !neg.contains(&v))
            .then_some(true)
            .or_else(|| (neg.contains(&v) && !pos.contains(&v)).then_some(false))
            .map(|p| (true, p))
            .unwrap_or((false, false));
        if pure {
            let lit = if phase { v as i32 } else { -(v as i32) };
            assign[v] = Some(phase);
            match simplify(&f, lit) {
                Some(nf) => return dpll(&nf, assign),
                None => return false,
            }
        }
    }
    // Empty formula: every clause satisfied.
    if f.is_empty() {
        return true;
    }
    // Split on the smallest variable still present; `true` first.
    let v = f
        .iter()
        .flat_map(|c| c.iter())
        .map(|l| l.unsigned_abs() as usize)
        .min();
    let Some(v) = v else { return true }; // non-empty formula always has a literal
    for phase in [true, false] {
        let mut branch = assign.clone();
        branch[v] = Some(phase);
        let lit = if phase { v as i32 } else { -(v as i32) };
        if let Some(nf) = simplify(&f, lit) {
            if dpll(&nf, &mut branch) {
                *assign = branch;
                return true;
            }
        }
    }
    false
}

/// Solve a CNF formula — `Some(model)` when satisfiable, `None` on a
/// proof of unsatisfiability.
///
/// `model` has one entry per variable appearing in `f` (length =
/// largest variable index); variables never mentioned take `false`.
/// A malformed literal `0` makes the clause impossible to satisfy and
/// the whole formula unsatisfiable (`None`).
pub fn solve(f: &[Vec<i32>]) -> Option<Vec<bool>> {
    if f.iter().any(|c| c.contains(&0)) {
        return None;
    }
    let max_var = f
        .iter()
        .flat_map(|c| c.iter())
        .map(|l| l.unsigned_abs() as usize)
        .max()
        .unwrap_or(0);
    if f.iter().any(|c| c.is_empty()) {
        return None; // an empty clause is unsatisfiable on its own
    }
    let mut assign = vec![None; max_var + 1];
    if !dpll(f, &mut assign) {
        return None;
    }
    Some(assign.iter().skip(1).map(|a| a.unwrap_or(false)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Every clause holds a literal the model makes true.
    fn satisfied(f: &[Vec<i32>], model: &[bool]) -> bool {
        f.iter().all(|c| {
            c.iter().any(|&l| {
                let v = l.unsigned_abs() as usize;
                model.get(v - 1).copied().unwrap_or(false) == (l > 0)
            })
        })
    }

    /// Brute-force oracle: enumerate all 2^n assignments.
    fn brute(n: usize, f: &[Vec<i32>]) -> Option<Vec<bool>> {
        for bits in 0..(1usize << n) {
            let model: Vec<bool> = (0..n).map(|i| (bits >> i) & 1 == 1).collect();
            if satisfied(f, &model) {
                return Some(model);
            }
        }
        None
    }

    #[test]
    fn models_verify_and_match_brute_force() {
        let mut rng = SplitMix64::new(0xD911);
        for _ in 0..400 {
            let vars = (rng.below(7) + 1) as usize;
            let m = rng.below(10) as usize;
            let f: Vec<Vec<i32>> = (0..m)
                .map(|_| {
                    let len = (rng.below(3) + 1) as usize;
                    (0..len)
                        .map(|_| {
                            let v = (rng.below(vars as u32) + 1) as i32;
                            if rng.below(2) == 0 {
                                -v
                            } else {
                                v
                            }
                        })
                        .collect()
                })
                .collect();
            let got = solve(&f);
            let want = brute(vars, &f);
            assert_eq!(
                got.is_some(),
                want.is_some(),
                "satisfiability mismatch for {f:?}"
            );
            if let Some(model) = &got {
                assert!(
                    satisfied(&f, model),
                    "returned model does not satisfy {f:?}"
                );
            }
        }
    }

    #[test]
    fn unit_propagation_forces_chain() {
        // x1, then x1→x2, then x2→x3: all forced true.
        let m = solve(&[vec![1], vec![-1, 2], vec![-2, 3]]).unwrap();
        assert_eq!(m, vec![true, true, true]);
        // Pure literal: x2 appears only negatively → forced false.
        let m = solve(&[vec![1, -2], vec![3, -2]]).unwrap();
        assert!(!m[1]);
    }

    #[test]
    fn unsat_and_edge_cases() {
        assert_eq!(solve(&[vec![1], vec![-1]]), None);
        assert_eq!(solve(&[vec![1, 2], vec![-1], vec![-2]]), None);
        assert_eq!(solve(&[vec![]]), None); // empty clause
        assert_eq!(solve(&[]), Some(vec![])); // empty formula is trivially sat
        assert_eq!(solve(&[vec![1, -1]]), Some(vec![true])); // tautology
        assert_eq!(solve(&[vec![0]]), None); // literal 0 is malformed
    }

    #[test]
    fn canonical_model_is_deterministic() {
        // Same formula always yields the identical model.
        let f = vec![vec![-1, 2], vec![1, 3], vec![-3, -2]];
        assert_eq!(solve(&f), solve(&f));
        // Variables absent from the formula default to false.
        let m = solve(&[vec![3]]).unwrap();
        assert_eq!(m, vec![false, false, true]);
    }
}

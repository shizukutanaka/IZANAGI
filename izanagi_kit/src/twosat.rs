//! 2-SAT via the implication graph — Every clause `a ∨ b` is the pair
//! of implications `¬a → b`, `¬b → a`; a formula is satisfiable iff no
//! variable shares a strongly-connected component with its own
//! negation (Aspvall–Plass–Tarjan 1979). Reuses
//! [`graph::strongly_connected`](crate::graph), which returns
//! components sinks-first — assignment `x = rank[x] < rank[¬x]`
//! satisfies every clause (verified against brute force in tests).
//!
//! Fits quest/tech-tree gating, key-and-lock placement constraints,
//! and "at most one of each pair" puzzle rules — anywhere the kit's
//! hand-rolled constraint checks used to live.
//!
//! ```
//! use izanagi_kit::twosat::{Lit, TwoSat};
//! let mut s = TwoSat::new(2);
//! s.add_clause(Lit::pos(0), Lit::pos(1)); // x0 ∨ x1
//! s.add_clause(Lit::neg(0), Lit::pos(1)); // ¬x0 ∨ x1
//! let sol = s.solve().unwrap_or_default();
//! assert!(sol[1]); // x1 forced
//! ```

use crate::graph::strongly_connected;

/// A literal: variable `v` with a polarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lit(pub u32, pub bool);

impl Lit {
    /// `v` is true.
    pub fn pos(v: u32) -> Self {
        Self(v, true)
    }

    /// `v` is false.
    pub fn neg(v: u32) -> Self {
        Self(v, false)
    }

    /// The opposite literal.
    pub fn negated(self) -> Self {
        Self(self.0, !self.1)
    }

    /// Implication-graph node: `var*2 + (is positive)`.
    fn node(self) -> usize {
        self.0 as usize * 2 + self.1 as usize
    }
}

/// A 2-SAT instance: `n` variables plus binary clauses.
pub struct TwoSat {
    n: usize,
    clauses: Vec<(Lit, Lit)>,
}

impl TwoSat {
    /// Instance over variables `0..nvars`.
    pub fn new(nvars: usize) -> Self {
        Self {
            n: nvars,
            clauses: Vec::new(),
        }
    }

    /// Number of variables.
    pub fn nvars(&self) -> usize {
        self.n
    }

    /// Clause `a ∨ b`.
    pub fn add_clause(&mut self, a: Lit, b: Lit) {
        self.clauses.push((a, b));
    }

    /// Unit clause — forces `l`.
    pub fn add_unit(&mut self, l: Lit) {
        self.clauses.push((l, l));
    }

    /// `a → b` — i.e. `¬a ∨ b`.
    pub fn add_implies(&mut self, a: Lit, b: Lit) {
        self.clauses.push((a.negated(), b));
    }

    /// `a ↔ b`.
    pub fn add_equiv(&mut self, a: Lit, b: Lit) {
        self.add_implies(a, b);
        self.add_implies(b, a);
    }

    /// `a XOR b` — differ.
    pub fn add_xor(&mut self, a: Lit, b: Lit) {
        self.add_clause(a, b);
        self.add_clause(a.negated(), b.negated());
    }

    /// Solve. `Some(assign)` when satisfiable — a *valid* assignment
    /// (not arbitrary: the sinks-first SCC rule picks a canonical one),
    /// `None` when some variable's two literals collide in an SCC.
    /// Clauses referencing variables `≥ nvars` extend the instance —
    /// the returned vector covers `0..max(nvars, max_var+1)`.
    pub fn solve(&self) -> Option<Vec<bool>> {
        let mut nv = self.n;
        for &(a, b) in &self.clauses {
            nv = nv.max(a.0 as usize + 1).max(b.0 as usize + 1);
        }
        let mut adj = vec![Vec::new(); nv * 2];
        for &(a, b) in &self.clauses {
            adj[a.negated().node()].push(b.node() as u32);
            adj[b.negated().node()].push(a.node() as u32);
        }
        let comps = strongly_connected(&adj);
        let mut rank = vec![0usize; nv * 2];
        for (i, c) in comps.iter().enumerate() {
            for &v in c {
                rank[v as usize] = i;
            }
        }
        let mut out = vec![false; nv];
        for (v, slot) in out.iter_mut().enumerate() {
            let (t, f) = (v * 2 + 1, v * 2);
            if rank[t] == rank[f] {
                return None;
            }
            // Components arrive sinks-first (reverse topo): a literal
            // is true when its SCC sits *earlier* in that order than
            // its negation's — i.e. closer to a sink.
            *slot = rank[t] < rank[f];
        }
        Some(out)
    }

    /// Whether `assign` satisfies every clause — exposed so callers
    /// can validate hand-rolled or alternative assignments.
    pub fn check(&self, assign: &[bool]) -> bool {
        let lit = |l: Lit| -> bool { assign.get(l.0 as usize).copied().unwrap_or(false) == l.1 };
        self.clauses.iter().all(|&(a, b)| lit(a) || lit(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn brute(n: usize, clauses: &[(Lit, Lit)]) -> bool {
        for mask in 0..(1usize << n) {
            let ok = clauses
                .iter()
                .all(|&(a, b)| ((mask >> a.0) & 1 == 1) == a.1 || ((mask >> b.0) & 1 == 1) == b.1);
            if ok {
                return true;
            }
        }
        false
    }

    #[test]
    fn solve_matches_brute_force_and_check() {
        let mut rng = SplitMix64::new(0x5A77);
        for _ in 0..400 {
            let n = rng.below(7) as usize;
            let mut s = TwoSat::new(n);
            let m = rng.below(14) as usize;
            let mut nv = n;
            for _ in 0..m {
                let a = Lit(rng.below(n.max(1) as u32), rng.next_u64() & 1 == 1);
                let b = Lit(rng.below(n.max(1) as u32), rng.next_u64() & 1 == 1);
                nv = nv.max(a.0 as usize + 1).max(b.0 as usize + 1);
                s.add_clause(a, b);
            }
            match s.solve() {
                Some(assign) => {
                    assert!(brute(nv, &s.clauses), "solver SAT but brute UNSAT");
                    assert!(s.check(&assign), "solver's own assignment invalid");
                }
                None => assert!(!brute(nv, &s.clauses), "solver UNSAT but brute SAT"),
            }
        }
    }

    #[test]
    fn known_unsat_and_forced() {
        // (x0 ∨ x1) ∧ (x0 ∨ ¬x1) ∧ (¬x0 ∨ x1) ∧ (¬x0 ∨ ¬x1) — XOR+NXOR.
        let mut s = TwoSat::new(2);
        s.add_clause(Lit::pos(0), Lit::pos(1));
        s.add_clause(Lit::pos(0), Lit::neg(1));
        s.add_clause(Lit::neg(0), Lit::pos(1));
        s.add_clause(Lit::neg(0), Lit::neg(1));
        assert_eq!(s.solve(), None);
        // Chain implications force x0 → x1 → x2, then ¬x2 → x0 keeps it free.
        let mut s = TwoSat::new(3);
        s.add_implies(Lit::pos(0), Lit::pos(1));
        s.add_implies(Lit::pos(1), Lit::pos(2));
        s.add_unit(Lit::pos(0));
        let sol = s.solve().unwrap_or_default();
        assert!(sol[0] && sol[1] && sol[2]);
        assert_eq!(s.nvars(), 3);
        // equiv + xor on the same pair is UNSAT.
        let mut s = TwoSat::new(2);
        s.add_equiv(Lit::pos(0), Lit::pos(1));
        s.add_xor(Lit::pos(0), Lit::pos(1));
        assert_eq!(s.solve(), None);
        // Empty instance over n vars trivially satisfies.
        assert_eq!(TwoSat::new(4).solve(), Some(vec![false; 4]));
    }
}

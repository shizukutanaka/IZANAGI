//! Reduced ordered binary decision diagrams (ROBDDs) — canonical
//! boolean functions via hash-consing and the Shannon `apply`.
//!
//! Nodes are arena indices into a `Vec`; terminals are `0` (false)
//! and `1` (true). The unique table keys `(var, lo, hi)`, so structurally
//! identical functions share a node id — with a fixed variable order,
//! `and(a, b) == and(b, a)` is a *pointer* equality, which is exactly the
//! canonicality theorem the tests exploit. All state is internal
//! `BTreeMap`s — no hashers, no ambient randomness — so node ids are a
//! pure function of construction order.
//!
//! ```
//! use izanagi_kit::bdd::Bdd;
//! let mut b = Bdd::new();
//! let x0 = b.var(0);
//! let x1 = b.var(1);
//! let f = b.and(x0, x1);
//! assert!(b.eval(f, 0b11)); // x0=1, x1=1
//! assert!(!b.eval(f, 0b10));
//! assert_eq!(b.count_sat(f, 2), 1);
//! ```
//!
//! Reference: Bryant (1986), "Graph-based algorithms for Boolean
//! function manipulation" (IEEE TC).

/// Node id of the `false` terminal.
pub const FALSE: u32 = 0;
/// Node id of the `true` terminal.
pub const TRUE: u32 = 1;

/// A ROBDD manager (unique table + apply cache + node arena).
pub struct Bdd {
    /// `nodes[i]` — for `i >= 2`, `(var, lo, hi)`; terminals occupy 0/1.
    nodes: Vec<Node>,
    unique: std::collections::BTreeMap<(u32, u32, u32), u32>,
    cache: std::collections::BTreeMap<(Op, u32, u32), u32>,
}

#[derive(Clone, Copy)]
struct Node {
    var: u32,
    lo: u32,
    hi: u32,
}

/// Boolean operator for [`Bdd::apply`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    /// `a AND b`.
    And,
    /// `a OR b`.
    Or,
    /// `a XOR b`.
    Xor,
    /// `a AND (NOT b)`.
    Diff,
    /// `a IMPLIES b` (`!a | b`).
    Implies,
}

impl Op {
    fn eval(self, a: bool, b: bool) -> bool {
        match self {
            Op::And => a & b,
            Op::Or => a | b,
            Op::Xor => a ^ b,
            Op::Diff => a & !b,
            Op::Implies => !a | b,
        }
    }
}

impl Default for Bdd {
    fn default() -> Self {
        Self::new()
    }
}

impl Bdd {
    /// Fresh manager with both terminals interned.
    pub fn new() -> Bdd {
        // Terminals store var = u32::MAX so they sort below every
        // decision node in the "lowest variable" comparison.
        Bdd {
            nodes: vec![
                Node {
                    var: !0,
                    lo: FALSE,
                    hi: FALSE,
                },
                Node {
                    var: !0,
                    lo: TRUE,
                    hi: TRUE,
                },
            ],
            unique: std::collections::BTreeMap::new(),
            cache: std::collections::BTreeMap::new(),
        }
    }

    /// Number of decision nodes (terminals excluded).
    pub fn node_count(&self) -> usize {
        self.nodes.len() - 2
    }

    /// Intern `(var, lo, hi)`; returns `lo` unchanged when `lo == hi`
    /// (the reduction rule that makes ROBDDs canonical).
    pub fn mk(&mut self, var: u32, lo: u32, hi: u32) -> u32 {
        if lo == hi {
            return lo;
        }
        if let Some(&id) = self.unique.get(&(var, lo, hi)) {
            return id;
        }
        let id = self.nodes.len() as u32;
        self.nodes.push(Node { var, lo, hi });
        self.unique.insert((var, lo, hi), id);
        id
    }

    /// The literal function `x_var`.
    pub fn var(&mut self, var: u32) -> u32 {
        self.mk(var, FALSE, TRUE)
    }

    /// `NOT f` — pushes complementation through the Shannon tree.
    pub fn not(&mut self, f: u32) -> u32 {
        self.apply(Op::Diff, TRUE, f)
    }

    /// `a AND b`.
    pub fn and(&mut self, a: u32, b: u32) -> u32 {
        self.apply(Op::And, a, b)
    }

    /// `a OR b`.
    pub fn or(&mut self, a: u32, b: u32) -> u32 {
        self.apply(Op::Or, a, b)
    }

    /// `a XOR b`.
    pub fn xor(&mut self, a: u32, b: u32) -> u32 {
        self.apply(Op::Xor, a, b)
    }

    /// Binary boolean operation via Shannon expansion with a global
    /// `(op, a, b)` memo — the classic `apply` of Bryant (1986).
    pub fn apply(&mut self, op: Op, a: u32, b: u32) -> u32 {
        if a <= TRUE && b <= TRUE {
            return if op.eval(a == TRUE, b == TRUE) {
                TRUE
            } else {
                FALSE
            };
        }
        if let Some(&hit) = self.cache.get(&(op, a, b)) {
            return hit;
        }
        let na = self.nodes[a as usize];
        let nb = self.nodes[b as usize];
        // Decision variable: lowest index present at either top.
        let (va, vb) = (na.var, nb.var);
        let var = va.min(vb);
        let (a_lo, a_hi) = if va == var { (na.lo, na.hi) } else { (a, a) };
        let (b_lo, b_hi) = if vb == var { (nb.lo, nb.hi) } else { (b, b) };
        let lo = self.apply(op, a_lo, b_lo);
        let hi = self.apply(op, a_hi, b_hi);
        let out = self.mk(var, lo, hi);
        self.cache.insert((op, a, b), out);
        out
    }

    /// Cofactor `f` by `var = value` (Shannon restrict).
    pub fn restrict(&mut self, f: u32, var: u32, value: bool) -> u32 {
        let n = self.nodes[f as usize];
        if n.var > var {
            // Ordered: `var` is not tested anywhere below.
            return f;
        }
        if n.var == var {
            return if value { n.hi } else { n.lo };
        }
        let lo = self.restrict(n.lo, var, value);
        let hi = self.restrict(n.hi, var, value);
        self.mk(n.var, lo, hi)
    }

    /// Existential quantification `exists var. f`.
    pub fn exists(&mut self, f: u32, var: u32) -> u32 {
        let lo = self.restrict(f, var, false);
        let hi = self.restrict(f, var, true);
        self.apply(Op::Or, lo, hi)
    }

    /// Evaluate under an assignment — bit `i` of `assign` is `x_i`.
    /// Variables beyond bit 63 read as `false`.
    pub fn eval(&self, f: u32, assign: u64) -> bool {
        let mut n = f;
        while n > TRUE {
            let node = self.nodes[n as usize];
            let bit = if node.var < 64 {
                (assign >> node.var) & 1 == 1
            } else {
                false
            };
            n = if bit { node.hi } else { node.lo };
        }
        n == TRUE
    }

    /// Number of satisfying assignments over `nvars` variables —
    /// exact `u128` count, so output is order- and structure-free.
    pub fn count_sat(&self, f: u32, nvars: u32) -> u128 {
        // Memo-free exponential walk is fine for verified sizes; each
        // node skips levels, contributing 2^(gap) paths.
        fn walk(bdd: &Bdd, n: u32, level: u32, nvars: u32) -> u128 {
            if n == FALSE {
                return 0;
            }
            if n == TRUE {
                return 1u128 << nvars.saturating_sub(level);
            }
            let node = bdd.nodes[n as usize];
            let skip = 1u128 << (node.var - level).min(nvars);
            let lo = walk(bdd, node.lo, node.var + 1, nvars);
            let hi = walk(bdd, node.hi, node.var + 1, nvars);
            skip * (lo + hi)
        }
        walk(self, f, 0, nvars)
    }

    /// Top variable of a decision node (`None` on terminals).
    pub fn top_var(&self, f: u32) -> Option<u32> {
        if f > TRUE {
            Some(self.nodes[f as usize].var)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Truth-table oracle over `nvars` variables — every pub surface
    /// of `Bdd` is checked against direct boolean evaluation.
    fn truth_oracle(b: &mut Bdd, f: u32, oracle: impl Fn(u64) -> bool, nvars: u32) {
        let mut want_count = 0u128;
        for a in 0..(1u64 << nvars) {
            let want = oracle(a);
            assert_eq!(b.eval(f, a), want, "assign {a:06b}");
            want_count += want as u128;
        }
        assert_eq!(b.count_sat(f, nvars), want_count);
    }

    #[test]
    fn vars_and_constants() {
        let mut b = Bdd::new();
        assert!(!b.eval(FALSE, 0));
        assert!(b.eval(TRUE, 0));
        let x0 = b.var(0);
        let x0_again = b.var(0);
        assert_eq!(x0, x0_again); // unique table interning
        truth_oracle(&mut b, x0, |a| a & 1 == 1, 3);
        assert_eq!(b.top_var(x0), Some(0));
        assert_eq!(b.top_var(TRUE), None);
    }

    #[test]
    fn ops_match_truth_tables() {
        let mut b = Bdd::new();
        let x0 = b.var(0);
        let x1 = b.var(1);
        let x2 = b.var(2);
        let x1_or_x2 = b.or(x1, x2);
        let f = b.and(x0, x1_or_x2);
        truth_oracle(
            &mut b,
            f,
            |a| (a & 1 == 1) && ((a >> 1 & 1 == 1) || (a >> 2 & 1 == 1)),
            3,
        );
        let x0_or_x1 = b.or(x0, x1);
        let g = b.apply(Op::Diff, x0_or_x1, x2);
        truth_oracle(
            &mut b,
            g,
            |a| ((a & 1 == 1) || (a >> 1 & 1 == 1)) && (a >> 2 & 1 != 1),
            3,
        );
        let h = b.apply(Op::Implies, x0, x1);
        truth_oracle(&mut b, h, |a| (a & 1 != 1) || (a >> 1 & 1 == 1), 3);
    }

    #[test]
    fn canonicity() {
        // Same function, different construction order -> same node id.
        let mut b = Bdd::new();
        let x0 = b.var(0);
        let x1 = b.var(1);
        let f1 = b.and(x0, x1);
        let f2 = b.and(x1, x0);
        assert_eq!(f1, f2);
        let t = b.and(x0, x1);
        let nx0 = b.not(x0);
        let g1 = b.or(t, nx0);
        let nx0b = b.not(x0);
        let g2 = b.or(nx0b, x1);
        truth_oracle(
            &mut b,
            g1,
            |a| {
                let (x0, x1) = (a & 1 == 1, a >> 1 & 1 == 1);
                // g1 is built as (x0 & x1) | !x0, which reduces to !x0 | x1 —
                // the very equivalence canonicity asserts, so the oracle states
                // the reduced form.
                !x0 || x1
            },
            2,
        );
        truth_oracle(
            &mut b,
            g2,
            |a| {
                let (x0, x1) = (a & 1 == 1, a >> 1 & 1 == 1);
                !x0 || x1
            },
            2,
        );
        // (x0∧x1) ∨ ¬x0 ≡ ¬x0 ∨ x1 — canonical ids must agree.
        assert_eq!(g1, g2);
    }

    #[test]
    fn restrict_and_exists() {
        let mut b = Bdd::new();
        let x0 = b.var(0);
        let x1 = b.var(1);
        let f = b.and(x0, x1);
        let r0 = b.restrict(f, 0, false);
        assert_eq!(r0, FALSE);
        let r1 = b.restrict(f, 0, true);
        assert_eq!(r1, x1);
        // exists x0. (x0 ∧ x1) = x1
        let e = b.exists(f, 0);
        truth_oracle(&mut b, e, |a| a >> 1 & 1 == 1, 2);
        // exists x1. (x0 ⊕ x1) = true
        let g = b.xor(x0, x1);
        let e2 = b.exists(g, 1);
        assert_eq!(e2, TRUE);
    }

    #[test]
    fn random_formula_oracle() {
        // Random composed formulas vs direct evaluation, 200 cases.
        let mut rng = SplitMix64::new(0xdead_beef_cafe_f00d);
        for _ in 0..200 {
            let mut b = Bdd::new();
            let x0 = b.var(0);
            let x1 = b.var(1);
            let x2 = b.var(2);
            let x3 = b.var(3);
            let vars = [x0, x1, x2, x3];
            // Two-level random composition: (¬?)x_li OP (¬?)x_ri —
            // leaf index and polarity tracked separately so the
            // truth-table oracle mirrors the BDD exactly.
            let (li, ri) = (rng.below(4), rng.below(4));
            let l = vars[li as usize];
            let r = vars[ri as usize];
            let (lneg, rneg) = (rng.below(2) == 0, rng.below(2) == 0);
            let l2 = if lneg { b.not(l) } else { l };
            let r2 = if rneg { b.not(r) } else { r };
            let op = match rng.below(3) {
                0 => Op::And,
                1 => Op::Or,
                _ => Op::Xor,
            };
            let f = b.apply(op, l2, r2);
            let lo = move |a: u64| (a >> li) & 1 == 1;
            let ro = move |a: u64| (a >> ri) & 1 == 1;
            truth_oracle(
                &mut b,
                f,
                move |a| {
                    let (x, y) = (lo(a) ^ lneg, ro(a) ^ rneg);
                    match op {
                        Op::And => x && y,
                        Op::Or => x || y,
                        Op::Xor => x ^ y,
                        _ => false,
                    }
                },
                4,
            );
        }
    }
}

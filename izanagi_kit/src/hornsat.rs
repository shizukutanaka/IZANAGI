//! Linear-time Horn satisfiability — the fragment of SAT where
//! every clause has at most one positive literal
//! (`a ∧ b ∧ … → p`, plus pure-goal clauses `¬a ∨ ¬b ∨ …`).
//! Dowling–Gallier: watch each variable's negative-literal
//! occurrences, count down each clause's un-satisfied negatives
//! as variables become true, and force a head literal the
//! moment its body is fully met — the result is the *least*
//! model (variables true exactly when forced) or `None` when a
//! goal clause is refuted.
//!
//! Horn formulas are the workhorse behind rule systems,
//! forward-chaining inference, and type-class resolution —
//! this is a one-shot batch solver, not an incremental engine.
//!
//! ```
//! use izanagi_kit::hornsat::{solve, Clause};
//! // a → b, b → c, a — forces a, b, c
//! let clauses = vec![
//!     Clause::imp(&[0], 1),
//!     Clause::imp(&[1], 2),
//!     Clause::fact(0),
//! ];
//! assert_eq!(solve(3, &clauses), Some(vec![true, true, true]));
//! // goal clause ¬a∨¬b is refuted by both true
//! let bad = vec![Clause::fact(0), Clause::fact(1), Clause::goal(&[0, 1])];
//! assert_eq!(solve(2, &bad), None);
//! ```
//!
//! References: Dowling & Gallier (1984) "Linear-time algorithms
//! for testing the satisfiability of propositional Horn
//! formulae"; Itai & Makowsky (1987) least-model uniqueness.

/// A Horn clause: `neg[0] ∧ neg[1] ∧ … → pos` (an implication
/// with a possibly absent head). `pos = None` is a *goal*
/// clause — it forbids all of `neg` holding simultaneously.
/// `neg` empty with `pos = Some(p)` is a fact. Both empty is a
/// logical contradiction.
#[derive(Clone, Debug)]
pub struct Clause {
    /// Optional positive literal (the head). `None` = goal clause.
    pub pos: Option<u32>,
    /// Negative literals (the body).
    pub neg: Vec<u32>,
}

impl Clause {
    /// `body[0] ∧ body[1] ∧ … → head`.
    pub fn imp(body: &[u32], head: u32) -> Clause {
        Clause {
            pos: Some(head),
            neg: body.to_vec(),
        }
    }

    /// Assert `v` unconditionally.
    pub fn fact(v: u32) -> Clause {
        Clause {
            pos: Some(v),
            neg: Vec::new(),
        }
    }

    /// Forbid all of `body` holding at once.
    pub fn goal(body: &[u32]) -> Clause {
        Clause {
            pos: None,
            neg: body.to_vec(),
        }
    }
}

/// Least-model satisfying assignment for a Horn formula over
/// `n_vars` variables, or `None` if unsatisfiable.
///
/// Runs in `O(total literals)`: each variable is enqueued at
/// most once, and each negative literal is decremented at most
/// once. Variables a satisfying assignment could leave either
/// way are reported `false` — the unique minimal model.
pub fn solve(n_vars: usize, clauses: &[Clause]) -> Option<Vec<bool>> {
    // watch[v] = clauses containing v as a negative literal
    let mut watch: Vec<Vec<u32>> = vec![Vec::new(); n_vars];
    let mut remaining = vec![0u32; clauses.len()];
    for (c, cl) in clauses.iter().enumerate() {
        remaining[c] = cl.neg.len() as u32;
        for &v in &cl.neg {
            watch[v as usize].push(c as u32);
        }
    }
    let mut assign = vec![false; n_vars];
    // seed the queue with facts and the empty clause check
    let mut queue: Vec<u32> = Vec::new();
    for (c, cl) in clauses.iter().enumerate() {
        if remaining[c] == 0 {
            queue.push(cl.pos?);
        }
    }
    let mut qi = 0usize;
    while qi < queue.len() {
        let v = queue[qi];
        qi += 1;
        if assign[v as usize] {
            continue;
        }
        assign[v as usize] = true;
        for &wc in &watch[v as usize] {
            let c = wc as usize;
            remaining[c] -= 1;
            if remaining[c] == 0 {
                match clauses[c].pos {
                    Some(p) if !assign[p as usize] => queue.push(p),
                    Some(_) => {}
                    None => return None,
                }
            }
        }
    }
    Some(assign)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn satisfied(assign: &[bool], clauses: &[Clause]) -> bool {
        clauses.iter().all(|cl| {
            let body = cl.neg.iter().all(|&v| assign[v as usize]);
            match cl.pos {
                Some(p) => !body || assign[p as usize],
                None => !body,
            }
        })
    }

    /// 2^n oracle: exists an assignment satisfying every clause.
    fn brute(n: usize, clauses: &[Clause]) -> bool {
        let total = 1u64 << n;
        for mask in 0..total {
            let assign: Vec<bool> = (0..n).map(|i| mask >> i & 1 == 1).collect();
            if satisfied(&assign, clauses) {
                return true;
            }
        }
        false
    }

    #[test]
    fn basics() {
        // facts + implications chain
        let c = vec![Clause::fact(0), Clause::imp(&[0], 1), Clause::imp(&[1], 2)];
        assert_eq!(solve(3, &c), Some(vec![true, true, true]));
        // goal refuted
        let bad = vec![Clause::fact(0), Clause::fact(1), Clause::goal(&[0, 1])];
        assert_eq!(solve(2, &bad), None);
        // empty clause: immediate contradiction
        let empty = vec![Clause {
            pos: None,
            neg: Vec::new(),
        }];
        assert_eq!(solve(1, &empty), None);
        // no clauses: everything free → all false
        assert_eq!(solve(4, &[]), Some(vec![false; 4]));
        // unused variable stays false (least model)
        let c = vec![Clause::fact(0)];
        assert_eq!(solve(3, &c), Some(vec![true, false, false]));
    }

    #[test]
    fn oracle_brute_force() {
        let mut rng = SplitMix64::new(41);
        for _ in 0..400 {
            let n = (rng.below(4) + 1) as usize; // ≤ 4 vars → 16 masks
            let nc = rng.below(7) as usize;
            let clauses: Vec<Clause> = (0..nc)
                .map(|_| {
                    let m = rng.below(3) as usize;
                    let neg: Vec<u32> = (0..m).map(|_| rng.below(n as u32)).collect();
                    let pos = if rng.below(4) == 0 {
                        None
                    } else {
                        Some(rng.below(n as u32))
                    };
                    Clause { pos, neg }
                })
                .collect();
            match solve(n, &clauses) {
                Some(assign) => {
                    assert!(brute(n, &clauses), "sat but brute says unsat: {clauses:?}");
                    assert!(
                        satisfied(&assign, &clauses),
                        "returned model violates: {clauses:?}"
                    );
                }
                None => assert!(
                    !brute(n, &clauses),
                    "unsat but brute found a model: {clauses:?}"
                ),
            }
        }
    }

    #[test]
    fn determinism_and_least_model() {
        // repeated runs identical; unforced vars are false
        let c = vec![
            Clause::imp(&[0, 1], 2),
            Clause::fact(0),
            Clause::imp(&[0], 1),
            Clause::imp(&[2], 3),
        ];
        let a = solve(5, &c);
        let b = solve(5, &c);
        assert_eq!(a, b);
        assert_eq!(a, Some(vec![true, true, true, true, false]));
    }
}

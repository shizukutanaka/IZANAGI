//! Finite Markov decision processes — Bellman value iteration and
//! Howard policy iteration over integer-transition tables. This is
//! the control-theoretic sibling of `hmm`/`kalman`: where those
//! infer hidden state, this picks actions that maximise discounted
//! return `Σ γᵗ rₜ`.
//!
//! Transitions are `(next_state, probability, reward)` triples with
//! `Fixed` probabilities and `i64` rewards — evaluated in `Fixed`,
//! so a given table + γ + iteration count replays bit-exactly.
//! Iteration counts are explicit inputs (determinism beats adaptive
//! stopping): value iteration is `V' ← max_a Σ p·(r + γV)`,
//! policy iteration alternates evaluation sweeps and greedy
//! improvement until the policy stabilises or the sweep budget ends.
//!
//! ```
//! use izanagi_kit::mdp::Mdp;
//! use izanagi_kit::fixed::Fixed;
//!
//! // State 0: act 0 → stay (r=0); act 1 → state 1 (r=10).
//! // State 1: absorbing, r=1 per step.
//! let m = Mdp::new(vec![
//!     vec![
//!         vec![(0, Fixed::ONE, 0)],
//!         vec![(1, Fixed::ONE, 10)],
//!     ],
//!     vec![vec![(1, Fixed::ONE, 1)]],
//! ]);
//! let (v, pi) = m.value_iteration(Fixed::from_ratio(9, 10), 200);
//! assert_eq!(pi[0], 1); // take the +10 action
//! assert!(v[0] > v[1]);
//! ```

use crate::fixed::Fixed;

/// One transition: `(next state, probability, reward)`.
pub type Trans = (u32, Fixed, i64);

/// A finite MDP as an action-indexed transition table:
/// `actions[s][a]` is the distribution over `Trans` for taking
/// action `a` in state `s`. Probabilities needn't sum to 1 —
/// shortfall is absorbed into a reward-0 stay.
pub struct Mdp {
    /// `actions[state][action] = Vec<Trans>`.
    pub actions: Vec<Vec<Vec<Trans>>>,
}

impl Mdp {
    /// Wrap a transition table; states with no actions act as
    /// absorbing zero-reward sinks.
    pub fn new(actions: Vec<Vec<Vec<Trans>>>) -> Self {
        Mdp { actions }
    }

    /// Expected one-step value of `action` under `v`.
    fn q(&self, s: usize, a: usize, v: &[Fixed], gamma: Fixed) -> Fixed {
        let mut acc = Fixed::ZERO;
        for &(next, p, r) in &self.actions[s][a] {
            let n = (next as usize).min(v.len().saturating_sub(1));
            let rc = r.clamp(-2_000_000_000, 2_000_000_000) as i32;
            let target = Fixed::from_int(rc) + gamma.mul(v[n]);
            acc = acc + p.mul(target);
        }
        acc
    }

    /// Greedy policy w.r.t. `v`; states without actions get
    /// `u32::MAX`-free sentinel 0 (there is no action to name).
    fn greedy(&self, v: &[Fixed], gamma: Fixed) -> Vec<u32> {
        let mut pi = vec![0u32; v.len()];
        for (s, slot) in pi.iter_mut().enumerate() {
            if self.actions[s].is_empty() {
                continue;
            }
            let mut best = 0usize;
            let mut best_q = self.q(s, 0, v, gamma);
            for a in 1..self.actions[s].len() {
                let q = self.q(s, a, v, gamma);
                if q > best_q {
                    best_q = q;
                    best = a;
                }
            }
            *slot = best as u32;
        }
        pi
    }

    /// Bellman value iteration: `iters` sweeps of
    /// `V(s) ← max_a Q(s,a)` starting from `V = 0`.
    /// Returns the value table and the greedy policy it implies.
    pub fn value_iteration(&self, gamma: Fixed, iters: u32) -> (Vec<Fixed>, Vec<u32>) {
        let n = self.actions.len();
        let mut v = vec![Fixed::ZERO; n];
        for _ in 0..iters {
            let mut next = vec![Fixed::ZERO; n];
            for (s, slot) in next.iter_mut().enumerate() {
                if self.actions[s].is_empty() {
                    continue;
                }
                let mut best_q = self.q(s, 0, &v, gamma);
                for a in 1..self.actions[s].len() {
                    let q = self.q(s, a, &v, gamma);
                    if q > best_q {
                        best_q = q;
                    }
                }
                *slot = best_q;
            }
            v = next;
        }
        let pi = self.greedy(&v, gamma);
        (v, pi)
    }

    /// Howard policy iteration: greedy improvement + `eval_sweeps`
    /// of policy-evaluation Bellman per round, at most `rounds`
    /// improvements. Converges when the policy stops changing.
    /// Returns `(values, policy)` like [`Mdp::value_iteration`].
    pub fn policy_iteration(
        &self,
        gamma: Fixed,
        rounds: u32,
        eval_sweeps: u32,
    ) -> (Vec<Fixed>, Vec<u32>) {
        let n = self.actions.len();
        let mut v = vec![Fixed::ZERO; n];
        let mut pi = vec![0u32; n];
        for _ in 0..rounds {
            // Policy evaluation: V(s) ← Q(s, π(s)).
            for _ in 0..eval_sweeps {
                let mut next = vec![Fixed::ZERO; n];
                for (s, slot) in next.iter_mut().enumerate() {
                    if !self.actions[s].is_empty() {
                        *slot = self.q(
                            s,
                            (pi[s] as usize).min(self.actions[s].len() - 1),
                            &v,
                            gamma,
                        );
                    }
                }
                v = next;
            }
            let new_pi = self.greedy(&v, gamma);
            if new_pi == pi {
                break;
            }
            pi = new_pi;
        }
        (v, pi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> Mdp {
        Mdp::new(vec![
            vec![vec![(0, Fixed::ONE, 0)], vec![(1, Fixed::ONE, 10)]],
            vec![vec![(1, Fixed::ONE, 1)]],
        ])
    }

    #[test]
    fn value_iteration_prefers_rewarding_action() {
        let m = chain();
        let g = Fixed::from_ratio(9, 10);
        let (v, pi) = m.value_iteration(g, 300);
        assert_eq!(pi[0], 1);
        // V(1) = 1/(1−0.9) = 10; V(0) = 10 + 0.9·V(1) = 19.
        let v1 = Fixed::from_int(10);
        assert!(v[1] >= v1 - Fixed::from_ratio(1, 100));
        assert!(v[1] <= v1 + Fixed::from_ratio(1, 100));
        assert!(v[0] > v[1]);
    }

    #[test]
    fn stochastic_transitions_expected_value() {
        // Single state, single action: half +4, half −4 → V ≈ 0.
        let m = Mdp::new(vec![vec![vec![
            (0, Fixed::from_ratio(1, 2), 4),
            (0, Fixed::from_ratio(1, 2), -4),
        ]]]);
        let (v, _) = m.value_iteration(Fixed::ZERO, 10);
        assert_eq!(v[0], Fixed::ZERO);
    }

    #[test]
    fn policy_iteration_matches_value_iteration() {
        // A 3-state bandit-ish chain where action choice matters.
        let m = Mdp::new(vec![
            vec![
                vec![(0, Fixed::ONE, 0)], // stay
                vec![(1, Fixed::ONE, 5)], // →1
            ],
            vec![
                vec![(0, Fixed::ONE, -3)], // ←0 costly
                vec![(2, Fixed::ONE, 9)],  // →2
            ],
            vec![vec![(2, Fixed::ONE, 1)]], // absorbing
        ]);
        let g = Fixed::from_ratio(8, 10);
        let (vv, pv) = m.value_iteration(g, 400);
        let (vp, pp) = m.policy_iteration(g, 60, 30);
        assert_eq!(pv, pp);
        for s in 0..3 {
            assert!(vp[s] >= vv[s] - Fixed::from_ratio(1, 50));
        }
        // Optimal: 0→1→2.
        assert_eq!(pp, vec![1, 1, 0]);
    }

    #[test]
    fn monotonic_convergence() {
        let m = chain();
        let g = Fixed::from_ratio(9, 10);
        let (v10, _) = m.value_iteration(g, 10);
        let (v300, _) = m.value_iteration(g, 300);
        assert!(v300[0] >= v10[0]); // geometric warmup rises
    }

    #[test]
    fn edges() {
        // Empty MDP and action-less states don't panic.
        let e = Mdp::new(vec![]);
        let (v, pi) = e.value_iteration(Fixed::ONE, 5);
        assert!(v.is_empty() && pi.is_empty());
        let m = Mdp::new(vec![vec![]]);
        let (v, _) = m.value_iteration(Fixed::ONE, 5);
        assert_eq!(v[0], Fixed::ZERO);
        // Out-of-range transition targets clamp to the last state.
        let m = Mdp::new(vec![vec![vec![(99, Fixed::ONE, 2)]]]);
        let (v, _) = m.value_iteration(Fixed::ZERO, 3);
        assert_eq!(v[0], Fixed::from_int(2));
    }

    #[test]
    fn deterministic_twice() {
        let m = chain();
        let g = Fixed::from_ratio(7, 10);
        assert_eq!(m.value_iteration(g, 50), m.value_iteration(g, 50));
        assert_eq!(m.policy_iteration(g, 20, 10), m.policy_iteration(g, 20, 10));
    }
}

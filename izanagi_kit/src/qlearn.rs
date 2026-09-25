//! Tabular TD control — Watkins' Q-learning and Rummery's SARSA,
//! the online counterpart to `mdp`'s offline planners. The model
//! (transition table) is the environment's secret; all the learner
//! sees is `(s, a, r, s')` quadruples and a step size:
//!
//! ```text
//! Q-learning:  Q ← Q + α·(r + γ·max_a' Q(s',a') − Q)   (off-policy)
//! SARSA:       Q ← Q + α·(r + γ·Q(s', a')      − Q)   (on-policy)
//! ```
//!
//! ε-greedy exploration draws from a caller-owned `SplitMix64`, so
//! a training run replays bit-exactly from `(seed, episodes)`.
//!
//! ```
//! use izanagi_kit::qlearn::QLearn;
//! use izanagi_kit::fixed::Fixed;
//!
//! let mut q = QLearn::new(2, 2, Fixed::from_ratio(1, 2), Fixed::from_ratio(9, 10));
//! // s=0, a=1 pays 10 and lands in absorbing s=1.
//! for _ in 0..200 {
//!     q.update(0, 1, 10, 1);
//!     q.update(0, 0, 0, 0);
//! }
//! assert_eq!(q.best(0).0, 1);
//! ```

use crate::fixed::Fixed;
use crate::rng::SplitMix64;

/// A dense `states × actions` Q-table with `Fixed` arithmetic.
pub struct QLearn {
    q: Vec<Vec<Fixed>>,
    alpha: Fixed,
    gamma: Fixed,
}

impl QLearn {
    /// Fresh zero table. `alpha` is the learning rate, `gamma` the
    /// discount.
    pub fn new(states: usize, actions: usize, alpha: Fixed, gamma: Fixed) -> Self {
        QLearn {
            q: vec![vec![Fixed::ZERO; actions]; states],
            alpha,
            gamma,
        }
    }

    /// `Q(s, a)`; out-of-range reads return 0.
    pub fn q(&self, s: usize, a: usize) -> Fixed {
        self.q
            .get(s)
            .and_then(|row| row.get(a))
            .copied()
            .unwrap_or(Fixed::ZERO)
    }

    /// `argmax_a Q(s,a)` and its value; first maximizer wins ties.
    /// States without actions return `(0, 0)`.
    pub fn best(&self, s: usize) -> (u32, Fixed) {
        let row = match self.q.get(s) {
            Some(r) if !r.is_empty() => r,
            _ => return (0, Fixed::ZERO),
        };
        let mut best = 0usize;
        for (a, &v) in row.iter().enumerate().skip(1) {
            if v > row[best] {
                best = a;
            }
        }
        (best as u32, row[best])
    }

    /// ε-greedy action pick: with probability `eps` a uniform draw,
    /// otherwise [`QLearn::best`]. `eps ≤ 0` is pure greedy.
    pub fn select(&self, s: usize, eps: Fixed, rng: &mut SplitMix64) -> u32 {
        let row = match self.q.get(s) {
            Some(r) if !r.is_empty() => r,
            _ => return 0,
        };
        let draw = Fixed::from_raw(rng.next_u32() as i32 & 0xffff);
        if draw < eps {
            rng.next_u32() % row.len() as u32
        } else {
            self.best(s).0
        }
    }

    /// One Q-learning update for `(s, a, r, s_next)` — off-policy:
    /// bootstraps from `max_a' Q(s', a')`.
    pub fn update(&mut self, s: usize, a: usize, r: i64, s_next: usize) {
        let bootstrap = self.best(s_next).1;
        self.td(s, a, r, bootstrap);
    }

    /// One SARSA update for `(s, a, r, s_next, a_next)` — on-policy:
    /// bootstraps from `Q(s', a')`, the action actually taken.
    pub fn update_sarsa(&mut self, s: usize, a: usize, r: i64, s_next: usize, a_next: usize) {
        let bootstrap = self.q(s_next, a_next);
        self.td(s, a, r, bootstrap);
    }

    /// Shared TD step: `Q ← Q + α·(r + γ·boot − Q)`.
    fn td(&mut self, s: usize, a: usize, r: i64, boot: Fixed) {
        if s >= self.q.len() || self.q[s].is_empty() {
            return;
        }
        let a = a.min(self.q[s].len() - 1);
        let rc = r.clamp(-2_000_000_000, 2_000_000_000) as i32;
        let target = Fixed::from_int(rc) + self.gamma.mul(boot);
        let cur = self.q[s][a];
        self.q[s][a] = cur + self.alpha.mul(target - cur);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_learning_learns_the_better_action() {
        let mut q = QLearn::new(2, 2, Fixed::ONE, Fixed::ZERO);
        // α=1, γ=0: Q is literally the last reward seen.
        q.update(0, 0, 1, 1);
        q.update(0, 1, 5, 1);
        assert_eq!(q.q(0, 1), Fixed::from_int(5));
        assert_eq!(q.best(0).0, 1);
    }

    #[test]
    fn bootstraps_downstream_value() {
        // s0 --a=1--> s1 (r=0), s1 pays 10 absorbing.
        // With γ=1 the 10 must reach back to s0.
        let mut q = QLearn::new(3, 2, Fixed::ONE, Fixed::ONE);
        q.update(1, 0, 10, 2);
        q.update(0, 1, 0, 1);
        assert_eq!(q.q(0, 1), Fixed::from_int(10));
        assert_eq!(q.best(0).0, 1);
    }

    #[test]
    fn sarsa_uses_the_actual_next_action() {
        // Off-policy vs on: s' has Q(s',a)=4 under a'=0 but a
        // better action exists (Q=9). Q-learning picks 9;
        // SARSA takes the told a'=0 → 4.
        let mut q = QLearn::new(3, 2, Fixed::ONE, Fixed::ONE);
        q.update(1, 0, 4, 2); // Q(1,0) = 4
        q.update(1, 1, 9, 2); // Q(1,1) = 9
        q.update(0, 0, 0, 1);
        assert_eq!(q.q(0, 0), Fixed::from_int(9));
        let mut q2 = QLearn::new(3, 2, Fixed::ONE, Fixed::ONE);
        q2.update(1, 0, 4, 2);
        q2.update(1, 1, 9, 2);
        q2.update_sarsa(0, 0, 0, 1, 0);
        assert_eq!(q2.q(0, 0), Fixed::from_int(4));
    }

    #[test]
    fn learning_rate_interpolates() {
        let mut q = QLearn::new(1, 1, Fixed::from_ratio(1, 2), Fixed::ZERO);
        q.update(0, 0, 10, 0);
        assert_eq!(q.q(0, 0), Fixed::from_int(5));
        q.update(0, 0, 10, 0);
        assert_eq!(q.q(0, 0), Fixed::from_ratio(15, 2));
    }

    #[test]
    fn greedy_and_exploring_select() {
        let mut q = QLearn::new(1, 4, Fixed::ONE, Fixed::ZERO);
        q.update(0, 2, 7, 0);
        let mut rng = SplitMix64::new(9);
        // eps = 0 → always argmax.
        for _ in 0..20 {
            assert_eq!(q.select(0, Fixed::ZERO, &mut rng), 2);
        }
        // eps = 1 → explores; over many draws another action shows.
        let mut rng = SplitMix64::new(9);
        let mut seen_other = false;
        for _ in 0..50 {
            if q.select(0, Fixed::ONE, &mut rng) != 2 {
                seen_other = true;
            }
        }
        assert!(seen_other);
    }

    #[test]
    fn edges() {
        let mut q = QLearn::new(2, 2, Fixed::ONE, Fixed::ZERO);
        assert_eq!(q.q(9, 9), Fixed::ZERO);
        assert_eq!(q.best(9), (0, Fixed::ZERO));
        q.update(9, 9, 5, 9); // out of range: no-op, no panic
        let mut rng = SplitMix64::new(1);
        assert_eq!(q.select(9, Fixed::ONE, &mut rng), 0);
        // Empty action row.
        let q = QLearn::new(1, 0, Fixed::ONE, Fixed::ZERO);
        assert_eq!(q.best(0), (0, Fixed::ZERO));
    }

    #[test]
    fn deterministic_twice() {
        let train = |seed: u64| {
            let mut q = QLearn::new(4, 2, Fixed::from_ratio(1, 4), Fixed::from_ratio(9, 10));
            let mut rng = SplitMix64::new(seed);
            for _ in 0..500 {
                let a = q.select(0, Fixed::from_ratio(1, 10), &mut rng);
                q.update(0, a as usize, if a == 1 { 3 } else { 0 }, 1);
            }
            q
        };
        assert_eq!(train(42).q(0, 1), train(42).q(0, 1));
        assert_eq!(train(42).best(0).0, 1);
    }
}

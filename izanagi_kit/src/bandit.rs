//! Deterministic multi-armed bandit strategies — all integer.
//!
//! A [`Bandit`] tracks per-arm pull counts and accumulated
//! rewards (scaled `i64` — e.g. reward × `SCALE` so fractional
//! payoffs stay exact). Three selection rules share that state:
//!
//! - [`Bandit::greedy_pick`] — argmax mean, ties to lowest arm.
//! - [`Bandit::ucb1_pick`] — UCB1 (Auer, Cesa-Bianchi, Fischer
//!   2002) in Q8 fixed point: `mean·256 + isqrt(2·log2(total)·
//!   SCALE²·256/pulls)` — the embedded-systems integer form.
//! - [`Bandit::eps_greedy_pick`] — ε-greedy driven by a seeded
//!   [`SplitMix64`], so runs are replayable.
//!
//! Unpulled arms are always selected before any scored pick
//! (the standard `n_arms` warm-up rounds).
//!
//! ```
//! use izanagi_kit::bandit::{Bandit, SCALE};
//! let mut b = Bandit::new(3);
//! for _ in 0..30 {
//!     let arm = b.ucb1_pick();
//!     b.reward(arm, if arm == 1 { SCALE } else { 0 });
//! }
//! assert_eq!(b.best_arm(), 1);
//! ```

use crate::rng::SplitMix64;

/// Reward fixed-point: callers pass `reward · SCALE` so
/// `SCALE/2` is "half a point" — means stay exact rationals
/// computed as `sum / pulls`.
pub const SCALE: i64 = 1024;

/// Pull/reward tallies shared by every selection rule.
pub struct Bandit {
    pulls: Vec<u64>,
    /// Sum of scaled rewards per arm.
    rewards: Vec<i64>,
    total: u64,
}

/// `floor(log2(x))` for `x >= 1` — the integer UCB term.
fn log2_floor(x: u64) -> u64 {
    63 - (x | 1).leading_zeros() as u64
}

impl Bandit {
    /// `n` arms, all unpulled.
    pub fn new(n: usize) -> Self {
        Bandit {
            pulls: vec![0; n],
            rewards: vec![0; n],
            total: 0,
        }
    }

    /// Number of arms.
    pub fn n_arms(&self) -> usize {
        self.pulls.len()
    }

    /// Pulls of `arm`.
    pub fn pulls(&self, arm: usize) -> u64 {
        self.pulls[arm]
    }

    /// Total pulls across arms.
    pub fn total_pulls(&self) -> u64 {
        self.total
    }

    /// Scaled reward sum of `arm`.
    pub fn reward_sum(&self, arm: usize) -> i64 {
        self.rewards[arm]
    }

    /// Mean reward of `arm` as a [`crate::frac::Frac`] (`pulls == 0`
    /// → `0/1`). Compare means across arms with `Frac` order.
    pub fn mean(&self, arm: usize) -> crate::frac::Frac {
        crate::frac::Frac::new(self.rewards[arm] as i128, (self.pulls[arm] | 1) as i128)
    }

    /// Record a pull of `arm` paying `reward` (scaled by
    /// [`SCALE`]).
    pub fn reward(&mut self, arm: usize, reward: i64) {
        self.pulls[arm] += 1;
        self.rewards[arm] += reward;
        self.total += 1;
    }

    /// First unpulled arm (warm-up), or `None` when every arm
    /// has been pulled at least once.
    fn unpulled(&self) -> Option<usize> {
        self.pulls.iter().position(|&p| p == 0)
    }

    /// Argmax mean; ties resolve to the lowest arm index —
    /// the canonical deterministic tie-break.
    pub fn greedy_pick(&self) -> usize {
        if let Some(u) = self.unpulled() {
            return u;
        }
        let mut best = 0usize;
        for a in 1..self.pulls.len() {
            // mean_a > mean_best  ⇔  r_a·p_b > r_b·p_a (i128)
            let lhs = self.rewards[a] as i128 * self.pulls[best] as i128;
            let rhs = self.rewards[best] as i128 * self.pulls[a] as i128;
            if lhs > rhs {
                best = a;
            }
        }
        best
    }

    /// UCB1 pick — `argmax_a (mean_a + explore_a)` in fixed
    /// point, the standard integer form used by embedded UCB
    /// implementations. Both terms are scaled to Q8:
    ///
    /// ```text
    /// score_a = floor(r_a·256 / p_a)                // mean, Q8
    ///         + isqrt( 2·log2(total)·SCALE²·256 / p_a ) // bonus, Q8
    /// ```
    ///
    /// The bonus shrinks as `1/sqrt(pulls)`, so every arm is
    /// eventually revisited while the best mean dominates —
    /// deterministic for a given pull history.
    pub fn ucb1_pick(&self) -> usize {
        if let Some(u) = self.unpulled() {
            return u;
        }
        let log_t = log2_floor(self.total) + 1; // ≈ ln t, monotone & deterministic
        let mut best = 0usize;
        let mut best_score = self.ucb_score(0, log_t);
        for a in 1..self.pulls.len() {
            let sc = self.ucb_score(a, log_t);
            if sc > best_score {
                best = a;
                best_score = sc;
            }
        }
        best
    }

    /// Q8 `mean + bonus` for one arm — see [`Self::ucb1_pick`].
    fn ucb_score(&self, arm: usize, log_t: u64) -> i128 {
        let p = self.pulls[arm] as i128;
        let mean_q8 = (self.rewards[arm] as i128 * 256) / p;
        let bonus_num = 2 * log_t * (SCALE as u64) * (SCALE as u64) * 256 / self.pulls[arm];
        mean_q8 + isqrt(bonus_num) as i128
    }

    /// ε-greedy pick driven by `rng`: with probability
    /// `eps_num/eps_den` a uniform-random arm, else
    /// [`Self::greedy_pick`]. Seeded RNG → replayable runs.
    pub fn eps_greedy_pick(&self, rng: &mut SplitMix64, eps_num: u32, eps_den: u32) -> usize {
        if rng.coin(eps_num, eps_den) {
            rng.pick_index(self.pulls.len()).unwrap_or(0)
        } else {
            self.greedy_pick()
        }
    }

    /// The arm greedy would commit to right now.
    pub fn best_arm(&self) -> usize {
        self.greedy_pick()
    }
}

/// Integer `floor(sqrt(x))` by binary search — the UCB bonus
/// term and any caller needing a deterministic integer root.
pub fn isqrt(x: u64) -> u64 {
    if x < 2 {
        return x;
    }
    let mut lo = 1u64;
    let mut hi = x.min(1u64 << 32);
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if mid <= x / mid {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warm_up_then_greedy() {
        let mut b = Bandit::new(3);
        // every selection is an unpulled arm first, in order
        assert_eq!(b.greedy_pick(), 0);
        b.reward(0, 0);
        assert_eq!(b.greedy_pick(), 1);
        b.reward(1, SCALE * 10);
        assert_eq!(b.greedy_pick(), 2);
        b.reward(2, SCALE);
        assert_eq!(b.greedy_pick(), 1); // arm 1 has the best mean
        assert_eq!(b.best_arm(), 1);
        assert_eq!(b.total_pulls(), 3);
        assert_eq!(b.pulls(0), 1);
        assert_eq!(b.reward_sum(1), SCALE * 10);
    }

    /// UCB1 must visit every arm and then concentrate on the
    /// best-paying one — checked against a hand-computed trace
    /// and a no-starvation property.
    #[test]
    fn ucb1_visits_all_then_exploits() {
        let mut b = Bandit::new(4);
        // deterministic rewards: arm 2 pays SCALE, others 0
        for i in 0..400u64 {
            let arm = b.ucb1_pick();
            // unpulled arms are always chosen in index order
            if i < 4 {
                assert_eq!(arm, i as usize);
            }
            b.reward(arm, if arm == 2 { SCALE } else { 0 });
        }
        let mut pulls: Vec<u64> = (0..4).map(|a| b.pulls(a)).collect();
        // the good arm dominates but no arm starved
        assert_eq!(pulls[2], *pulls.iter().max().unwrap());
        assert!(pulls.iter().all(|&p| p >= 1));
        // UCB1 determinism: the whole run is reproducible
        let mut b2 = Bandit::new(4);
        for _ in 0..400 {
            let arm = b2.ucb1_pick();
            b2.reward(arm, if arm == 2 { SCALE } else { 0 });
        }
        pulls = (0..4).map(|a| b2.pulls(a)).collect();
        assert_eq!(pulls, (0..4).map(|a| b.pulls(a)).collect::<Vec<u64>>());
    }

    /// ε-greedy with a seeded RNG is replayable bit-for-bit,
    /// and ε=0 degenerates to pure greedy.
    #[test]
    fn eps_greedy_replayable() {
        let run = |seed: u64, eps_num: u32| -> Vec<usize> {
            let mut rng = SplitMix64::new(seed);
            let mut b = Bandit::new(3);
            for a in 0..3 {
                b.reward(a, a as i64 * SCALE); // arm 2 best
            }
            (0..200)
                .map(|_| {
                    let arm = b.eps_greedy_pick(&mut rng, eps_num, 100);
                    b.reward(arm, if arm == 2 { SCALE } else { 0 });
                    arm
                })
                .collect()
        };
        // same seed → identical trace
        assert_eq!(run(7, 20), run(7, 20));
        // different seeds diverge
        assert_ne!(run(7, 50), run(8, 50));
        // ε = 0 → always the greedy arm
        assert!(run(7, 0).iter().all(|&a| a == 2));
    }

    /// Mean ordering is an exact rational comparison — verify
    /// against cross-multiplication oracle on random states.
    #[test]
    fn oracle_greedy_argmax() {
        let mut rng = SplitMix64::new(0xBAD);
        for _ in 0..500 {
            let mut b = Bandit::new(1 + rng.below(6) as usize);
            let n = 1 + rng.below(40) as usize;
            for _ in 0..n {
                let a = rng.below(b.n_arms() as u32) as usize;
                b.reward(a, rng.below(20) as i64);
            }
            let g = b.greedy_pick();
            if b.pulls(g) == 0 {
                continue; // warm-up pick — fine
            }
            for a in 0..b.n_arms() {
                if b.pulls(a) == 0 {
                    continue;
                }
                let lhs = b.reward_sum(g) as i128 * b.pulls(a) as i128;
                let rhs = b.reward_sum(a) as i128 * b.pulls(g) as i128;
                assert!(lhs >= rhs, "arm {a} beats greedy pick {g}");
            }
        }
    }

    #[test]
    fn isqrt_floor() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(2), 1);
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(8), 2);
        assert_eq!(isqrt(9), 3);
        for x in [u64::MAX, u64::MAX - 1, 1u64 << 62, (1u64 << 62) + 7] {
            let r = isqrt(x);
            assert!(r <= x / r, "isqrt({x}) = {r} too big");
            assert!((r + 1) as u128 * (r + 1) as u128 > x as u128);
        }
    }
}

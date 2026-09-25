//! Simulated annealing — the generic metaheuristic (Kirkpatrick,
//! Gelatt & Vecchi 1983): wander the state space with a neighbor
//! operator, accept worsening moves with probability
//! `exp(−ΔE/T)` while the temperature cools geometrically
//! `T(t0) → T(t1)` over a fixed iteration budget.
//!
//! Determinism is the point here: every accept/reject decision draws
//! from a [`SplitMix64`] and the acceptance exponent is evaluated in
//! `Fixed`, so a given `(state, seed, iters, t0, t1)` tuple replays
//! bit-exactly. The caller owns the energy (lower is better) and the
//! neighbor generator; the kit owns the schedule and the dice.
//!
//! ```
//! use izanagi_kit::{anneal::anneal, fixed::Fixed};
//!
//! // Minimize f(x) = (x − 7)² over integers: neighbors ±1.
//! let (best, e) = anneal(
//!     &0i64,
//!     |x| (x - 7) * (x - 7),
//!     |x, rng| x + if rng.next_bool() { 1 } else { -1 },
//!     1,
//!     500,
//!     Fixed::from_int(4),
//!     Fixed::from_ratio(1, 1000),
//! );
//! assert!(e < 50, "annealer wandered far from the minimum: {best}");
//! ```

use crate::fixed::Fixed;
use crate::rng::SplitMix64;

/// Run `iters` annealing steps from `state`. Returns the
/// lowest-energy state seen and its energy (the classic
/// "keep the best" variant — the walk itself may end uphill).
///
/// `t0 > t1 > 0` gives cooling; either bound non-positive collapses
/// to a pure greedy walk (only improving moves accepted). `energy`
/// values are clamped into `Fixed` range for the Boltzmann factor.
#[allow(clippy::too_many_arguments)]
pub fn anneal<S: Clone>(
    state: &S,
    energy: impl Fn(&S) -> i64,
    mut neighbor: impl FnMut(&S, &mut SplitMix64) -> S,
    seed: u64,
    iters: u32,
    t0: Fixed,
    t1: Fixed,
) -> (S, i64) {
    let mut rng = SplitMix64::new(seed);
    let mut cur = state.clone();
    let mut cur_e = energy(&cur);
    let mut best = cur.clone();
    let mut best_e = cur_e;

    // Per-step log-temperature increment: ln T_i = ln t0 + f·Δ,
    // f = i/iters. Non-positive bounds → T = 0 (greedy only).
    let greedy = t0 <= Fixed::ZERO || t1 <= Fixed::ZERO;
    let ln_t0 = if greedy { Fixed::ZERO } else { t0.ln() };
    let d_ln = if greedy { Fixed::ZERO } else { t1.ln() - ln_t0 };

    for i in 0..iters {
        let next = neighbor(&cur, &mut rng);
        let next_e = energy(&next);
        let delta = next_e - cur_e;
        let accept = if delta <= 0 {
            true
        } else if greedy {
            false
        } else {
            // P = exp(−delta/T_i). −delta/T as raw 16.16:
            // −delta·65536 / t_raw, clamped so exp stays finite.
            let frac = Fixed::from_ratio(i as i32, iters as i32);
            let t_raw = (ln_t0 + frac.mul(d_ln)).exp().raw().max(1) as i64;
            let scaled = (delta.saturating_mul(65536) / t_raw).min(2_000_000_000);
            let prob = Fixed::from_raw(-(scaled as i32)).exp();
            Fixed::from_raw(rng.next_u32() as i32 & 0xffff) <= prob
        };
        if accept {
            cur = next;
            cur_e = next_e;
            if cur_e < best_e {
                best = cur.clone();
                best_e = cur_e;
            }
        }
    }
    (best, best_e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_finds_local_min() {
        // f(x) = (x−7)²: from 0, ±1 steps, T=0 → strictly downhill.
        let (best, e) = anneal(
            &0i64,
            |x| (x - 7) * (x - 7),
            |x, rng| x + if rng.next_bool() { 1 } else { -1 },
            0,
            40,
            Fixed::ZERO,
            Fixed::ZERO,
        );
        assert_eq!(e, 0);
        assert_eq!(best, 7);
    }

    #[test]
    fn escapes_local_minimum() {
        // Two wells on the integers: x=0 has energy 0, the band
        // 1..=5 is a +10 ridge, x ≥ 6 is a deeper −10 well.
        // Greedy from 0 never accepts the ridge and is stuck;
        // a hot start diffuses through and finds −10.
        let energy = |x: &i64| -> i64 {
            if *x >= 6 {
                -10
            } else if *x == 0 {
                0
            } else {
                10
            }
        };
        let (greedy_best, _) = anneal(
            &0i64,
            energy,
            |x, rng| x + if rng.next_bool() { 1 } else { -1 },
            0,
            300,
            Fixed::ZERO,
            Fixed::ZERO,
        );
        assert_eq!(greedy_best, 0);
        let mut found = false;
        for seed in 0..20 {
            let (b, e) = anneal(
                &0i64,
                energy,
                |x, rng| x + if rng.next_bool() { 1 } else { -1 },
                seed,
                400,
                Fixed::from_int(64),
                Fixed::from_ratio(1, 1000),
            );
            if e == -10 && b >= 6 {
                found = true;
            }
        }
        assert!(found, "no seed crossed the ridge");
    }

    #[test]
    fn deterministic_twice() {
        let a = anneal(
            &3i64,
            |x| x * x,
            |x, rng| x + rng.range_closed(-3, 3) as i64,
            77,
            200,
            Fixed::from_int(10),
            Fixed::ONE,
        );
        let b = anneal(
            &3i64,
            |x| x * x,
            |x, rng| x + rng.range_closed(-3, 3) as i64,
            77,
            200,
            Fixed::from_int(10),
            Fixed::ONE,
        );
        assert_eq!(a, b);
    }
}

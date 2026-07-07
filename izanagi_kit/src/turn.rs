//! Energy-based turn scheduler — speed-driven turn order.
//!
//! The standard roguelike progression model: each actor accumulates *energy*
//! proportional to its `speed` as time passes, and acts once it has banked a
//! full [`ACTION_COST`]. Faster actors act more often; the leftover energy
//! carries over, so fractional speed ratios stay fair over time. (See RogueBasin
//! "Time Systems" / the classic Angband-style energy loop.)
//!
//! Deterministic: time advances in whole units by closed-form `ceil` division
//! (no float), and when several actors are ready on the same unit the one with
//! the smallest id goes first — a fixed, replay-safe tie-break. Generic over the
//! actor id `A` so it is not tied to the ECS.

use std::cmp::Ordering;

/// Energy required to take one turn. An actor with `speed == ACTION_COST` acts
/// once per time unit; double that acts twice as often, half as often for half.
pub const ACTION_COST: i32 = 100;

#[derive(Clone, Copy, Debug)]
struct Actor<A> {
    id: A,
    speed: i32,
    energy: i32,
}

/// A speed-ordered turn queue over actor ids of type `A`.
#[derive(Clone, Debug, Default)]
pub struct Scheduler<A> {
    actors: Vec<Actor<A>>,
}

impl<A: Copy + Ord> Scheduler<A> {
    /// Create an empty scheduler.
    pub fn new() -> Scheduler<A> {
        Scheduler { actors: Vec::new() }
    }

    /// Number of scheduled actors.
    pub fn len(&self) -> usize {
        self.actors.len()
    }

    /// `true` if no actors are scheduled.
    pub fn is_empty(&self) -> bool {
        self.actors.is_empty()
    }

    /// `true` if `id` is currently scheduled.
    pub fn contains(&self, id: A) -> bool {
        self.actors.iter().any(|a| a.id == id)
    }

    /// Add an actor starting with zero energy. `speed` is clamped to `>= 1` so
    /// time always advances (a zero-speed actor would never act and could stall
    /// the scheduler). Re-adding an existing id just updates its speed.
    pub fn add(&mut self, id: A, speed: i32) {
        let speed = speed.max(1);
        if let Some(a) = self.actors.iter_mut().find(|a| a.id == id) {
            a.speed = speed;
        } else {
            self.actors.push(Actor {
                id,
                speed,
                energy: 0,
            });
        }
    }

    /// Remove an actor (e.g. on death). No-op if absent.
    pub fn remove(&mut self, id: A) {
        self.actors.retain(|a| a.id != id);
    }

    /// Change an actor's speed (hastened/slowed), keeping its banked energy.
    /// Clamped to `>= 1`. No-op if absent.
    pub fn set_speed(&mut self, id: A, speed: i32) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.id == id) {
            a.speed = speed.max(1);
        }
    }

    /// Current banked energy for actor `id`, or `None` if not registered.
    ///
    /// Useful for save/load (restore exact energy) and diagnostics. A value
    /// ≥ [`ACTION_COST`] means the actor will act on the next
    /// [`next_turn`](Self::next_turn) call.
    pub fn energy(&self, id: A) -> Option<i32> {
        self.actors.iter().find(|a| a.id == id).map(|a| a.energy)
    }

    /// Directly set banked energy for actor `id`. No-op if not registered.
    ///
    /// Values ≥ [`ACTION_COST`] make the actor immediately ready on the next
    /// [`next_turn`](Self::next_turn) call; negative values add a delay.
    /// Primarily used to restore saved state exactly.
    pub fn set_energy(&mut self, id: A, energy: i32) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.id == id) {
            a.energy = energy;
        }
    }

    /// Current speed for actor `id`, or `None` if not registered.
    /// Mirrors the `energy` API — a read-only complement to `set_speed`.
    pub fn speed(&self, id: A) -> Option<i32> {
        self.actors.iter().find(|a| a.id == id).map(|a| a.speed)
    }

    /// Count of actors whose banked energy has not yet reached [`ACTION_COST`].
    /// Complement of `actors_ready().len()` but avoids the allocation — use
    /// for UI countdowns ("N actors still waiting to act") and early-exit
    /// checks in AI planning loops.
    pub fn pending_count(&self) -> usize {
        self.actors
            .iter()
            .filter(|a| a.energy < ACTION_COST)
            .count()
    }

    /// All actors whose banked energy is ≥ [`ACTION_COST`] right now, in
    /// insertion order. Does **not** advance the queue or deduct energy — purely
    /// a diagnostic read. Useful for "who can act on this turn?" planning, AI
    /// look-ahead, and save-file inspection.
    pub fn actors_ready(&self) -> Vec<A> {
        self.actors
            .iter()
            .filter(|a| a.energy >= ACTION_COST)
            .map(|a| a.id)
            .collect()
    }

    /// Set an actor's banked energy to 0 — the "stun / interrupt" primitive.
    /// No-op if `id` is not registered. Equivalent to `set_energy(id, 0)` but
    /// clarifies intent at call sites: "this actor loses its current turn".
    #[inline]
    pub fn reset_actor(&mut self, id: A) {
        self.set_energy(id, 0);
    }

    /// Iterate all registered actor ids in insertion order.
    /// Useful for serialisation, debug overlays, and tests that need to
    /// enumerate every actor without driving the turn queue.
    pub fn iter_actors(&self) -> impl Iterator<Item = A> + '_ {
        self.actors.iter().map(|a| a.id)
    }

    /// Collect all registered actor ids into a `Vec` in insertion order.
    /// Convenience wrapper around `iter_actors().collect()` — avoids the
    /// explicit type annotation at call sites where a `Vec` is expected.
    pub fn all_actors(&self) -> Vec<A> {
        self.iter_actors().collect()
    }

    /// How many time units until actor `id` first accumulates ≥ `ACTION_COST`
    /// energy. Returns `Some(0)` if the actor is already ready. Returns `None`
    /// if `id` is not registered. Does **not** advance the queue.
    ///
    /// Useful for AI planning ("the goblin acts in 3 turns") and UI countdown
    /// displays without driving the scheduler forward.
    pub fn time_until_ready(&self, id: A) -> Option<i32> {
        let actor = self.actors.iter().find(|a| a.id == id)?;
        let deficit = ACTION_COST - actor.energy;
        if deficit <= 0 {
            Some(0)
        } else {
            Some((deficit + actor.speed - 1) / actor.speed)
        }
    }

    /// Index of the ready actor (energy ≥ cost) with the smallest id, if any.
    fn ready(&self) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (i, a) in self.actors.iter().enumerate() {
            if a.energy < ACTION_COST {
                continue;
            }
            match best {
                None => best = Some(i),
                Some(b) if a.id.cmp(&self.actors[b].id) == Ordering::Less => best = Some(i),
                _ => {}
            }
        }
        best
    }

    /// The id of the actor who is ready right now and would act on the next
    /// [`next_turn`](Self::next_turn) call, without advancing time or deducting
    /// energy. Returns `None` if no actor is currently ready (or the scheduler
    /// is empty). Uses the same smallest-id tie-break as `next_turn`, so it is a
    /// true non-destructive preview. Useful for "whose turn is it?" UI and AI
    /// look-ahead.
    pub fn peek_next_turn(&self) -> Option<A> {
        self.ready().map(|i| self.actors[i].id)
    }

    /// Non-destructively simulate the next `n` turns and return the ids of the
    /// actors who would act, **in order**. This is the "turn-order timeline" /
    /// initiative bar that turn-based games show the player (e.g. Final Fantasy
    /// Tactics, Into the Breach): a look-ahead of who acts next, next-next, etc.
    ///
    /// The simulation mirrors [`next_turn`](Self::next_turn) exactly — same
    /// time-advance rule and same smallest-id tie-break — but runs on a private
    /// copy, so the scheduler is left untouched. An actor can appear multiple
    /// times in the result if it is fast enough to act more than once within the
    /// window. Returns an empty `Vec` if the scheduler is empty; otherwise the
    /// result has exactly `n` entries (idle actors still regenerate energy).
    pub fn forecast(&self, n: usize) -> Vec<A> {
        let mut order = Vec::with_capacity(n);
        if self.actors.is_empty() || n == 0 {
            return order;
        }
        // Local mutable copy of (id, speed, energy) so we don't disturb state.
        let mut sim: Vec<Actor<A>> = self.actors.clone();
        for _ in 0..n {
            // Find the ready actor (energy >= cost) with the smallest id.
            let mut ready: Option<usize> = None;
            for (i, a) in sim.iter().enumerate() {
                if a.energy < ACTION_COST {
                    continue;
                }
                match ready {
                    None => ready = Some(i),
                    Some(b) if a.id.cmp(&sim[b].id) == Ordering::Less => ready = Some(i),
                    _ => {}
                }
            }
            // If nobody is ready, advance time by the minimum whole units needed
            // for someone to reach the threshold (same i64 math as next_turn).
            let idx = match ready {
                Some(i) => i,
                None => {
                    let units: i64 = sim
                        .iter()
                        .map(|a| {
                            let deficit = ACTION_COST as i64 - a.energy as i64;
                            if deficit <= 0 {
                                0
                            } else {
                                (deficit + a.speed as i64 - 1) / a.speed as i64
                            }
                        })
                        .min()
                        .unwrap_or(0);
                    for a in &mut sim {
                        let delta = (a.speed as i64).saturating_mul(units);
                        a.energy = (a.energy as i64)
                            .saturating_add(delta)
                            .clamp(i32::MIN as i64, i32::MAX as i64)
                            as i32;
                    }
                    // Recompute the smallest-id ready actor after advancing.
                    let mut best: Option<usize> = None;
                    for (i, a) in sim.iter().enumerate() {
                        if a.energy < ACTION_COST {
                            continue;
                        }
                        match best {
                            None => best = Some(i),
                            Some(b) if a.id.cmp(&sim[b].id) == Ordering::Less => best = Some(i),
                            _ => {}
                        }
                    }
                    best.expect("an actor is ready after advancing")
                }
            };
            order.push(sim[idx].id);
            sim[idx].energy = sim[idx].energy.saturating_sub(ACTION_COST);
        }
        order
    }

    /// Remove all actors and return their ids in insertion order.
    ///
    /// Useful for "end of floor" cleanup where every actor's ECS entity needs
    /// to be despawned: drain the scheduler once, then remove all entities in
    /// one pass. After the call the scheduler is empty.
    pub fn drain(&mut self) -> Vec<A> {
        let ids: Vec<A> = self.actors.iter().map(|a| a.id).collect();
        self.actors.clear();
        ids
    }

    /// Remove all actors without returning them. Cheaper than `drain` when
    /// the caller does not need the id list (e.g. a full scene reset that
    /// destroys everything at once). After the call the scheduler is empty.
    #[inline]
    pub fn clear(&mut self) {
        self.actors.clear();
    }

    /// Advance time until an actor is ready, then return it and deduct one
    /// action's worth of energy. Returns `None` only when empty.
    ///
    /// Among actors that become ready on the same time unit, the smallest id
    /// acts first (deterministic). Energy beyond the cost carries over.
    pub fn next_turn(&mut self) -> Option<A> {
        if self.actors.is_empty() {
            return None;
        }
        if self.ready().is_none() {
            // Jump straight to the first unit on which someone is ready: the
            // minimum ceil((cost - energy)/speed) over all actors.
            // i64 throughout: callers may set extreme energy (down to i32::MIN
            // via set_energy) or extreme speed, so `cost - energy`, the ceil
            // division, and `speed * units` all overflow i32 otherwise.
            let units: i64 = self
                .actors
                .iter()
                .map(|a| {
                    let deficit = ACTION_COST as i64 - a.energy as i64;
                    if deficit <= 0 {
                        0
                    } else {
                        // ceil(deficit / speed); speed >= 1 by construction.
                        (deficit + a.speed as i64 - 1) / a.speed as i64
                    }
                })
                .min()
                .unwrap_or(0);
            for a in &mut self.actors {
                let delta = (a.speed as i64).saturating_mul(units);
                a.energy = (a.energy as i64)
                    .saturating_add(delta)
                    .clamp(i32::MIN as i64, i32::MAX as i64) as i32;
            }
        }
        let i = self.ready().expect("an actor is ready after advancing");
        self.actors[i].energy = self.actors[i].energy.saturating_sub(ACTION_COST);
        Some(self.actors[i].id)
    }
}

impl<A: Copy + Ord + crate::world_hash::DetHash> Scheduler<A> {
    /// Fold the scheduler into a hasher in canonical id order (so it can be part
    /// of replay-checked world state, independent of add order).
    pub fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        let mut ordered: Vec<&Actor<A>> = self.actors.iter().collect();
        ordered.sort_unstable_by(|x, y| x.id.cmp(&y.id));
        hasher.write_u32(ordered.len() as u32);
        for a in ordered {
            a.id.det_hash(hasher);
            hasher.write_i32(a.speed);
            hasher.write_i32(a.energy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_scheduler_yields_none() {
        let mut s: Scheduler<u32> = Scheduler::new();
        assert_eq!(s.next_turn(), None);
    }

    #[test]
    fn test_single_actor_acts_every_call() {
        let mut s = Scheduler::new();
        s.add(7u32, ACTION_COST);
        for _ in 0..5 {
            assert_eq!(s.next_turn(), Some(7));
        }
    }

    #[test]
    fn test_faster_actor_acts_proportionally_more() {
        let mut s = Scheduler::new();
        s.add(1u32, ACTION_COST); // normal
        s.add(2u32, ACTION_COST * 2); // double speed
        let mut fast: i32 = 0;
        let mut slow: i32 = 0;
        for _ in 0..300 {
            match s.next_turn().unwrap() {
                2 => fast += 1,
                1 => slow += 1,
                _ => unreachable!(),
            }
        }
        // Double speed → roughly twice the turns.
        assert_eq!(fast + slow, 300);
        assert!(
            fast > slow,
            "faster actor should act more ({fast} vs {slow})"
        );
        assert!((fast - 2 * slow).abs() <= 2, "ratio ~2:1");
    }

    #[test]
    fn test_equal_speed_breaks_ties_by_id_deterministically() {
        let mut s = Scheduler::new();
        s.add(2u32, ACTION_COST);
        s.add(1u32, ACTION_COST);
        s.add(3u32, ACTION_COST);
        // All ready at the same unit → ascending id order.
        assert_eq!(s.next_turn(), Some(1));
        assert_eq!(s.next_turn(), Some(2));
        assert_eq!(s.next_turn(), Some(3));
    }

    #[test]
    fn test_remove_stops_scheduling_an_actor() {
        let mut s = Scheduler::new();
        s.add(1u32, ACTION_COST);
        s.add(2u32, ACTION_COST);
        s.remove(1);
        for _ in 0..4 {
            assert_eq!(s.next_turn(), Some(2));
        }
        assert!(!s.contains(1));
    }

    #[test]
    fn test_set_speed_takes_effect() {
        let mut s = Scheduler::new();
        s.add(1u32, ACTION_COST);
        s.add(2u32, ACTION_COST);
        s.set_speed(1, ACTION_COST * 3); // hasten actor 1
        let mut a1 = 0;
        for _ in 0..400 {
            if s.next_turn() == Some(1) {
                a1 += 1;
            }
        }
        assert!(a1 > 250, "hastened actor should dominate ({a1}/400)");
    }

    #[test]
    fn test_energy_returns_none_for_unknown_actor() {
        let s: Scheduler<u32> = Scheduler::new();
        assert_eq!(s.energy(42), None);
    }

    #[test]
    fn test_energy_starts_at_zero_and_set_energy_restores_state() {
        let mut s = Scheduler::new();
        s.add(1u32, ACTION_COST);
        assert_eq!(s.energy(1), Some(0));
        // Advance one turn; energy returns to 0 after the cost is deducted.
        s.next_turn();
        assert_eq!(s.energy(1), Some(0));
        // Restore to a specific value (e.g. mid-save point).
        s.set_energy(1, 50);
        assert_eq!(s.energy(1), Some(50));
    }

    #[test]
    fn test_set_energy_to_action_cost_makes_actor_immediately_ready() {
        let mut s = Scheduler::new();
        s.add(1u32, ACTION_COST);
        s.add(2u32, ACTION_COST);
        // Force actor 2 to act before actor 1 by giving it a full energy bank.
        s.set_energy(2, ACTION_COST);
        // Actor 2 should be ready first (energy >= ACTION_COST).
        assert_eq!(s.next_turn(), Some(2));
    }

    #[test]
    fn test_speed_returns_none_for_unknown_actor() {
        let s: Scheduler<u32> = Scheduler::new();
        assert_eq!(s.speed(99), None);
    }

    #[test]
    fn test_speed_returns_registered_speed() {
        let mut s = Scheduler::new();
        s.add(5u32, ACTION_COST * 2);
        assert_eq!(s.speed(5), Some(ACTION_COST * 2));
    }

    #[test]
    fn test_speed_reflects_set_speed() {
        let mut s = Scheduler::new();
        s.add(1u32, ACTION_COST);
        s.set_speed(1, ACTION_COST * 3);
        assert_eq!(s.speed(1), Some(ACTION_COST * 3));
    }

    #[test]
    fn test_iter_actors_empty() {
        let s: Scheduler<u32> = Scheduler::new();
        assert_eq!(s.iter_actors().count(), 0);
    }

    #[test]
    fn test_iter_actors_yields_all_ids_in_insertion_order() {
        let mut s = Scheduler::new();
        s.add(3u32, ACTION_COST);
        s.add(1u32, ACTION_COST);
        s.add(2u32, ACTION_COST);
        let ids: Vec<u32> = s.iter_actors().collect();
        assert_eq!(ids, vec![3, 1, 2]);
    }

    #[test]
    fn test_iter_actors_excludes_removed_actor() {
        let mut s = Scheduler::new();
        s.add(1u32, ACTION_COST);
        s.add(2u32, ACTION_COST);
        s.remove(1);
        let ids: Vec<u32> = s.iter_actors().collect();
        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn test_sequence_is_deterministic() {
        let run = || {
            let mut s = Scheduler::new();
            s.add(1u32, 100);
            s.add(2u32, 70);
            s.add(3u32, 130);
            (0..50).map(|_| s.next_turn().unwrap()).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn test_actors_ready_empty_when_none_have_enough_energy() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.add(2, 100);
        // No time has passed; energy = 0.
        assert!(s.actors_ready().is_empty());
    }

    #[test]
    fn test_actors_ready_returns_all_with_enough_energy() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.add(2, 100);
        s.set_energy(1, ACTION_COST); // ready
        s.set_energy(2, ACTION_COST + 50); // also ready
        let ready = s.actors_ready();
        assert!(ready.contains(&1));
        assert!(ready.contains(&2));
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_actors_ready_does_not_advance_queue() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.set_energy(1, ACTION_COST);
        let _ = s.actors_ready();
        // Actor 1 must still be ready (no energy deducted).
        assert_eq!(s.energy(1), Some(ACTION_COST));
    }

    #[test]
    fn test_reset_actor_sets_energy_to_zero() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.set_energy(1, 200);
        s.reset_actor(1);
        assert_eq!(s.energy(1), Some(0));
    }

    #[test]
    fn test_reset_actor_noop_for_unknown_id() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.reset_actor(99); // unknown — must not panic
        assert_eq!(s.energy(1), Some(0));
    }

    #[test]
    fn test_time_until_ready_already_ready() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.set_energy(1, ACTION_COST);
        assert_eq!(s.time_until_ready(1), Some(0));
    }

    #[test]
    fn test_time_until_ready_needs_one_unit() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100); // speed 100, energy 0: needs 1 unit
        assert_eq!(s.time_until_ready(1), Some(1));
    }

    #[test]
    fn test_time_until_ready_unknown_returns_none() {
        let s: Scheduler<u32> = Scheduler::new();
        assert_eq!(s.time_until_ready(42), None);
    }

    #[test]
    fn test_time_until_ready_slow_actor_needs_more_units() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 50); // speed 50, energy 0: needs ceil(100/50)=2 units
        assert_eq!(s.time_until_ready(1), Some(2));
    }

    #[test]
    fn test_peek_next_turn_none_when_no_actor_ready() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100); // energy 0, not yet ready
        assert_eq!(s.peek_next_turn(), None);
    }

    #[test]
    fn test_peek_next_turn_matches_next_turn_without_side_effects() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.add(2, 100);
        s.set_energy(2, ACTION_COST); // make 2 ready
        let peeked = s.peek_next_turn();
        let energy_before = s.energy(2);
        assert_eq!(peeked, Some(2));
        // Peeking must not deduct energy.
        assert_eq!(s.energy(2), energy_before);
        // And the actual next_turn returns the same actor.
        assert_eq!(s.next_turn(), Some(2));
    }

    #[test]
    fn test_peek_next_turn_empty_scheduler_returns_none() {
        let s: Scheduler<u32> = Scheduler::new();
        assert_eq!(s.peek_next_turn(), None);
    }

    // --- forecast (turn-order timeline) ---

    #[test]
    fn test_forecast_empty_or_zero_is_empty() {
        let mut s: Scheduler<u32> = Scheduler::new();
        assert!(s.forecast(5).is_empty(), "empty scheduler → empty forecast");
        s.add(1, 100);
        assert!(s.forecast(0).is_empty(), "n=0 → empty forecast");
    }

    #[test]
    fn test_forecast_matches_repeated_next_turn() {
        // The forecast must be exactly what repeated next_turn() would yield.
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.add(2, 60);
        s.add(3, 150);
        let predicted = s.forecast(12);

        let mut actual = Vec::new();
        for _ in 0..12 {
            actual.push(s.next_turn().unwrap());
        }
        assert_eq!(predicted, actual, "forecast must match real turn sequence");
    }

    #[test]
    fn test_forecast_is_non_destructive() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        s.add(2, 100);
        let e1_before = s.energy(1);
        let e2_before = s.energy(2);
        let _ = s.forecast(20);
        assert_eq!(s.energy(1), e1_before, "forecast must not mutate energy");
        assert_eq!(s.energy(2), e2_before);
        assert_eq!(s.len(), 2, "forecast must not add/remove actors");
    }

    #[test]
    fn test_forecast_faster_actor_appears_more_often() {
        // A double-speed actor should act about twice as often in the window.
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 200); // fast
        s.add(2, 100); // slow
        let order = s.forecast(30);
        let fast = order.iter().filter(|&&a| a == 1).count();
        let slow = order.iter().filter(|&&a| a == 2).count();
        assert!(
            fast > slow,
            "faster actor must act more often ({fast} vs {slow})"
        );
    }

    #[test]
    fn test_forecast_length_is_n_when_non_empty() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, 100);
        assert_eq!(
            s.forecast(7).len(),
            7,
            "non-empty scheduler yields exactly n"
        );
    }

    #[test]
    fn test_drain_returns_all_ids_in_insertion_order() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(3, ACTION_COST);
        s.add(1, ACTION_COST);
        s.add(2, ACTION_COST);
        let ids = s.drain();
        assert_eq!(ids, vec![3, 1, 2]);
        assert!(s.is_empty());
    }

    #[test]
    fn test_drain_on_empty_returns_empty_vec() {
        let mut s: Scheduler<u32> = Scheduler::new();
        assert!(s.drain().is_empty());
    }

    #[test]
    fn test_drain_clears_scheduler() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(5, ACTION_COST);
        s.add(6, ACTION_COST);
        s.drain();
        assert_eq!(s.len(), 0);
        assert_eq!(s.next_turn(), None);
    }

    #[test]
    fn test_clear_removes_all_actors() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, ACTION_COST);
        s.add(2, ACTION_COST);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_clear_on_empty_is_noop() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_clear_leaves_scheduler_empty_like_drain() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, ACTION_COST);
        s.add(2, ACTION_COST);
        s.clear();
        assert_eq!(s.next_turn(), None);
    }

    // --- pending_count ---

    #[test]
    fn test_pending_count_all_not_ready() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, ACTION_COST);
        s.add(2, ACTION_COST);
        assert_eq!(s.pending_count(), 2);
    }

    #[test]
    fn test_pending_count_decreases_as_actors_become_ready() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, ACTION_COST);
        s.add(2, ACTION_COST);
        s.add(3, ACTION_COST);
        s.next_turn(); // advances time until someone is ready and pops them
        assert!(s.pending_count() <= 3);
    }

    #[test]
    fn test_pending_count_plus_ready_equals_total() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, ACTION_COST);
        s.add(2, ACTION_COST * 2); // fast actor, ready immediately after advance
        s.add(3, ACTION_COST);
        s.next_turn(); // pop one ready actor
        assert_eq!(s.pending_count() + s.actors_ready().len(), s.len());
    }

    // --- all_actors ---

    #[test]
    fn test_all_actors_returns_all_ids() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(10, ACTION_COST);
        s.add(20, ACTION_COST);
        s.add(30, ACTION_COST);
        let all = s.all_actors();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&10));
        assert!(all.contains(&20));
        assert!(all.contains(&30));
    }

    #[test]
    fn test_all_actors_empty_scheduler() {
        let s: Scheduler<u32> = Scheduler::new();
        assert!(s.all_actors().is_empty());
    }

    #[test]
    fn test_all_actors_matches_iter_actors() {
        let mut s: Scheduler<u32> = Scheduler::new();
        s.add(1, ACTION_COST);
        s.add(2, ACTION_COST);
        let from_all: Vec<u32> = s.all_actors();
        let from_iter: Vec<u32> = s.iter_actors().collect();
        assert_eq!(from_all, from_iter);
    }
}

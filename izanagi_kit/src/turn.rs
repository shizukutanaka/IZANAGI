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
    pub fn new() -> Scheduler<A> {
        Scheduler { actors: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.actors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actors.is_empty()
    }

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
            let units = self
                .actors
                .iter()
                .map(|a| {
                    let deficit = ACTION_COST - a.energy;
                    if deficit <= 0 {
                        0
                    } else {
                        (deficit + a.speed - 1) / a.speed
                    }
                })
                .min()
                .unwrap_or(0);
            for a in &mut self.actors {
                a.energy += a.speed * units;
            }
        }
        let i = self.ready().expect("an actor is ready after advancing");
        self.actors[i].energy -= ACTION_COST;
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
}

//! Condition → action rules — the missing link between game state and effects.
//!
//! [`dialogue`](crate::dialogue) produces a player choice, [`quest`](crate::quest)
//! tracks task completion, and [`status`](crate::status)/[`eventqueue`](crate::eventqueue)
//! carry timed or immediate effects — but nothing tied an arbitrary **condition**
//! ("is this quest active?", "is the player in this zone?") to a **chain of
//! actions** ("show this dialogue", "grant this item", "start this encounter").
//! [`TriggerSet`] is that missing rule layer.
//!
//! Following the same decoupling as [`tween`](crate::tween) and
//! [`recipe`](crate::recipe) — where a curve function or match predicate is
//! supplied at call time rather than stored — a trigger's condition and
//! actions are **opaque data** (`C` and `A`), not closures. The caller
//! evaluates conditions against the live world with a plain function passed
//! to [`check`](TriggerSet::check); [`TriggerSet`] only tracks *which* rules
//! exist and *whether* a one-shot rule has already consumed its single firing.
//! This keeps the trigger set itself trivially [`DetHash`](crate::world_hash::DetHash)
//! — no function pointers, no captured state.
//!
//! ```
//! use izanagi_kit::trigger::{Trigger, TriggerSet};
//!
//! #[derive(Clone, PartialEq, Eq)]
//! enum Cond { QuestActive(u32), PlayerInZone(u32) }
//!
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! enum Action { ShowDialogue(u32), GrantItem(u32), StartEncounter(u32) }
//!
//! let mut triggers: TriggerSet<&str, Cond, Action> = TriggerSet::new();
//! triggers.insert("goblin_ambush", Trigger::once(
//!     Cond::PlayerInZone(3),
//!     vec![Action::StartEncounter(7)],
//! ));
//! triggers.insert("shopkeeper_greeting", Trigger::new(
//!     Cond::PlayerInZone(1),
//!     vec![Action::ShowDialogue(2)],
//! ));
//!
//! // The caller supplies the evaluator — TriggerSet never inspects world state itself.
//! let player_zone = 3;
//! let fired = triggers.check(|c| matches!(c, Cond::PlayerInZone(z) if *z == player_zone));
//! assert_eq!(fired, vec![("goblin_ambush", vec![Action::StartEncounter(7)])]);
//!
//! // The one-shot ambush won't fire again even if the condition still holds.
//! let fired_again = triggers.check(|c| matches!(c, Cond::PlayerInZone(z) if *z == player_zone));
//! assert!(fired_again.is_empty());
//! ```
//!
//! ## Design
//!
//! Rules live in a `BTreeMap<K, Trigger<C, A>>` for canonical iteration and
//! hashing. [`check`](TriggerSet::check) evaluates every **armed** rule (not
//! yet fired, for a [`once`](Trigger::once) rule) in ascending key order and
//! returns `(key, actions)` for each that fires — the caller then dispatches
//! those actions however it likes (this module intentionally does not invoke
//! them, keeping the boundary between "which rules fired" and "what firing
//! means" explicit). A repeatable ([`new`](Trigger::new)) rule fires every
//! `check` call where its condition holds; a one-shot rule fires at most once
//! until explicitly [`reset`](TriggerSet::reset).

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::{BTreeMap, BTreeSet};

/// A single rule: a condition plus the actions to run when it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trigger<C, A> {
    condition: C,
    actions: Vec<A>,
    once: bool,
}

impl<C, A> Trigger<C, A> {
    /// A repeatable trigger: fires every time its condition holds.
    pub fn new(condition: C, actions: Vec<A>) -> Self {
        Trigger {
            condition,
            actions,
            once: false,
        }
    }

    /// A one-shot trigger: fires at most once, then stays disarmed until
    /// [`TriggerSet::reset`] re-arms it.
    pub fn once(condition: C, actions: Vec<A>) -> Self {
        Trigger {
            condition,
            actions,
            once: true,
        }
    }

    /// The rule's condition.
    pub fn condition(&self) -> &C {
        &self.condition
    }

    /// The actions this rule fires.
    pub fn actions(&self) -> &[A] {
        &self.actions
    }

    /// `true` if this rule fires at most once.
    pub fn is_once(&self) -> bool {
        self.once
    }
}

impl<C: DetHash, A: DetHash> DetHash for Trigger<C, A> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.condition.det_hash(hasher);
        hasher.write_u32(self.actions.len() as u32);
        for a in &self.actions {
            a.det_hash(hasher);
        }
        hasher.write_bool(self.once);
    }
}

/// A collection of named [`Trigger`]s, evaluated together each tick.
#[derive(Clone, Debug, Default)]
pub struct TriggerSet<K: Ord + Clone, C, A> {
    triggers: BTreeMap<K, Trigger<C, A>>,
    fired: BTreeSet<K>,
}

impl<K: Ord + Clone, C: Clone, A: Clone> TriggerSet<K, C, A> {
    /// An empty trigger set.
    pub fn new() -> Self {
        TriggerSet {
            triggers: BTreeMap::new(),
            fired: BTreeSet::new(),
        }
    }

    /// Add or replace the rule at `key`. Replacing a rule clears any prior
    /// fired-state for that key (the new rule starts armed).
    pub fn insert(&mut self, key: K, trigger: Trigger<C, A>) {
        self.fired.remove(&key);
        self.triggers.insert(key, trigger);
    }

    /// Remove the rule at `key`, returning it if present.
    pub fn remove(&mut self, key: K) -> Option<Trigger<C, A>> {
        self.fired.remove(&key);
        self.triggers.remove(&key)
    }

    /// The rule at `key`, if any.
    pub fn get(&self, key: K) -> Option<&Trigger<C, A>> {
        self.triggers.get(&key)
    }

    /// `true` if `key` names a rule that is currently eligible to fire: it
    /// exists, and is either repeatable or a not-yet-fired one-shot.
    pub fn is_armed(&self, key: K) -> bool {
        match self.triggers.get(&key) {
            Some(t) => !t.once || !self.fired.contains(&key),
            None => false,
        }
    }

    /// `true` if the one-shot rule at `key` has already consumed its firing.
    /// Always `false` for a repeatable rule or an absent key.
    pub fn is_fired(&self, key: K) -> bool {
        self.fired.contains(&key)
    }

    /// Re-arm the one-shot rule at `key` so it can fire again. A no-op for a
    /// repeatable rule or an absent key.
    pub fn reset(&mut self, key: K) {
        self.fired.remove(&key);
    }

    /// Re-arm every one-shot rule.
    pub fn reset_all(&mut self) {
        self.fired.clear();
    }

    /// The number of rules currently held (armed or fired).
    pub fn len(&self) -> usize {
        self.triggers.len()
    }

    /// `true` if no rules are held.
    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty()
    }

    /// Evaluate every armed rule against `eval` in ascending key order and
    /// return `(key, actions)` for each whose condition holds this call. A
    /// fired one-shot rule is marked and will not appear again until
    /// [`reset`](Self::reset). The caller owns dispatching the returned
    /// actions — this method only decides which rules fired.
    pub fn check<F: Fn(&C) -> bool>(&mut self, eval: F) -> Vec<(K, Vec<A>)> {
        let mut fired = Vec::new();
        for (key, trigger) in &self.triggers {
            if trigger.once && self.fired.contains(key) {
                continue;
            }
            if eval(&trigger.condition) {
                if trigger.once {
                    self.fired.insert(key.clone());
                }
                fired.push((key.clone(), trigger.actions.clone()));
            }
        }
        fired
    }
}

impl<K: Ord + Clone + DetHash, C: Clone + DetHash, A: Clone + DetHash> DetHash
    for TriggerSet<K, C, A>
{
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.triggers.len() as u32);
        for (key, trigger) in &self.triggers {
            key.det_hash(hasher);
            trigger.det_hash(hasher);
        }
        hasher.write_u32(self.fired.len() as u32);
        for key in &self.fired {
            key.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_empty() {
        let ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        assert!(ts.is_empty());
        assert_eq!(ts.len(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::new(true, vec![10, 20]));
        assert_eq!(ts.len(), 1);
        let t = ts.get(1).unwrap();
        assert_eq!(t.actions(), &[10, 20]);
        assert!(!t.is_once());
    }

    #[test]
    fn test_remove_returns_former_trigger() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::new(true, vec![10]));
        let removed = ts.remove(1);
        assert!(removed.is_some());
        assert!(ts.is_empty());
        assert_eq!(ts.remove(1), None, "second remove returns None");
    }

    #[test]
    fn test_repeatable_trigger_fires_every_check() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::new(true, vec![10]));
        for _ in 0..5 {
            let fired = ts.check(|c| *c);
            assert_eq!(fired, vec![(1, vec![10])], "repeatable rule fires every call");
        }
    }

    #[test]
    fn test_once_trigger_fires_only_once() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::once(true, vec![10]));
        let first = ts.check(|c| *c);
        assert_eq!(first, vec![(1, vec![10])]);
        let second = ts.check(|c| *c);
        assert!(second.is_empty(), "once-trigger must not fire twice");
    }

    #[test]
    fn test_condition_false_never_fires() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::new(false, vec![10]));
        let fired = ts.check(|c| *c);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_is_armed_reflects_once_state() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::once(true, vec![10]));
        assert!(ts.is_armed(1));
        ts.check(|c| *c);
        assert!(!ts.is_armed(1), "fired once-trigger is disarmed");
        assert!(ts.is_fired(1));
    }

    #[test]
    fn test_is_armed_absent_key_is_false() {
        let ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        assert!(!ts.is_armed(999));
        assert!(!ts.is_fired(999));
    }

    #[test]
    fn test_repeatable_is_always_armed() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::new(true, vec![10]));
        ts.check(|c| *c);
        assert!(ts.is_armed(1), "repeatable rule stays armed after firing");
    }

    #[test]
    fn test_reset_rearms_a_fired_once_trigger() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::once(true, vec![10]));
        ts.check(|c| *c);
        assert!(!ts.is_armed(1));
        ts.reset(1);
        assert!(ts.is_armed(1));
        let fired = ts.check(|c| *c);
        assert_eq!(fired, vec![(1, vec![10])], "reset rule fires again");
    }

    #[test]
    fn test_reset_all_rearms_every_once_trigger() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::once(true, vec![10]));
        ts.insert(2, Trigger::once(true, vec![20]));
        ts.check(|c| *c);
        assert!(!ts.is_armed(1) && !ts.is_armed(2));
        ts.reset_all();
        assert!(ts.is_armed(1) && ts.is_armed(2));
    }

    #[test]
    fn test_insert_replacing_a_rule_rearms_it() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::once(true, vec![10]));
        ts.check(|c| *c);
        assert!(!ts.is_armed(1));
        ts.insert(1, Trigger::once(true, vec![99]));
        assert!(ts.is_armed(1), "replacing a rule starts it armed again");
        let fired = ts.check(|c| *c);
        assert_eq!(fired, vec![(1, vec![99])]);
    }

    #[test]
    fn test_check_returns_multiple_fired_rules_in_key_order() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(5, Trigger::new(true, vec![50]));
        ts.insert(1, Trigger::new(true, vec![10]));
        ts.insert(3, Trigger::new(true, vec![30]));
        let fired = ts.check(|c| *c);
        assert_eq!(
            fired,
            vec![(1, vec![10]), (3, vec![30]), (5, vec![50])],
            "fired rules are returned in ascending key order"
        );
    }

    #[test]
    fn test_check_evaluator_receives_the_rule_specific_condition() {
        let mut ts: TriggerSet<u32, u32, u32> = TriggerSet::new();
        ts.insert(1, Trigger::new(100, vec![1]));
        ts.insert(2, Trigger::new(200, vec![2]));
        // Only the rule whose condition equals 200 should fire.
        let fired = ts.check(|c| *c == 200);
        assert_eq!(fired, vec![(2, vec![2])]);
    }

    #[test]
    fn test_multiple_actions_per_rule_all_returned() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::new(true, vec![10, 20, 30]));
        let fired = ts.check(|c| *c);
        assert_eq!(fired, vec![(1, vec![10, 20, 30])]);
    }

    #[test]
    fn test_empty_actions_rule_fires_with_empty_list() {
        let mut ts: TriggerSet<u32, bool, u32> = TriggerSet::new();
        ts.insert(1, Trigger::new(true, Vec::<u32>::new()));
        let fired = ts.check(|c| *c);
        assert_eq!(fired, vec![(1, vec![])]);
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let mut a: TriggerSet<u32, u32, u32> = TriggerSet::new();
        a.insert(1, Trigger::new(100, vec![1]));
        a.insert(2, Trigger::once(200, vec![2]));

        let mut b: TriggerSet<u32, u32, u32> = TriggerSet::new();
        b.insert(2, Trigger::once(200, vec![2]));
        b.insert(1, Trigger::new(100, vec![1]));
        assert_eq!(hash_state(&a), hash_state(&b), "insertion order does not affect the hash");

        a.check(|c| *c == 200); // fires the once-trigger at key 2
        assert_ne!(hash_state(&a), hash_state(&b), "fired-state changes the hash");
    }

    #[test]
    fn test_det_hash_sensitive_to_actions() {
        let mut a: TriggerSet<u32, u32, u32> = TriggerSet::new();
        a.insert(1, Trigger::new(100, vec![1, 2]));
        let mut b: TriggerSet<u32, u32, u32> = TriggerSet::new();
        b.insert(1, Trigger::new(100, vec![1, 3]));
        assert_ne!(hash_state(&a), hash_state(&b), "different actions → different hash");
    }
}

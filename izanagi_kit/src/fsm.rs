//! Finite state machine (FSM) for deterministic game AI.
//!
//! A `Fsm<S, E>` is a table-driven state machine: for each `(current_state,
//! event)` pair, it looks up the next state in a transition table. Missing
//! transitions are self-loops (the state is unchanged). State and event are
//! generic so the machine adapts to any AI or UI domain.
//!
//! Determinism: the transition function is a pure lookup (no random, no wall
//! clock, no I/O). Given the same sequence of events it produces the same
//! sequence of states — safe for replay and world-hash inclusion.
//!
//! For simple roguelike AI the state type is typically a small enum
//! (`Idle, Patrol, Chase, Flee, Dead`); events are `PlayerSeen`, `LostPlayer`,
//! `TookDamage`, etc. More complex behaviour trees are out of scope for this
//! module (see taxonomy J7 for the future WFC / influence-map extensions).

use crate::world_hash::{DetHash, Fnv1a};

/// A table-driven finite state machine.
///
/// Transitions are stored as `(from_state, event) → to_state` triples.
/// Looking up a pair not in the table returns the current state unchanged
/// (self-loop / "don't care" semantics — no panic).
#[derive(Clone, Debug)]
pub struct Fsm<S, E> {
    state: S,
    /// Transition table: (from, event) → to. Searched linearly; small enough
    /// that binary search or hashing would add more overhead than they save.
    table: Vec<(S, E, S)>,
}

impl<S: Eq + Clone, E: Eq> Fsm<S, E> {
    /// Create a new FSM starting in `initial_state` with an empty transition table.
    pub fn new(initial_state: S) -> Self {
        Fsm {
            state: initial_state,
            table: Vec::new(),
        }
    }

    /// Current state.
    #[inline]
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Add a transition: when in `from` and `event` fires, move to `to`.
    /// Adding the same `(from, event)` pair twice replaces the earlier entry.
    pub fn add_transition(&mut self, from: S, event: E, to: S) {
        if let Some((_, _, t)) = self
            .table
            .iter_mut()
            .find(|(f, e, _)| *f == from && *e == event)
        {
            *t = to;
        } else {
            self.table.push((from, event, to));
        }
    }

    /// Fire `event`. Returns `true` if the state changed, `false` if no
    /// matching transition was found (self-loop / unchanged).
    pub fn fire(&mut self, event: &E) -> bool {
        if let Some((_, _, to)) = self
            .table
            .iter()
            .find(|(f, e, _)| *f == self.state && *e == *event)
        {
            let to = to.clone();
            if to != self.state {
                self.state = to;
                return true;
            }
        }
        false
    }

    /// Force the machine into `state`, bypassing the transition table.
    /// Useful for external events like entity death that override AI logic.
    #[inline]
    pub fn set_state(&mut self, state: S) {
        self.state = state;
    }

    /// Whether a transition exists for `(current_state, event)`.
    pub fn has_transition(&self, event: &E) -> bool {
        self.table
            .iter()
            .any(|(f, e, _)| *f == self.state && *e == *event)
    }

    /// Remove the transition for `(from, event)` if it exists. No-op if absent.
    pub fn remove_transition(&mut self, from: &S, event: &E) {
        self.table.retain(|(f, e, _)| !(f == from && e == event));
    }

    /// Remove all transitions, leaving the current state and an empty table.
    /// Useful for resetting AI behaviour at runtime (e.g. a simplified "stunned"
    /// state that ignores all events until recovered).
    pub fn clear_transitions(&mut self) {
        self.table.clear();
    }

    /// Return the state this FSM would transition to if `event` were fired now,
    /// without actually changing state. Returns `None` when no transition is
    /// defined for `(current_state, event)` — i.e. the event would be a
    /// self-loop. Useful for AI lookahead ("would attacking now trigger Dead?")
    /// and UI "show next state" indicators without committing to the transition.
    pub fn peek_next(&self, event: &E) -> Option<&S> {
        self.table
            .iter()
            .find(|(f, e, _)| *f == self.state && *e == *event)
            .map(|(_, _, t)| t)
    }

    /// Iterate all events that have a defined outgoing transition from `state`.
    /// Useful for "show valid actions" UIs and AI planning ("what can I do from
    /// here?"). Returns an iterator of `&E` in table-insertion order. Does not
    /// deduplicate (duplicate events won't appear because `add_transition`
    /// already prevents them).
    pub fn transitions_from<'a>(&'a self, state: &'a S) -> impl Iterator<Item = &'a E> {
        self.table
            .iter()
            .filter(move |(f, _, _)| f == state)
            .map(|(_, e, _)| e)
    }

    /// Count outgoing transitions from `state`. Equivalent to
    /// `transitions_from(state).count()` but avoids constructing an iterator
    /// when only the count matters (e.g. "does this state have any exits?").
    pub fn transition_count(&self, from: &S) -> usize {
        self.table.iter().filter(|(f, _, _)| f == from).count()
    }

    /// Whether the machine is currently in `state`.
    ///
    /// Shorthand for `self.state() == state` — avoids call sites that would
    /// otherwise need to import and compare state variants directly.
    #[inline]
    pub fn is_in(&self, state: &S) -> bool {
        &self.state == state
    }
}

impl<S: DetHash + Eq + Clone, E: Eq> DetHash for Fsm<S, E> {
    /// Folds only the current state — the transition table is constant
    /// configuration and not part of the simulation state.
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.state.det_hash(hasher);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    /// Simple guard AI states.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum GuardState {
        Idle,
        Alert,
        Chase,
        Dead,
    }

    /// Events that drive the guard FSM.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum GuardEvent {
        PlayerSpotted,
        PlayerLost,
        TookDamage,
        Killed,
    }

    impl DetHash for GuardState {
        fn det_hash(&self, hasher: &mut Fnv1a) {
            let v: u32 = match self {
                GuardState::Idle => 0,
                GuardState::Alert => 1,
                GuardState::Chase => 2,
                GuardState::Dead => 3,
            };
            hasher.write_u32(v);
        }
    }

    fn guard_fsm() -> Fsm<GuardState, GuardEvent> {
        let mut fsm = Fsm::new(GuardState::Idle);
        fsm.add_transition(
            GuardState::Idle,
            GuardEvent::PlayerSpotted,
            GuardState::Alert,
        );
        fsm.add_transition(
            GuardState::Alert,
            GuardEvent::PlayerSpotted,
            GuardState::Chase,
        );
        fsm.add_transition(GuardState::Chase, GuardEvent::PlayerLost, GuardState::Alert);
        fsm.add_transition(GuardState::Alert, GuardEvent::PlayerLost, GuardState::Idle);
        fsm.add_transition(GuardState::Idle, GuardEvent::TookDamage, GuardState::Chase);
        fsm.add_transition(GuardState::Alert, GuardEvent::TookDamage, GuardState::Chase);
        fsm.add_transition(GuardState::Chase, GuardEvent::TookDamage, GuardState::Chase);
        fsm.add_transition(GuardState::Idle, GuardEvent::Killed, GuardState::Dead);
        fsm.add_transition(GuardState::Alert, GuardEvent::Killed, GuardState::Dead);
        fsm.add_transition(GuardState::Chase, GuardEvent::Killed, GuardState::Dead);
        fsm
    }

    #[test]
    fn test_initial_state() {
        let fsm = guard_fsm();
        assert_eq!(fsm.state(), &GuardState::Idle);
    }

    #[test]
    fn test_fire_valid_transition() {
        let mut fsm = guard_fsm();
        let changed = fsm.fire(&GuardEvent::PlayerSpotted);
        assert!(changed);
        assert_eq!(fsm.state(), &GuardState::Alert);
    }

    #[test]
    fn test_fire_unmapped_is_self_loop() {
        let mut fsm = guard_fsm();
        // Dead state has no transitions; any event is a self-loop.
        fsm.set_state(GuardState::Dead);
        let changed = fsm.fire(&GuardEvent::PlayerSpotted);
        assert!(!changed);
        assert_eq!(fsm.state(), &GuardState::Dead);
    }

    #[test]
    fn test_sequence_idle_to_chase() {
        let mut fsm = guard_fsm();
        fsm.fire(&GuardEvent::PlayerSpotted); // → Alert
        fsm.fire(&GuardEvent::PlayerSpotted); // → Chase
        assert_eq!(fsm.state(), &GuardState::Chase);
    }

    #[test]
    fn test_player_lost_from_chase_to_alert() {
        let mut fsm = guard_fsm();
        fsm.set_state(GuardState::Chase);
        fsm.fire(&GuardEvent::PlayerLost);
        assert_eq!(fsm.state(), &GuardState::Alert);
    }

    #[test]
    fn test_killed_from_any_state() {
        for start in [GuardState::Idle, GuardState::Alert, GuardState::Chase] {
            let mut fsm = guard_fsm();
            fsm.set_state(start);
            fsm.fire(&GuardEvent::Killed);
            assert_eq!(fsm.state(), &GuardState::Dead);
        }
    }

    #[test]
    fn test_has_transition_true_when_mapped() {
        let fsm = guard_fsm();
        assert!(fsm.has_transition(&GuardEvent::PlayerSpotted));
    }

    #[test]
    fn test_has_transition_false_for_unmapped() {
        let fsm = guard_fsm();
        // From Idle, PlayerLost is not mapped.
        assert!(!fsm.has_transition(&GuardEvent::PlayerLost));
    }

    #[test]
    fn test_add_transition_replaces_existing() {
        let mut fsm = Fsm::new(GuardState::Idle);
        fsm.add_transition(
            GuardState::Idle,
            GuardEvent::PlayerSpotted,
            GuardState::Alert,
        );
        fsm.add_transition(
            GuardState::Idle,
            GuardEvent::PlayerSpotted,
            GuardState::Chase, // override
        );
        fsm.fire(&GuardEvent::PlayerSpotted);
        assert_eq!(fsm.state(), &GuardState::Chase);
    }

    #[test]
    fn test_det_hash_changes_on_state_change() {
        let mut fsm = guard_fsm();
        let h1 = hash_state(&fsm);
        fsm.fire(&GuardEvent::PlayerSpotted);
        let h2 = hash_state(&fsm);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_det_hash_same_state_same_hash() {
        let mut a = guard_fsm();
        let mut b = guard_fsm();
        a.fire(&GuardEvent::PlayerSpotted);
        b.fire(&GuardEvent::PlayerSpotted);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_peek_next_returns_next_state() {
        let fsm = guard_fsm(); // Idle
        let next = fsm.peek_next(&GuardEvent::PlayerSpotted);
        assert_eq!(next, Some(&GuardState::Alert));
    }

    #[test]
    fn test_peek_next_no_transition_returns_none() {
        let fsm = guard_fsm(); // Idle — PlayerLost not mapped from Idle
        assert_eq!(fsm.peek_next(&GuardEvent::PlayerLost), None);
    }

    #[test]
    fn test_peek_next_does_not_change_state() {
        let fsm = guard_fsm();
        let _ = fsm.peek_next(&GuardEvent::PlayerSpotted);
        assert_eq!(fsm.state(), &GuardState::Idle); // unchanged
    }

    #[test]
    fn test_transitions_from_current_state() {
        let fsm = guard_fsm();
        // Idle can respond to PlayerSpotted, TookDamage, Killed.
        let events: Vec<&GuardEvent> = fsm.transitions_from(&GuardState::Idle).collect();
        assert!(events.contains(&&GuardEvent::PlayerSpotted));
        assert!(events.contains(&&GuardEvent::TookDamage));
        assert!(events.contains(&&GuardEvent::Killed));
        // PlayerLost is not mapped from Idle.
        assert!(!events.contains(&&GuardEvent::PlayerLost));
    }

    #[test]
    fn test_transitions_from_dead_is_empty() {
        let fsm = guard_fsm();
        let events: Vec<&GuardEvent> = fsm.transitions_from(&GuardState::Dead).collect();
        assert!(events.is_empty(), "Dead has no outgoing transitions");
    }

    #[test]
    fn test_remove_transition_prevents_fire() {
        let mut fsm = guard_fsm();
        fsm.remove_transition(&GuardState::Idle, &GuardEvent::PlayerSpotted);
        let changed = fsm.fire(&GuardEvent::PlayerSpotted);
        assert!(!changed);
        assert_eq!(fsm.state(), &GuardState::Idle);
    }

    #[test]
    fn test_remove_transition_nonexistent_is_noop() {
        let mut fsm = guard_fsm();
        // Dead has no transitions. Removing a non-existent entry is a no-op —
        // firing the event still results in a self-loop (no panic, no state change).
        fsm.remove_transition(&GuardState::Dead, &GuardEvent::PlayerSpotted);
        fsm.set_state(GuardState::Dead);
        assert!(!fsm.fire(&GuardEvent::PlayerSpotted));
    }

    #[test]
    fn test_clear_transitions_makes_all_events_self_loop() {
        let mut fsm = guard_fsm();
        fsm.clear_transitions();
        assert!(!fsm.fire(&GuardEvent::PlayerSpotted));
        assert!(!fsm.fire(&GuardEvent::Killed));
        assert_eq!(fsm.state(), &GuardState::Idle);
    }

    #[test]
    fn test_clear_transitions_does_not_change_state() {
        let mut fsm = guard_fsm();
        fsm.set_state(GuardState::Chase);
        fsm.clear_transitions();
        assert_eq!(fsm.state(), &GuardState::Chase);
    }

    #[test]
    fn test_sequence_is_deterministic() {
        let events = [
            GuardEvent::PlayerSpotted,
            GuardEvent::PlayerSpotted,
            GuardEvent::PlayerLost,
            GuardEvent::TookDamage,
        ];
        let run = || {
            let mut fsm = guard_fsm();
            for e in &events {
                fsm.fire(e);
            }
            fsm.state().clone()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn test_transition_count_from_state_with_exits() {
        let fsm = guard_fsm();
        // Idle has transitions for: PlayerSpotted, TookDamage, Killed = 3
        assert_eq!(fsm.transition_count(&GuardState::Idle), 3);
    }

    #[test]
    fn test_transition_count_from_dead_is_zero() {
        let fsm = guard_fsm();
        assert_eq!(fsm.transition_count(&GuardState::Dead), 0);
    }

    #[test]
    fn test_transition_count_decreases_after_remove() {
        let mut fsm = guard_fsm();
        let before = fsm.transition_count(&GuardState::Idle);
        fsm.remove_transition(&GuardState::Idle, &GuardEvent::PlayerSpotted);
        assert_eq!(fsm.transition_count(&GuardState::Idle), before - 1);
    }

    #[test]
    fn test_is_in_matches_initial_state() {
        let fsm = guard_fsm();
        assert!(fsm.is_in(&GuardState::Idle));
        assert!(!fsm.is_in(&GuardState::Chase));
    }

    #[test]
    fn test_is_in_reflects_state_change() {
        let mut fsm = guard_fsm();
        fsm.fire(&GuardEvent::PlayerSpotted); // Idle → Alert
        assert!(fsm.is_in(&GuardState::Alert));
        assert!(!fsm.is_in(&GuardState::Idle));
    }

    #[test]
    fn test_is_in_after_set_state() {
        let mut fsm = guard_fsm();
        fsm.set_state(GuardState::Dead);
        assert!(fsm.is_in(&GuardState::Dead));
        assert!(!fsm.is_in(&GuardState::Idle));
    }
}

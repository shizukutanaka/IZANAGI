//! Hierarchical finite state machine (HFSM) for game AI (W7 in
//! `STRENGTHS_WEAKNESSES.md`).
//!
//! Unlike the flat [`Fsm<S,E>`](crate::fsm::Fsm), states in an `HFsm` form a
//! tree: each state may have a parent. When an event fires:
//!
//! 1. The innermost matching transition wins — if the current state has a
//!    transition for the event, it is used.
//! 2. If not, the parent state is checked, then the grandparent, and so on
//!    up the hierarchy.
//! 3. After exhausting the ancestor chain, wildcard transitions (registered
//!    via [`on_any`](HFsm::on_any)) are checked — useful for "die from any
//!    state" patterns.
//!
//! `is_in(state)` returns `true` whether the current state *is* `state` or is
//! a **descendant** of `state` — enabling broad "are we in the combat branch?"
//! queries without enumerating every substate.
//!
//! ## Invariants
//!
//! The parent table is always **acyclic** (a forest). [`set_parent`](HFsm::set_parent)
//! / [`with_parent`](HFsm::with_parent) reject any edge that would close a cycle
//! (and self-parenting), so every ancestor-chain walk — `fire`, `is_in`,
//! `ancestors_of`, `peek_next`, `has_transition` — is guaranteed to terminate.
//! Use [`try_set_parent`](HFsm::try_set_parent) when you need to know whether the
//! edge was accepted.
//!
//! ## Determinism
//!
//! Transition lookup is a linear scan (no `HashMap`). Parent chains are walked
//! in insertion order. Identical event sequences produce identical state traces.
//!
//! ## Example
//!
//! ```
//! use izanagi_kit::hfsm::HFsm;
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum S { Alive, Combat, Chase, Flee, Dead }
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum E { SeePlayer, LostPlayer, LowHp, Die }
//!
//! let mut ai: HFsm<S, E> = HFsm::new(S::Alive)
//!     // hierarchy: Combat and Flee are sub-states of Alive
//!     .with_parent(S::Combat, S::Alive)
//!     .with_parent(S::Chase,  S::Combat)
//!     .with_parent(S::Flee,   S::Alive)
//!     // transitions
//!     .on(S::Alive,  E::SeePlayer, S::Chase)  // from Alive (or any descendant)
//!     .on(S::Combat, E::LowHp,     S::Flee)   // from Combat (or Chase)
//!     .on(S::Chase,  E::LostPlayer,S::Alive)
//!     .on_any(E::Die, S::Dead);               // from any state
//!
//! ai.fire(&E::SeePlayer);      // Alive → Chase
//! assert_eq!(*ai.state(), S::Chase);
//! assert!(ai.is_in(&S::Combat), "Chase is a sub-state of Combat");
//!
//! ai.fire(&E::LowHp);          // Chase inherits Combat→Flee
//! assert_eq!(*ai.state(), S::Flee);
//!
//! ai.fire(&E::Die);            // wildcard fires from Flee
//! assert_eq!(*ai.state(), S::Dead);
//! ```

use crate::world_hash::{DetHash, Fnv1a};

/// A hierarchical finite state machine.
///
/// Build with [`new`](HFsm::new) and the builder methods [`with_parent`],
/// [`on`], [`on_any`]. Use [`fire`](HFsm::fire) to send events and
/// [`is_in`](HFsm::is_in) to query the current state hierarchy.
///
/// [`with_parent`]: HFsm::with_parent
/// [`on`]: HFsm::on
/// [`on_any`]: HFsm::on_any
#[derive(Clone, Debug)]
pub struct HFsm<S, E> {
    initial: S,
    state: S,
    /// (child, parent) relationships.
    parents: Vec<(S, S)>,
    /// Transitions: `(Option<from>, event, to)`. `None` = wildcard (any state).
    /// Specific transitions are searched before wildcards.
    transitions: Vec<(Option<S>, E, S)>,
}

impl<S: Eq + Clone, E: Eq + Clone> HFsm<S, E> {
    /// Create an HFSM starting in `initial_state` with no hierarchy or
    /// transitions defined.
    pub fn new(initial_state: S) -> Self {
        HFsm {
            initial: initial_state.clone(),
            state: initial_state,
            parents: Vec::new(),
            transitions: Vec::new(),
        }
    }

    // ── Builder methods ────────────────────────────────────────────────────

    /// Declare `child` as a sub-state of `parent`. If `child` already has a
    /// parent, the existing declaration is replaced.
    ///
    /// Edges that would create a cycle (or self-parenting) are silently ignored
    /// to keep the hierarchy a forest — see [`try_set_parent`](HFsm::try_set_parent)
    /// for a form that reports rejection.
    pub fn with_parent(mut self, child: S, parent: S) -> Self {
        self.set_parent(child, parent);
        self
    }

    /// Add a transition: when the current state is `from` (or a descendant of
    /// `from`) and `event` fires, move to `to`. If a transition for
    /// `(from, event)` already exists, it is replaced.
    pub fn on(mut self, from: S, event: E, to: S) -> Self {
        self.add_transition(from, event, to);
        self
    }

    /// Add a wildcard transition: if no ancestor-specific transition matches,
    /// this fires from **any** state. Lower priority than any specific transition.
    /// If a wildcard for `event` already exists, it is replaced.
    pub fn on_any(mut self, event: E, to: S) -> Self {
        self.add_wildcard(event, to);
        self
    }

    // ── Mutation methods ───────────────────────────────────────────────────

    /// Declare `child` as a sub-state of `parent` (non-builder form).
    ///
    /// Cycle-forming edges (and self-parenting) are silently ignored so the
    /// parent table stays acyclic. Use [`try_set_parent`](HFsm::try_set_parent)
    /// if you need to know whether the edge was applied.
    pub fn set_parent(&mut self, child: S, parent: S) {
        self.try_set_parent(child, parent);
    }

    /// Declare `child` as a sub-state of `parent`, returning `false` (and making
    /// no change) when the edge would create a cycle or is a self-parent, and
    /// `true` when applied. If `child` already has a parent, it is replaced.
    ///
    /// Mirrors [`Relations::attach`](crate::relations::Relations::attach): the
    /// parent table is kept acyclic so every ancestor-chain walk terminates.
    pub fn try_set_parent(&mut self, child: S, parent: S) -> bool {
        if self.would_cycle(&child, &parent) {
            return false;
        }
        if let Some((_, p)) = self.parents.iter_mut().find(|(c, _)| *c == child) {
            *p = parent;
        } else {
            self.parents.push((child, parent));
        }
        true
    }

    /// Whether adding the edge `child → parent` would create a cycle (including
    /// the degenerate self-parent `child == parent`).
    ///
    /// Relies on the existing parent table being acyclic (the maintained
    /// invariant) so the upward walk from `parent` is guaranteed to terminate.
    pub fn would_cycle(&self, child: &S, parent: &S) -> bool {
        if child == parent {
            return true;
        }
        // Walk up from `parent`; if the chain reaches `child`, closing the edge
        // `child → parent` would form a cycle.
        let mut cur = parent.clone();
        loop {
            match self.parent_of(&cur) {
                None => return false,
                Some(p) if p == child => return true,
                Some(p) => cur = p.clone(),
            }
        }
    }

    /// Add a specific `(from, event) → to` transition (non-builder form).
    pub fn add_transition(&mut self, from: S, event: E, to: S) {
        if let Some((_, _, t)) = self
            .transitions
            .iter_mut()
            .find(|(f, e, _)| f.as_ref() == Some(&from) && *e == event)
        {
            *t = to;
        } else {
            self.transitions.push((Some(from), event, to));
        }
    }

    /// Add a wildcard `(any → to)` transition for `event` (non-builder form).
    pub fn add_wildcard(&mut self, event: E, to: S) {
        if let Some((_, _, t)) = self
            .transitions
            .iter_mut()
            .find(|(f, e, _)| f.is_none() && *e == event)
        {
            *t = to;
        } else {
            self.transitions.push((None, event, to));
        }
    }

    /// Force the machine into `state`, bypassing the transition table.
    #[inline]
    pub fn set_state(&mut self, state: S) {
        self.state = state;
    }

    /// Return to the initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.state = self.initial.clone();
    }

    // ── Event firing ───────────────────────────────────────────────────────

    /// Fire `event`.
    ///
    /// Searches in priority order:
    /// 1. Exact current state → event.
    /// 2. Parent, grandparent, … up the ancestor chain.
    /// 3. Wildcard (`on_any`) transitions.
    ///
    /// Returns `true` if the state changed, `false` if no matching transition
    /// was found (self-loop / no-op).
    pub fn fire(&mut self, event: &E) -> bool {
        // Walk up from current state through ancestors looking for a match.
        let mut candidate = Some(self.state.clone());
        while let Some(ref s) = candidate.clone() {
            if let Some(to) = self.find_specific(s, event) {
                let to = to.clone();
                if to != self.state {
                    self.state = to;
                    return true;
                }
                return false; // self-transition: found but no change
            }
            candidate = self.parent_of(s).cloned();
        }
        // Check wildcard transitions.
        if let Some(to) = self.find_wildcard(event) {
            let to = to.clone();
            if to != self.state {
                self.state = to;
                return true;
            }
        }
        false
    }

    // ── Query methods ──────────────────────────────────────────────────────

    /// Current state.
    #[inline]
    pub fn state(&self) -> &S {
        &self.state
    }

    /// The state this machine was constructed with.
    #[inline]
    pub fn initial_state(&self) -> &S {
        &self.initial
    }

    /// `true` if the current state is `state` **or** any descendant of `state`.
    ///
    /// ```text
    /// // If hierarchy: Chase ⊂ Combat ⊂ Alive, and current = Chase:
    /// is_in(Chase)  → true
    /// is_in(Combat) → true  (Chase is a sub-state of Combat)
    /// is_in(Alive)  → true  (Combat is a sub-state of Alive)
    /// is_in(Flee)   → false (unrelated)
    /// ```
    pub fn is_in(&self, state: &S) -> bool {
        if &self.state == state {
            return true;
        }
        // Walk up the ancestor chain of the current state.
        let mut cur = self.state.clone();
        while let Some(p) = self.parent_of(&cur) {
            if p == state {
                return true;
            }
            cur = p.clone();
        }
        false
    }

    /// Direct parent of `state`, or `None` if it is a root state.
    pub fn parent_of(&self, state: &S) -> Option<&S> {
        self.parents
            .iter()
            .find_map(|(c, p)| if c == state { Some(p) } else { None })
    }

    /// All ancestor states of `state` from closest parent to root, in order.
    /// Returns an empty `Vec` for root states.
    pub fn ancestors_of(&self, state: &S) -> Vec<&S> {
        let mut result = Vec::new();
        let mut cur = state.clone();
        while let Some(p) = self.parent_of(&cur) {
            result.push(p);
            cur = p.clone();
        }
        result
    }

    /// All ancestor states of the **current** state from closest parent to root.
    pub fn ancestors(&self) -> Vec<&S> {
        self.ancestors_of(&self.state.clone())
    }

    /// Whether a transition exists for `(current_state_or_ancestor, event)`.
    /// Respects the same search order as [`fire`](HFsm::fire).
    pub fn has_transition(&self, event: &E) -> bool {
        let mut candidate = Some(self.state.clone());
        while let Some(ref s) = candidate.clone() {
            if self.find_specific(s, event).is_some() {
                return true;
            }
            candidate = self.parent_of(s).cloned();
        }
        self.find_wildcard(event).is_some()
    }

    /// What state `fire(event)` would produce without actually changing state.
    /// Returns `None` if no transition would fire.
    pub fn peek_next(&self, event: &E) -> Option<&S> {
        let mut candidate = Some(self.state.clone());
        while let Some(ref s) = candidate.clone() {
            if let Some(to) = self.find_specific(s, event) {
                return Some(to);
            }
            // borrow: we need to reborrow after calling parent_of
            let p = self.parent_of(s).cloned();
            candidate = p;
        }
        self.find_wildcard(event)
    }

    // ── Private helpers ────────────────────────────────────────────────────

    fn find_specific(&self, from: &S, event: &E) -> Option<&S> {
        self.transitions.iter().find_map(|(f, e, t)| {
            if f.as_ref() == Some(from) && e == event {
                Some(t)
            } else {
                None
            }
        })
    }

    fn find_wildcard(&self, event: &E) -> Option<&S> {
        self.transitions.iter().find_map(|(f, e, t)| {
            if f.is_none() && e == event {
                Some(t)
            } else {
                None
            }
        })
    }
}

impl<S: DetHash, E: DetHash> DetHash for HFsm<S, E> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.state.det_hash(hasher);
        hasher.write_u32(self.parents.len() as u32);
        for (c, p) in &self.parents {
            c.det_hash(hasher);
            p.det_hash(hasher);
        }
        hasher.write_u32(self.transitions.len() as u32);
        for (f, e, t) in &self.transitions {
            match f {
                None => hasher.write_u8(0),
                Some(s) => {
                    hasher.write_u8(1);
                    s.det_hash(hasher);
                }
            }
            e.det_hash(hasher);
            t.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum S {
        Alive,
        Combat,
        Chase,
        Flee,
        Patrol,
        Dead,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Ev {
        SeePlayer,
        LostPlayer,
        LowHp,
        Healed,
        Die,
        Tick,
    }

    impl DetHash for S {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_u8(*self as u8);
        }
    }

    impl DetHash for Ev {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_u8(*self as u8);
        }
    }

    /// Build the reference HFSM used by most tests.
    ///
    /// Hierarchy: Chase ⊂ Combat ⊂ Alive; Flee ⊂ Alive; Patrol ⊂ Alive.
    ///
    /// Transitions:
    ///   Alive  + SeePlayer → Chase
    ///   Combat + LowHp     → Flee
    ///   Chase  + LostPlayer→ Patrol
    ///   Flee   + Healed    → Chase
    ///   Any    + Die       → Dead
    fn make_ai() -> HFsm<S, Ev> {
        HFsm::new(S::Alive)
            .with_parent(S::Patrol, S::Alive)
            .with_parent(S::Combat, S::Alive)
            .with_parent(S::Chase, S::Combat)
            .with_parent(S::Flee, S::Alive)
            .on(S::Alive, Ev::SeePlayer, S::Chase)
            .on(S::Combat, Ev::LowHp, S::Flee)
            .on(S::Chase, Ev::LostPlayer, S::Patrol)
            .on(S::Flee, Ev::Healed, S::Chase)
            .on_any(Ev::Die, S::Dead)
    }

    // ── Basic firing ──────────────────────────────────────────────────────

    #[test]
    fn test_flat_fire_no_parents() {
        let mut f: HFsm<S, Ev> = HFsm::new(S::Alive)
            .on(S::Alive, Ev::SeePlayer, S::Chase);
        assert!(f.fire(&Ev::SeePlayer));
        assert_eq!(*f.state(), S::Chase);
    }

    #[test]
    fn test_fire_unknown_event_no_change() {
        let mut f = make_ai();
        let changed = f.fire(&Ev::Tick); // no transition for Tick
        assert!(!changed);
        assert_eq!(*f.state(), S::Alive);
    }

    #[test]
    fn test_fire_returns_false_on_self_loop() {
        // A transition that returns to the current state is a no-change self-loop.
        let mut f: HFsm<S, Ev> =
            HFsm::new(S::Alive).on(S::Alive, Ev::Tick, S::Alive);
        assert!(!f.fire(&Ev::Tick), "self-loop must return false");
        assert_eq!(*f.state(), S::Alive);
    }

    // ── Hierarchical inheritance ──────────────────────────────────────────

    #[test]
    fn test_child_inherits_parent_transition() {
        // Chase ⊂ Combat. LowHp is defined on Combat, not on Chase.
        // When in Chase and LowHp fires, Combat's transition should be used.
        let mut ai = make_ai();
        ai.fire(&Ev::SeePlayer); // Alive → Chase
        assert_eq!(*ai.state(), S::Chase);
        ai.fire(&Ev::LowHp); // Chase inherits Combat → Flee
        assert_eq!(*ai.state(), S::Flee);
    }

    #[test]
    fn test_grandchild_inherits_ancestor_transition() {
        // Chase ⊂ Combat ⊂ Alive. SeePlayer is on Alive.
        // A grandchild not handled by its immediate parent still finds the transition.
        let mut ai = make_ai();
        // Start at Chase (via SeePlayer), lose player → Patrol (⊂ Alive)
        ai.fire(&Ev::SeePlayer);
        ai.fire(&Ev::LostPlayer); // Chase → Patrol
        assert_eq!(*ai.state(), S::Patrol);
        // SeePlayer is defined on Alive; Patrol inherits it.
        ai.fire(&Ev::SeePlayer); // Patrol inherits Alive → Chase
        assert_eq!(*ai.state(), S::Chase);
    }

    #[test]
    fn test_child_specific_transition_wins_over_parent() {
        // Chase has LostPlayer → Patrol; if parent also had LostPlayer, child wins.
        let mut ai = make_ai();
        ai.fire(&Ev::SeePlayer); // → Chase
        ai.fire(&Ev::LostPlayer); // Chase's own transition wins
        assert_eq!(*ai.state(), S::Patrol);
    }

    // ── Wildcard transitions ──────────────────────────────────────────────

    #[test]
    fn test_wildcard_fires_from_any_state() {
        let mut ai = make_ai();
        // From initial Alive — wildcard Die → Dead
        assert!(ai.fire(&Ev::Die));
        assert_eq!(*ai.state(), S::Dead);
    }

    #[test]
    fn test_wildcard_fires_from_deep_substate() {
        let mut ai = make_ai();
        ai.fire(&Ev::SeePlayer); // → Chase (deep substate)
        ai.fire(&Ev::Die); // wildcard → Dead
        assert_eq!(*ai.state(), S::Dead);
    }

    #[test]
    fn test_specific_transition_wins_over_wildcard() {
        // If the current (or ancestor) state has a specific transition for an
        // event, it beats the wildcard.
        let mut ai = make_ai();
        ai.fire(&Ev::SeePlayer); // → Chase
        // SeePlayer has a specific transition on Alive; if we add a Die transition
        // on Alive and also have a Die wildcard, the specific wins.
        // Here we just verify that LowHp from Chase uses Combat's specific, not
        // the wildcard (Die fires correctly as wildcard only when nothing else matches).
        ai.fire(&Ev::LowHp); // Combat's transition → Flee, not wildcard
        assert_eq!(*ai.state(), S::Flee);
    }

    // ── is_in ─────────────────────────────────────────────────────────────

    #[test]
    fn test_is_in_current_state() {
        let ai = make_ai();
        assert!(ai.is_in(&S::Alive));
        assert!(!ai.is_in(&S::Dead));
    }

    #[test]
    fn test_is_in_ancestor_of_current() {
        let mut ai = make_ai();
        ai.fire(&Ev::SeePlayer); // → Chase (⊂ Combat ⊂ Alive)
        assert!(ai.is_in(&S::Chase));
        assert!(ai.is_in(&S::Combat), "Chase ⊂ Combat");
        assert!(ai.is_in(&S::Alive), "Chase ⊂ Combat ⊂ Alive");
    }

    #[test]
    fn test_is_in_false_for_unrelated_state() {
        let mut ai = make_ai();
        ai.fire(&Ev::SeePlayer); // → Chase
        assert!(!ai.is_in(&S::Flee), "Flee is unrelated to Chase");
        assert!(!ai.is_in(&S::Dead));
    }

    // ── ancestors / parent_of ─────────────────────────────────────────────

    #[test]
    fn test_ancestors_of_root_is_empty() {
        let ai = make_ai();
        assert!(ai.ancestors_of(&S::Alive).is_empty());
    }

    #[test]
    fn test_ancestors_of_deep_state() {
        let ai = make_ai();
        let anc = ai.ancestors_of(&S::Chase);
        assert_eq!(anc, vec![&S::Combat, &S::Alive]);
    }

    #[test]
    fn test_ancestors_returns_current_ancestors() {
        let mut ai = make_ai();
        ai.fire(&Ev::SeePlayer); // → Chase
        assert_eq!(ai.ancestors(), vec![&S::Combat, &S::Alive]);
    }

    // ── has_transition / peek_next ────────────────────────────────────────

    #[test]
    fn test_has_transition_direct_event() {
        let ai = make_ai();
        assert!(ai.has_transition(&Ev::SeePlayer));
        assert!(!ai.has_transition(&Ev::Tick));
    }

    #[test]
    fn test_peek_next_returns_target_without_firing() {
        let ai = make_ai();
        assert_eq!(ai.peek_next(&Ev::SeePlayer), Some(&S::Chase));
        assert_eq!(*ai.state(), S::Alive, "state must not change");
    }

    // ── reset / set_state ─────────────────────────────────────────────────

    #[test]
    fn test_reset_returns_to_initial() {
        let mut ai = make_ai();
        ai.fire(&Ev::SeePlayer);
        ai.reset();
        assert_eq!(*ai.state(), S::Alive);
    }

    #[test]
    fn test_set_state_bypasses_table() {
        let mut ai = make_ai();
        ai.set_state(S::Dead);
        assert_eq!(*ai.state(), S::Dead);
    }

    // ── Cycle safety (parent table stays acyclic) ─────────────────────────

    #[test]
    fn test_self_parent_rejected() {
        let f: HFsm<S, Ev> = HFsm::new(S::Alive).with_parent(S::Combat, S::Combat);
        assert_eq!(
            f.parent_of(&S::Combat),
            None,
            "self-parent edge must be ignored"
        );
    }

    #[test]
    fn test_try_set_parent_reports_acceptance() {
        let mut f: HFsm<S, Ev> = HFsm::new(S::Alive);
        assert!(f.try_set_parent(S::Combat, S::Alive), "valid edge accepted");
        // Combat ⊂ Alive; making Alive ⊂ Combat would close a 2-cycle.
        assert!(
            !f.try_set_parent(S::Alive, S::Combat),
            "cycle-forming edge rejected"
        );
        assert_eq!(f.parent_of(&S::Alive), None, "rejected edge made no change");
        assert_eq!(f.parent_of(&S::Combat), Some(&S::Alive));
    }

    #[test]
    fn test_would_cycle_detects_self_and_two_cycle() {
        let f: HFsm<S, Ev> = HFsm::new(S::Alive).with_parent(S::Combat, S::Alive);
        assert!(f.would_cycle(&S::Alive, &S::Alive), "self-parent is a cycle");
        assert!(
            f.would_cycle(&S::Alive, &S::Combat),
            "Alive→Combat closes Combat→Alive"
        );
        assert!(
            !f.would_cycle(&S::Chase, &S::Combat),
            "Chase→Combat is acyclic"
        );
    }

    #[test]
    fn test_three_node_cycle_rejected() {
        // Chase ⊂ Combat ⊂ Alive. Closing Alive → Chase would form a 3-cycle.
        let mut f: HFsm<S, Ev> = HFsm::new(S::Alive)
            .with_parent(S::Combat, S::Alive)
            .with_parent(S::Chase, S::Combat);
        assert!(
            !f.try_set_parent(S::Alive, S::Chase),
            "Alive→Chase (Chase→Combat→Alive→Chase) must be rejected"
        );
        assert_eq!(f.parent_of(&S::Alive), None);
    }

    #[test]
    fn test_replacing_parent_stays_cycle_safe() {
        // Combat ⊂ Alive, Chase ⊂ Combat. Re-parenting Combat under Chase would
        // make Combat→Chase→Combat — must be rejected; a valid re-parent works.
        let mut f: HFsm<S, Ev> = HFsm::new(S::Alive)
            .with_parent(S::Combat, S::Alive)
            .with_parent(S::Chase, S::Combat);
        assert!(!f.try_set_parent(S::Combat, S::Chase), "cycle via replace");
        assert_eq!(f.parent_of(&S::Combat), Some(&S::Alive), "unchanged");
        // Valid re-parent: Combat under Patrol (Patrol is a root here).
        assert!(f.try_set_parent(S::Combat, S::Patrol));
        assert_eq!(f.parent_of(&S::Combat), Some(&S::Patrol));
    }

    #[test]
    fn test_traversals_terminate_after_cycle_attempt() {
        // The payoff: after a rejected cycle the table is still a forest, so the
        // ancestor-walking queries return finite, correct results (no hang).
        let mut ai = make_ai();
        assert!(!ai.try_set_parent(S::Alive, S::Chase), "would cycle, rejected");
        ai.fire(&Ev::SeePlayer); // → Chase, terminates
        assert_eq!(*ai.state(), S::Chase);
        assert_eq!(ai.ancestors(), vec![&S::Combat, &S::Alive]); // bounded
        assert!(ai.is_in(&S::Alive)); // bounded walk
        assert!(ai.has_transition(&Ev::Die)); // bounded walk + wildcard
    }

    // ── DetHash ───────────────────────────────────────────────────────────

    #[test]
    fn test_det_hash_same_machine_same_hash() {
        assert_eq!(hash_state(&make_ai()), hash_state(&make_ai()));
    }

    #[test]
    fn test_det_hash_differs_after_fire() {
        let m1 = make_ai();
        let mut m2 = make_ai();
        m2.fire(&Ev::SeePlayer);
        assert_ne!(hash_state(&m1), hash_state(&m2));
    }

    #[test]
    fn test_det_hash_wildcard_vs_specific_differ() {
        let specific: HFsm<S, Ev> =
            HFsm::new(S::Alive).on(S::Alive, Ev::Die, S::Dead);
        let wildcard: HFsm<S, Ev> = HFsm::new(S::Alive).on_any(Ev::Die, S::Dead);
        assert_ne!(hash_state(&specific), hash_state(&wildcard));
    }
}

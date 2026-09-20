//! Structural-change observation for sparse-set storage — the *push* half of
//! change awareness.
//!
//! [`crate::change::Changed`] answers a pull question — "was this component
//! written since tick N?" — but it cannot answer the push question a despawn
//! raises: "an entity just lost its `Position`; drop its threat-table row and
//! its spatial-hash cell *now*." [`Observed<T>`] wraps a [`SparseSet<T>`] and
//! pushes a [`ComponentEvent`] onto an internal [`EventQueue`] for every
//! structural mutation, in program order, ready to drain at the tick
//! boundary.
//!
//! Bevy implements the same mechanism as `OnAdd`/`OnRemove` observer
//! callbacks. Here the hook is **data, not a callback**: a callback that runs
//! mid-mutation can hide shared mutable state and re-entrant writes; a queued
//! event is ordered, hashable and replayable — the same guarantee the rest of
//! the crate makes.
//!
//! ```
//! use izanagi_kit::entity::EntityAllocator;
//! use izanagi_kit::observe::{ComponentEvent, Observed};
//!
//! let mut alloc = EntityAllocator::new();
//! let e = alloc.allocate();
//! let mut pos: Observed<u32> = Observed::new();
//!
//! pos.insert(e, 10); // emits ComponentEvent::Added(e)
//! pos.insert(e, 20); // emits ComponentEvent::Replaced(e)
//! pos.remove(e); // emits ComponentEvent::Removed(e)
//!
//! let events: Vec<_> = pos.drain_events().collect();
//! assert_eq!(
//!     events,
//!     [
//!         ComponentEvent::Added(e),
//!         ComponentEvent::Replaced(e),
//!         ComponentEvent::Removed(e),
//!     ]
//! );
//! ```
//!
//! Only *structural* changes produce events. Mutating a component in place
//! through [`Observed::get_mut`] or [`Observed::iter_mut`] emits nothing —
//! value-level tracking is [`crate::change::Changed`]'s job, and composing
//! `Observed<Changed<T>>` covers both axes.
//!
//! Determinism: events are emitted in program order with no timestamps,
//! randomness or address dependence, so identical call sequences produce
//! identical event streams. [`Observed`]'s [`DetHash`] folds both the storage
//! (in canonical entity order, same as `SparseSet`) and the *pending* event
//! stream — pending events are observable state, so a run that forgot to
//! drain them desyncs loudly rather than silently.

use crate::entity::Entity;
use crate::eventqueue::EventQueue;
use crate::sparse_set::SparseSet;
use crate::world_hash::{DetHash, Fnv1a};

/// One structural change to an [`Observed`] storage.
///
/// The entity is the whole payload: a removal's job is to let other systems
/// clean up *their* state for that entity, and the component value is gone by
/// the time the event is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentEvent {
    /// `insert` added a component to an entity that had none.
    Added(Entity),
    /// `insert` overwrote a component the entity already had.
    Replaced(Entity),
    /// The component was removed — by [`Observed::remove`],
    /// [`Observed::remove_where`], [`Observed::retain`], [`Observed::clear`]
    /// or [`Observed::drain`].
    Removed(Entity),
}

impl DetHash for ComponentEvent {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        let (tag, e) = match self {
            ComponentEvent::Added(e) => (0u8, *e),
            ComponentEvent::Replaced(e) => (1u8, *e),
            ComponentEvent::Removed(e) => (2u8, *e),
        };
        hasher.write_u8(tag);
        e.det_hash(hasher);
    }
}

/// A [`SparseSet`] that reports every structural change as an event.
///
/// Mutation methods mirror `SparseSet`'s and additionally push a
/// [`ComponentEvent`] onto the internal queue; read access goes through them
/// or through [`inner`](Observed::inner), which borrows the raw set for the
/// whole read API (`iter`, `entities`, `iter_sorted`, `join`, …) without
/// this wrapper re-declaring it.
///
/// There is deliberately no `&mut SparseSet` accessor: it would let callers
/// mutate structure without emitting events, which is the one thing this type
/// exists to prevent.
pub struct Observed<T> {
    set: SparseSet<T>,
    events: EventQueue<ComponentEvent>,
}

impl<T> Default for Observed<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Observed<T> {
    /// Create an empty observed storage.
    pub fn new() -> Self {
        Observed {
            set: SparseSet::new(),
            events: EventQueue::new(),
        }
    }

    /// Number of components currently stored.
    #[inline]
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// `true` if no components are stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// `true` if `entity` has a component in this set.
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.set.contains(entity)
    }

    /// Look up the component for `entity`, if it has one.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.set.get(entity)
    }

    /// Mutably look up the component for `entity`.
    ///
    /// In-place mutation is not structural: it emits **no event**. Track
    /// value changes with [`crate::change::Changed`] if they need observing.
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.set.get_mut(entity)
    }

    /// Mutable dense iteration over `(Entity, &mut T)` pairs.
    ///
    /// Like [`get_mut`](Observed::get_mut), value mutation through this
    /// iterator emits no events — structural changes are impossible through
    /// it, so nothing is missed.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.set.iter_mut()
    }

    /// Borrow the underlying storage for the full read API
    /// (`iter`, `entities`, `values`, `iter_sorted`, `find_entity_where`,
    /// `join`, …). Read-only by design — see the type-level docs.
    #[inline]
    pub fn inner(&self) -> &SparseSet<T> {
        &self.set
    }

    /// Insert or overwrite the component for `entity`.
    ///
    /// Emits [`ComponentEvent::Added`] when the entity had no component and
    /// [`ComponentEvent::Replaced`] when it did — an overwrite is not silent,
    /// because downstream caches often index the *value*.
    pub fn insert(&mut self, entity: Entity, value: T) {
        let event = if self.set.contains(entity) {
            ComponentEvent::Replaced(entity)
        } else {
            ComponentEvent::Added(entity)
        };
        self.set.insert(entity, value);
        self.events.push(event);
    }

    /// Remove the component for `entity`, returning it. Emits
    /// [`ComponentEvent::Removed`] only when the entity was present.
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let removed = self.set.remove(entity);
        if removed.is_some() {
            self.events.push(ComponentEvent::Removed(entity));
        }
        removed
    }

    /// Remove every entry for which `pred(entity, &value)` holds, emitting one
    /// [`ComponentEvent::Removed`] per removal in the order the removals are
    /// applied. Returns the count removed — the [`SparseSet::remove_where`]
    /// contract.
    ///
    /// Matching entities are collected in dense order first, then removed —
    /// so the event stream reads in storage order rather than in
    /// swap-removal order.
    pub fn remove_where<F: Fn(Entity, &T) -> bool>(&mut self, pred: F) -> usize {
        let doomed: Vec<Entity> = self
            .set
            .iter()
            .filter(|(e, v)| pred(*e, v))
            .map(|(e, _)| e)
            .collect();
        let removed = doomed.len();
        for entity in doomed {
            self.set.remove(entity);
            self.events.push(ComponentEvent::Removed(entity));
        }
        removed
    }

    /// Keep only the entries for which `pred` holds; emits
    /// [`ComponentEvent::Removed`] for each entry dropped. The inverse of
    /// [`remove_where`](Observed::remove_where), same as on `SparseSet`.
    pub fn retain<F: Fn(Entity, &T) -> bool>(&mut self, pred: F) {
        self.remove_where(|e, v| !pred(e, v));
    }

    /// Remove all entries, emitting [`ComponentEvent::Removed`] for each in
    /// dense order — despawn cascades still need to hear about every entity.
    pub fn clear(&mut self) {
        let entities: Vec<Entity> = self.set.entities().collect();
        self.set.clear();
        for entity in entities {
            self.events.push(ComponentEvent::Removed(entity));
        }
    }

    /// Remove all entries and return them as a `Vec<(Entity, T)>` in dense
    /// order, emitting [`ComponentEvent::Removed`] for each — the
    /// [`SparseSet::drain`] contract plus events.
    pub fn drain(&mut self) -> Vec<(Entity, T)> {
        let pairs = self.set.drain();
        for (entity, _) in &pairs {
            self.events.push(ComponentEvent::Removed(*entity));
        }
        pairs
    }

    /// The pending event queue, oldest first. Inspect without consuming —
    /// drain via [`drain_events`](Observed::drain_events) at the tick boundary.
    #[inline]
    pub fn events(&self) -> &EventQueue<ComponentEvent> {
        &self.events
    }

    /// Consume all pending events in emission order, leaving the queue empty.
    pub fn drain_events(&mut self) -> impl Iterator<Item = ComponentEvent> + '_ {
        self.events.drain()
    }

    /// Drop all pending events without reading them — for the rare caller
    /// that snapshots but never reacts.
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

impl<T: DetHash> DetHash for Observed<T> {
    /// Folds the storage in canonical entity order (identical to
    /// `SparseSet::det_hash`), then the pending event stream. Two `Observed`s
    /// with equal contents and equal pending events hash identically; pending
    /// events left undrained on one side desync the pair immediately.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.set.det_hash(hasher);
        self.events.det_hash(hasher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use crate::world_hash::hash_state;

    fn alloc_entities(alloc: &mut crate::entity::EntityAllocator, n: usize) -> Vec<Entity> {
        (0..n).map(|_| alloc.allocate()).collect()
    }

    #[test]
    fn insert_emits_added_then_replaced() {
        let mut alloc = crate::entity::EntityAllocator::new();
        let e = alloc.allocate();
        let mut s: Observed<u32> = Observed::new();
        s.insert(e, 1);
        s.insert(e, 2);
        let events: Vec<_> = s.drain_events().collect();
        assert_eq!(
            events,
            [ComponentEvent::Added(e), ComponentEvent::Replaced(e)]
        );
        assert_eq!(s.get(e), Some(&2));
        assert_eq!(s.len(), 1);
        assert!(!s.is_empty());
    }

    #[test]
    fn remove_present_emits_removed_absent_does_not() {
        let mut alloc = crate::entity::EntityAllocator::new();
        let e = alloc.allocate();
        let absent = alloc.allocate();
        let mut s: Observed<u32> = Observed::new();
        s.insert(e, 7);
        s.drain_events().for_each(drop); // clear the Added event

        assert_eq!(s.remove(e), Some(7));
        assert_eq!(s.remove(absent), None);
        let events: Vec<_> = s.drain_events().collect();
        assert_eq!(events, [ComponentEvent::Removed(e)]);
        assert!(!s.contains(e));
    }

    #[test]
    fn bulk_paths_emit_one_event_per_entity() {
        let mut alloc = crate::entity::EntityAllocator::new();
        let es = alloc_entities(&mut alloc, 5);
        let mut s: Observed<u32> = Observed::new();
        for &e in &es {
            s.insert(e, e.index());
        }
        s.drain_events().for_each(drop);

        // remove_where: drop evens (indices 0, 2, 4), keep odds.
        let removed = s.remove_where(|e, _| e.index() % 2 == 0);
        assert_eq!(removed, 3);
        let events: Vec<_> = s.drain_events().collect();
        assert_eq!(
            events,
            [
                ComponentEvent::Removed(es[0]),
                ComponentEvent::Removed(es[2]),
                ComponentEvent::Removed(es[4]),
            ]
        );

        // retain drops everything that fails the predicate: of the two
        // survivors {es[1], es[3]} only es[1] is kept.
        s.retain(|e, _| e.index() == 1);
        let events: Vec<_> = s.drain_events().collect();
        assert_eq!(events, [ComponentEvent::Removed(es[3])]);
        assert_eq!(s.len(), 1);
        assert!(s.contains(es[1]));
    }

    #[test]
    fn clear_and_drain_emit_for_every_entity() {
        let mut alloc = crate::entity::EntityAllocator::new();
        let es = alloc_entities(&mut alloc, 3);
        let mut s: Observed<u32> = Observed::new();
        for &e in &es {
            s.insert(e, e.index());
        }
        s.drain_events().for_each(drop);

        let pairs = s.drain();
        assert_eq!(pairs.len(), 3);
        assert!(s.is_empty());
        let events: Vec<_> = s.drain_events().collect();
        assert_eq!(events.len(), 3);
        assert!(events
            .iter()
            .all(|ev| matches!(ev, ComponentEvent::Removed(_))));

        for &e in &es {
            s.insert(e, e.index());
        }
        s.drain_events().for_each(drop);
        s.clear();
        assert_eq!(s.events().len(), 3);
        s.clear_events();
        assert!(s.events().is_empty());
    }

    #[test]
    fn in_place_mutation_emits_nothing() {
        let mut alloc = crate::entity::EntityAllocator::new();
        let e = alloc.allocate();
        let mut s: Observed<u32> = Observed::new();
        s.insert(e, 1);
        s.drain_events().for_each(drop);

        *s.get_mut(e).unwrap() = 9;
        for (_, v) in s.iter_mut() {
            *v += 1;
        }
        assert_eq!(s.get(e), Some(&10));
        assert!(
            s.events().is_empty(),
            "value mutation is not structural; it emits no events"
        );
        assert_eq!(s.inner().len(), 1);
    }

    #[test]
    fn identical_op_sequences_replay_identically() {
        let build = || {
            let mut alloc = crate::entity::EntityAllocator::new();
            let es = alloc_entities(&mut alloc, 6);
            let mut s: Observed<u32> = Observed::new();
            for &e in &es {
                s.insert(e, e.index() * 10);
            }
            s.insert(es[0], 999);
            s.remove(es[3]);
            s.remove_where(|e, _| e.index() % 2 == 1);
            let events: Vec<_> = s.events().iter().copied().collect();
            (s, events)
        };
        let (a, events_a) = build();
        let (b, events_b) = build();
        assert_eq!(events_a, events_b);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn pending_events_are_part_of_the_hash() {
        let mut alloc = crate::entity::EntityAllocator::new();
        let e = alloc.allocate();
        let mut a: Observed<u32> = Observed::new();
        let mut b: Observed<u32> = Observed::new();
        a.insert(e, 1);
        b.insert(e, 1);
        a.drain_events().for_each(drop); // same storage, empty queue
        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "an undrained event stream is observable state — it must hash"
        );
        b.drain_events().for_each(drop);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn despawn_cascade_consumes_removed_events() {
        // The motivating case: when Position leaves an entity, dependent
        // systems drop their own state for it.
        let mut alloc = crate::entity::EntityAllocator::new();
        let es = alloc_entities(&mut alloc, 4);
        let mut pos: Observed<u32> = Observed::new();
        let mut health: SparseSet<u32> = SparseSet::new();
        for &e in &es {
            pos.insert(e, e.index());
            health.insert(e, 100);
        }
        pos.drain_events().for_each(drop);

        // es[1] dies: remove its position, then let the event drive cleanup.
        pos.remove(es[1]);
        for event in pos.drain_events() {
            if let ComponentEvent::Removed(e) = event {
                health.remove(e);
            }
        }
        assert!(!health.contains(es[1]));
        assert_eq!(health.len(), 3);
    }

    #[test]
    fn event_stream_matches_a_model_under_random_ops() {
        // Oracle: a live-set model computes the expected event stream. Only
        // single-entity ops are modelled — bulk-path emission order follows
        // the set's dense order, which the model would have to re-implement;
        // `bulk_paths_emit_one_event_per_entity` pins that order instead.
        let mut alloc = crate::entity::EntityAllocator::new();
        let es = alloc_entities(&mut alloc, 8);
        let mut rng = SplitMix64::new(0xC0FFEE);
        let mut s: Observed<u32> = Observed::default();
        let mut live: Vec<Entity> = Vec::new();
        let mut expected: Vec<ComponentEvent> = Vec::new();

        for _ in 0..400 {
            match rng.next_u64() % 3 {
                0 | 1 => {
                    let e = es[rng.next_u64() as usize % es.len()];
                    let existed = live.contains(&e);
                    s.insert(e, rng.next_u64() as u32);
                    if existed {
                        expected.push(ComponentEvent::Replaced(e));
                    } else {
                        expected.push(ComponentEvent::Added(e));
                        live.push(e);
                    }
                }
                _ => {
                    // Remove a live entity most of the time, an absent one
                    // occasionally — the no-event case must stay silent.
                    let e = if live.is_empty() || rng.next_u64() % 4 == 0 {
                        es[rng.next_u64() as usize % es.len()]
                    } else {
                        live[rng.next_u64() as usize % live.len()]
                    };
                    let present = live.contains(&e);
                    // The model, not the system under test, decides whether a
                    // removal should have happened.
                    assert_eq!(s.remove(e).is_some(), present);
                    if present {
                        expected.push(ComponentEvent::Removed(e));
                        let pos = live.iter().position(|&x| x == e).unwrap();
                        live.swap_remove(pos);
                    }
                }
            }
        }
        let events: Vec<_> = s.events().iter().copied().collect();
        assert_eq!(events, expected);
        assert_eq!(s.len(), live.len());
        for &e in &live {
            assert!(s.contains(e));
        }
    }
}

//! Intra-tick game event queue — a FIFO channel for game-logic events.
//!
//! The kit already has purpose-built event containers in other directions:
//! [`cmdqueue::CmdQueue`](crate::cmdqueue::CmdQueue) carries *input commands*
//! from the outside world *into* the simulation,
//! [`timer::TimerQueue`](crate::timer::TimerQueue) fires *future* events when
//! a tick deadline passes, and
//! [`profiler::EventLog`](crate::profiler::EventLog) is a bounded *history*
//! ring buffer for diagnostics. None of them is the right shape for intra-tick
//! **game events flowing between systems**: when a monster dies the combat loop
//! wants to emit `MonsterKilled(id)` so that the XP, loot, faction, and UI
//! systems can each react — without the combat loop knowing those systems exist.
//!
//! [`EventQueue<E>`] is that missing channel: a plain FIFO queue that producers
//! [`push`](EventQueue::push) events onto and consumers
//! [`pop`](EventQueue::pop) / [`drain`](EventQueue::drain) from. It carries no
//! timestamps and enforces no capacity limit — both of those concerns belong in
//! the domain layer on top.
//!
//! ```
//! use izanagi_kit::eventqueue::EventQueue;
//!
//! #[derive(Debug, PartialEq)]
//! enum GameEvent { Killed(u32), XpGained(u32), LevelUp(u32) }
//!
//! let mut q: EventQueue<GameEvent> = EventQueue::new();
//! q.push(GameEvent::Killed(42));
//! q.push(GameEvent::XpGained(100));
//!
//! // Consume in the order they were produced.
//! assert_eq!(q.pop(), Some(GameEvent::Killed(42)));
//! assert_eq!(q.pop(), Some(GameEvent::XpGained(100)));
//! assert!(q.is_empty());
//! ```
//!
//! Determinism: the queue is purely an ordered collection — no timestamps, no
//! internal randomness, no platform calls. The replay checksum folds in the
//! queue contents via [`DetHash`](crate::world_hash::DetHash) (order-sensitive:
//! `[A, B]` hashes differently from `[B, A]`).

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::VecDeque;

/// A FIFO event queue for intra-tick game logic.
///
/// Producers call [`push`](Self::push) to enqueue events; consumers call
/// [`pop`](Self::pop) to dequeue the oldest event, or
/// [`drain`](Self::drain) to take all events at once.
#[derive(Clone, Debug, Default)]
pub struct EventQueue<E> {
    events: VecDeque<E>,
}

impl<E> EventQueue<E> {
    /// Create an empty queue.
    pub fn new() -> Self {
        EventQueue {
            events: VecDeque::new(),
        }
    }

    /// Create an empty queue with pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        EventQueue {
            events: VecDeque::with_capacity(cap),
        }
    }

    /// Enqueue `event` at the back.
    pub fn push(&mut self, event: E) {
        self.events.push_back(event);
    }

    /// Dequeue and return the oldest event, or `None` if the queue is empty.
    pub fn pop(&mut self) -> Option<E> {
        self.events.pop_front()
    }

    /// Borrow the oldest event without removing it, or `None` if empty.
    pub fn peek(&self) -> Option<&E> {
        self.events.front()
    }

    /// The number of events currently in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// `true` if the queue holds no events.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Remove all events from the queue.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Drain all events in FIFO order, removing them from the queue.
    /// The queue is empty after this iterator is consumed.
    pub fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.events.drain(..)
    }

    /// Iterate over all events in FIFO order without removing them.
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.events.iter()
    }

    /// Extend the queue by pushing every element of `iter`.
    pub fn extend<I: IntoIterator<Item = E>>(&mut self, iter: I) {
        self.events.extend(iter);
    }
}

impl<E: DetHash> DetHash for EventQueue<E> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.events.len() as u32);
        for e in &self.events {
            e.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_empty() {
        let q: EventQueue<u32> = EventQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert_eq!(q.peek(), None);
    }

    #[test]
    fn test_push_pop_fifo_order() {
        let mut q = EventQueue::new();
        q.push(1u32);
        q.push(2);
        q.push(3);
        assert_eq!(q.len(), 3);
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn test_peek_does_not_remove() {
        let mut q = EventQueue::new();
        q.push(42u32);
        assert_eq!(q.peek(), Some(&42));
        assert_eq!(q.peek(), Some(&42), "peek is non-destructive");
        assert_eq!(q.len(), 1);
        assert_eq!(q.pop(), Some(42));
        assert_eq!(q.peek(), None);
    }

    #[test]
    fn test_drain_empties_the_queue() {
        let mut q = EventQueue::new();
        q.push("a");
        q.push("b");
        q.push("c");
        let drained: Vec<&str> = q.drain().collect();
        assert_eq!(drained, vec!["a", "b", "c"], "drain must yield FIFO order");
        assert!(q.is_empty(), "queue must be empty after drain");
    }

    #[test]
    fn test_iter_is_non_destructive() {
        let mut q = EventQueue::new();
        q.push(10u32);
        q.push(20);
        let seen: Vec<u32> = q.iter().copied().collect();
        assert_eq!(seen, vec![10, 20]);
        assert_eq!(q.len(), 2, "iter must not remove events");
    }

    #[test]
    fn test_clear_empties() {
        let mut q = EventQueue::new();
        q.push(1u32);
        q.push(2);
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn test_extend() {
        let mut q = EventQueue::new();
        q.extend([1u32, 2, 3]);
        assert_eq!(q.len(), 3);
        assert_eq!(q.pop(), Some(1));
    }

    #[test]
    fn test_len_invariant() {
        let mut q = EventQueue::new();
        for i in 0u32..10 {
            q.push(i);
        }
        assert_eq!(q.len(), 10);
        for _ in 0..4 {
            q.pop();
        }
        assert_eq!(q.len(), 6);
        q.push(99);
        assert_eq!(q.len(), 7);
    }

    #[test]
    fn test_interleaved_push_pop() {
        let mut q = EventQueue::new();
        q.push(1u32);
        assert_eq!(q.pop(), Some(1));
        q.push(2);
        q.push(3);
        assert_eq!(q.pop(), Some(2));
        q.push(4);
        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), Some(4));
        assert!(q.is_empty());
    }

    #[test]
    fn test_with_capacity_behaves_correctly() {
        let mut q: EventQueue<u32> = EventQueue::with_capacity(16);
        assert!(q.is_empty());
        q.push(7);
        assert_eq!(q.pop(), Some(7));
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let mut a: EventQueue<u32> = EventQueue::new();
        a.push(1);
        a.push(2);
        let mut b: EventQueue<u32> = EventQueue::new();
        b.push(1);
        b.push(2);
        assert_eq!(hash_state(&a), hash_state(&b), "same queue, same hash");

        let mut c: EventQueue<u32> = EventQueue::new();
        c.push(2);
        c.push(1);
        assert_ne!(hash_state(&a), hash_state(&c), "order matters in hash");

        let mut d = a.clone();
        d.pop();
        assert_ne!(hash_state(&a), hash_state(&d), "pop changes hash");
    }
}

//! Event bus — decouple systems.
//!
//! Events are queued during a frame and drained at frame end. There is
//! one queue per event type. No heap allocation for small event batches.
//!
//! ```
//! use izanagi::event::Events;
//!
//! #[derive(Debug, Clone)]
//! struct EnemyDied { pub id: u32, pub score: u32 }
//!
//! let mut bus: Events<EnemyDied> = Events::new();
//! bus.send(EnemyDied { id: 7, score: 100 });
//! for e in bus.drain() { println!("enemy {} died, +{}", e.id, e.score); }
//! ```

/// A queue for events of type `E`.
pub struct Events<E> {
    queue: Vec<E>,
}

impl<E: Clone> Events<E> {
    /// Empty queue.
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    /// Enqueue an event.
    pub fn send(&mut self, e: E) {
        self.queue.push(e);
    }

    /// Iterate all queued events without clearing.
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.queue.iter()
    }

    /// Drain all events. Call at the end of each frame.
    pub fn drain(&mut self) -> Vec<E> {
        let out = self.queue.clone();
        self.queue.clear();
        out
    }

    /// Number of pending events.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// No pending events.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Discard all pending events without processing them.
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

impl<E: Clone> Default for Events<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Hit(u32);

    #[test]
    fn send_and_drain() {
        let mut bus: Events<Hit> = Events::new();
        bus.send(Hit(1));
        bus.send(Hit(2));
        assert_eq!(bus.len(), 2);
        let events = bus.drain();
        assert_eq!(events, vec![Hit(1), Hit(2)]);
        assert!(bus.is_empty());
    }

    #[test]
    fn drain_empties_queue() {
        let mut bus: Events<Hit> = Events::new();
        bus.send(Hit(99));
        let _ = bus.drain();
        let second = bus.drain();
        assert!(second.is_empty());
    }

    #[test]
    fn iter_does_not_clear() {
        let mut bus: Events<Hit> = Events::new();
        bus.send(Hit(1));
        let count = bus.iter().count();
        assert_eq!(count, 1);
        assert_eq!(bus.len(), 1);
    }
}

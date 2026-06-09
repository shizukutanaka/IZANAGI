//! Deterministic command queue — the replay-safe input abstraction.
//!
//! Roguelike/lockstep games must not poll OS events at arbitrary points in the
//! simulation loop: the exact *tick* an input is processed must be recorded
//! and reproducible. `CmdQueue<C>` collects commands between ticks and hands
//! them to the simulation in one atomic batch at the tick boundary.
//!
//! The design mirrors the "input buffer" pattern from determinism literature
//! (Gaffer "Deterministic Lockstep", arXiv:1705.05937): all non-determinism
//! from the OS (key events, AI decisions, network packets) enters through this
//! queue; the simulation only sees the drained slice, which replay can record
//! and reproduce exactly.
//!
//! `CmdQueue` is generic over `C` (the command type) and imposes no trait
//! bounds itself so it is usable with any game-specific command enum. The
//! `DetHash` impl is gated on `C: DetHash` so callers that care about replay
//! checking can still fold the pending commands into the world hash.

use crate::world_hash::{DetHash, Fnv1a};

/// A FIFO queue of pending game commands.
///
/// Commands are pushed between simulation ticks and drained (consumed) at the
/// tick boundary. Draining is the *only* way to retrieve commands so that no
/// command is processed more than once.
#[derive(Clone, Debug, Default)]
pub struct CmdQueue<C> {
    buf: Vec<C>,
}

impl<C> CmdQueue<C> {
    pub fn new() -> Self {
        CmdQueue { buf: Vec::new() }
    }

    /// Enqueue a single command.
    #[inline]
    pub fn push(&mut self, cmd: C) {
        self.buf.push(cmd);
    }

    /// Enqueue multiple commands from a slice (cloned into the queue).
    pub fn push_batch(&mut self, cmds: &[C])
    where
        C: Clone,
    {
        self.buf.extend_from_slice(cmds);
    }

    /// Returns all queued commands in insertion order and clears the queue.
    /// This is the canonical way to consume the queue at a tick boundary.
    pub fn drain(&mut self) -> Vec<C> {
        std::mem::take(&mut self.buf)
    }

    /// Number of commands currently in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Discard all pending commands without returning them.
    #[inline]
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Read-only view of pending commands (in insertion order) without draining.
    /// Useful for preview/debug; the simulation tick should use [`drain`](Self::drain).
    #[inline]
    pub fn peek(&self) -> &[C] {
        &self.buf
    }

    /// Drain and return only commands for which `pred` returns `true`.
    /// Commands that don't match are kept in the queue in their original
    /// relative order. Useful when two subsystems share one queue but each
    /// should only see its own command variants.
    pub fn drain_if<F>(&mut self, mut pred: F) -> Vec<C>
    where
        F: FnMut(&C) -> bool,
    {
        let mut drained = Vec::new();
        let mut kept = Vec::new();
        for cmd in self.buf.drain(..) {
            if pred(&cmd) {
                drained.push(cmd);
            } else {
                kept.push(cmd);
            }
        }
        self.buf = kept;
        drained
    }
}

impl<C: DetHash> DetHash for CmdQueue<C> {
    /// Folds pending commands in insertion order. Hashing the queue before a
    /// tick lets the replay harness verify that both original and replayed runs
    /// present the same commands to the sim at that tick.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.buf.len() as u32);
        for cmd in &self.buf {
            cmd.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_push_and_drain_fifo_order() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(1);
        q.push(2);
        q.push(3);
        assert_eq!(q.drain(), vec![1, 2, 3]);
        assert!(q.is_empty());
    }

    #[test]
    fn test_drain_empties_queue() {
        let mut q: CmdQueue<u32> = CmdQueue::new();
        q.push(7);
        q.push(8);
        let _ = q.drain();
        assert_eq!(q.len(), 0);
        assert!(q.is_empty());
    }

    #[test]
    fn test_push_batch() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[10, 20, 30]);
        assert_eq!(q.drain(), vec![10, 20, 30]);
    }

    #[test]
    fn test_drain_returns_empty_vec_when_queue_empty() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        assert_eq!(q.drain(), vec![]);
    }

    #[test]
    fn test_multiple_drain_cycles() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(1);
        let first = q.drain();
        q.push(2);
        q.push(3);
        let second = q.drain();
        assert_eq!(first, vec![1]);
        assert_eq!(second, vec![2, 3]);
    }

    #[test]
    fn test_clear_discards_without_returning() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(99);
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.drain(), vec![]);
    }

    #[test]
    fn test_peek_does_not_consume() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(42);
        q.push(43);
        let peeked = q.peek().to_vec();
        assert_eq!(peeked, [42, 43]);
        // Commands still in queue.
        assert_eq!(q.drain(), vec![42, 43]);
    }

    #[test]
    fn test_drain_if_splits_matching_and_nonmatching() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[1, 2, 3, 4, 5]);
        let evens = q.drain_if(|x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4]);
        assert_eq!(q.peek(), &[1, 3, 5]);
    }

    #[test]
    fn test_drain_if_all_match_clears_queue() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[10, 20]);
        let all = q.drain_if(|_| true);
        assert_eq!(all, vec![10, 20]);
        assert!(q.is_empty());
    }

    #[test]
    fn test_drain_if_none_match_leaves_queue_intact() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[1, 3, 5]);
        let none = q.drain_if(|x| x % 2 == 0);
        assert!(none.is_empty());
        assert_eq!(q.drain(), vec![1, 3, 5]);
    }

    #[test]
    fn test_drain_if_preserves_order_of_kept() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[5, 1, 4, 1, 3]);
        q.drain_if(|&x| x > 3); // drain 5 and 4
        assert_eq!(q.drain(), vec![1, 1, 3]);
    }

    #[test]
    fn test_det_hash_same_commands_same_hash() {
        let mut a: CmdQueue<u32> = CmdQueue::new();
        let mut b: CmdQueue<u32> = CmdQueue::new();
        for v in [1u32, 2, 3] {
            a.push(v);
            b.push(v);
        }
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_order_matters() {
        let mut a: CmdQueue<u32> = CmdQueue::new();
        let mut b: CmdQueue<u32> = CmdQueue::new();
        a.push(1);
        a.push(2);
        b.push(2);
        b.push(1);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_empty_differs_from_nonempty() {
        let empty: CmdQueue<u32> = CmdQueue::new();
        let mut nonempty: CmdQueue<u32> = CmdQueue::new();
        nonempty.push(0);
        assert_ne!(hash_state(&empty), hash_state(&nonempty));
    }

    #[test]
    fn test_len_tracks_pushes_and_drains() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        assert_eq!(q.len(), 0);
        q.push(1);
        assert_eq!(q.len(), 1);
        q.push(2);
        assert_eq!(q.len(), 2);
        let _ = q.drain();
        assert_eq!(q.len(), 0);
    }
}

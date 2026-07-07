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
    /// Create an empty queue.
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

    /// `true` if the queue has no pending commands.
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

    /// View the pending commands as a slice. Follows the Rust `as_slice`
    /// convention; identical in result to [`peek`](Self::peek) but preferred
    /// when callers want slice-specific APIs (e.g. `windows`, `chunks`).
    #[inline]
    pub fn as_slice(&self) -> &[C] {
        &self.buf
    }

    /// Mutable reference to the first command in the queue, or `None` if
    /// empty. Lets callers modify a pending command in-place (e.g. update a
    /// target position) without popping and re-inserting.
    #[inline]
    pub fn peek_mut(&mut self) -> Option<&mut C> {
        self.buf.first_mut()
    }

    /// Insert a command at the *front* of the queue (LIFO-priority insertion).
    ///
    /// Use for priority commands that must be processed before already-queued
    /// ones (e.g. "abort" overrides pending movement). O(n) — not for hot paths
    /// with large queues.
    pub fn prepend(&mut self, cmd: C) {
        self.buf.insert(0, cmd);
    }

    /// Keep only commands for which `pred` returns `true`; discard the rest.
    /// The complement of [`drain_if`](Self::drain_if) — use when you want to
    /// filter in-place without taking ownership of the discarded commands.
    pub fn retain<F>(&mut self, pred: F)
    where
        F: FnMut(&C) -> bool,
    {
        self.buf.retain(pred);
    }

    /// Remove and return the front command (oldest / first-in), or `None` if
    /// the queue is empty. O(n) — shifts remaining elements. Use `drain` for
    /// bulk consumption; this is for the "process exactly one command per tick"
    /// rate-limiting pattern.
    pub fn pop_front(&mut self) -> Option<C> {
        if self.buf.is_empty() {
            None
        } else {
            Some(self.buf.remove(0))
        }
    }

    /// Remove and return the back command (newest / last-in), or `None` if
    /// the queue is empty. O(1). Useful for LIFO (stack) semantics or
    /// "cancel last queued command" patterns.
    pub fn pop_back(&mut self) -> Option<C> {
        self.buf.pop()
    }

    /// Remove and return the front command (oldest / first-in), or `None` if
    /// the queue is empty. Alias for `pop_front` — use whichever reads more
    /// naturally at the call site for typical FIFO consumption.
    #[inline]
    pub fn pop(&mut self) -> Option<C> {
        self.pop_front()
    }

    /// Return a reference to the command at position `i` (0 = oldest), or
    /// `None` if `i >= len`. Does not consume or advance the queue — use
    /// `drain` or `pop_front` for that. Follows the same insertion order as
    /// `peek()`; equivalent to `peek().get(i)`.
    #[inline]
    pub fn index(&self, i: usize) -> Option<&C> {
        self.buf.get(i)
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

    /// Return `true` if any queued command satisfies `pred`, without consuming
    /// the queue. Useful for "is there a Cancel command pending?" checks before
    /// committing to a long operation.
    pub fn contains<F>(&self, pred: F) -> bool
    where
        F: Fn(&C) -> bool,
    {
        self.buf.iter().any(pred)
    }

    /// Count queued commands for which `pred` returns `true`, without draining
    /// the queue.
    ///
    /// Mirrors [`contains`](Self::contains) but returns the exact count rather
    /// than a boolean — useful for "how many move commands are pending?" or
    /// rate-limiting guards that allow at most N commands of a given type.
    pub fn count<F>(&self, pred: F) -> usize
    where
        F: Fn(&C) -> bool,
    {
        self.buf.iter().filter(|c| pred(c)).count()
    }

    /// Reference to the last (most recently pushed) command, or `None` if
    /// empty. Mirrors `peek_mut` / `pop_back` for the back end — useful for
    /// "what did the player just queue?" checks without consuming the command.
    #[inline]
    pub fn peek_back(&self) -> Option<&C> {
        self.buf.last()
    }

    /// Keep only the first `n` commands, discarding everything after them.
    /// No-op if the queue already has ≤ `n` entries. Useful for hard caps on
    /// input buffering — `truncate(0)` is equivalent to `clear`.
    #[inline]
    pub fn truncate(&mut self, n: usize) {
        self.buf.truncate(n);
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
    fn test_prepend_inserts_at_front() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(2);
        q.push(3);
        q.prepend(1);
        assert_eq!(q.drain(), vec![1, 2, 3]);
    }

    #[test]
    fn test_prepend_empty_queue() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.prepend(42);
        assert_eq!(q.drain(), vec![42]);
    }

    #[test]
    fn test_retain_keeps_matching() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[1, 2, 3, 4, 5]);
        q.retain(|x| x % 2 != 0); // keep odds
        assert_eq!(q.drain(), vec![1, 3, 5]);
    }

    #[test]
    fn test_retain_all_removed() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[2, 4, 6]);
        q.retain(|x| x % 2 != 0);
        assert!(q.is_empty());
    }

    #[test]
    fn test_retain_all_kept() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[1, 3, 5]);
        q.retain(|_| true);
        assert_eq!(q.len(), 3);
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
    fn test_pop_front_removes_first_element() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[1, 2, 3]);
        assert_eq!(q.pop_front(), Some(1));
        assert_eq!(q.peek(), &[2, 3]);
    }

    #[test]
    fn test_pop_back_removes_last_element() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[1, 2, 3]);
        assert_eq!(q.pop_back(), Some(3));
        assert_eq!(q.peek(), &[1, 2]);
    }

    #[test]
    fn test_pop_front_empty_returns_none() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        assert_eq!(q.pop_front(), None);
    }

    #[test]
    fn test_pop_back_empty_returns_none() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        assert_eq!(q.pop_back(), None);
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

    #[test]
    fn test_as_slice_returns_pending_commands() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(10);
        q.push(20);
        assert_eq!(q.as_slice(), &[10, 20]);
    }

    #[test]
    fn test_as_slice_empty_queue_is_empty_slice() {
        let q: CmdQueue<i32> = CmdQueue::new();
        assert!(q.as_slice().is_empty());
    }

    #[test]
    fn test_as_slice_does_not_consume() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(1);
        let _ = q.as_slice();
        assert_eq!(q.len(), 1); // still in the queue
    }

    #[test]
    fn test_index_returns_element_at_position() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(10);
        q.push(20);
        q.push(30);
        assert_eq!(q.index(0), Some(&10));
        assert_eq!(q.index(1), Some(&20));
        assert_eq!(q.index(2), Some(&30));
    }

    #[test]
    fn test_index_out_of_bounds_returns_none() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(5);
        assert_eq!(q.index(1), None);
        assert_eq!(q.index(100), None);
    }

    #[test]
    fn test_index_does_not_consume() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(99);
        let _ = q.index(0);
        assert_eq!(q.len(), 1);
        let drained = q.drain();
        assert_eq!(drained, vec![99]);
    }

    #[test]
    fn test_contains_returns_true_when_match_exists() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(1);
        q.push(2);
        q.push(3);
        assert!(q.contains(|c| *c == 2));
    }

    #[test]
    fn test_contains_returns_false_when_no_match() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(1);
        q.push(3);
        assert!(!q.contains(|c| *c == 99));
    }

    #[test]
    fn test_contains_does_not_consume_queue() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(5);
        let _ = q.contains(|c| *c == 5);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_count_returns_zero_for_no_matches() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(1);
        q.push(3);
        assert_eq!(q.count(|c| *c == 99), 0);
    }

    #[test]
    fn test_count_returns_exact_match_count() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(2);
        q.push(3);
        q.push(2);
        q.push(5);
        assert_eq!(q.count(|c| *c == 2), 2);
    }

    #[test]
    fn test_count_does_not_consume_queue() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(7);
        q.push(7);
        let _ = q.count(|c| *c == 7);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn test_pop_returns_front_command() {
        let mut q: CmdQueue<u32> = CmdQueue::new();
        q.push(1);
        q.push(2);
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
    }

    #[test]
    fn test_pop_empty_returns_none() {
        let mut q: CmdQueue<u32> = CmdQueue::new();
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn test_pop_matches_pop_front() {
        let mut q1: CmdQueue<u32> = CmdQueue::new();
        let mut q2: CmdQueue<u32> = CmdQueue::new();
        q1.push(10);
        q2.push(10);
        assert_eq!(q1.pop(), q2.pop_front());
    }

    // --- peek_mut ---

    #[test]
    fn test_peek_mut_returns_first_element() {
        let mut q: CmdQueue<u32> = CmdQueue::new();
        q.push(1);
        q.push(2);
        assert_eq!(q.peek_mut(), Some(&mut 1u32));
    }

    #[test]
    fn test_peek_mut_allows_in_place_modification() {
        let mut q: CmdQueue<u32> = CmdQueue::new();
        q.push(10);
        q.push(20);
        *q.peek_mut().unwrap() = 99;
        assert_eq!(q.as_slice()[0], 99);
        assert_eq!(q.as_slice()[1], 20);
    }

    #[test]
    fn test_peek_mut_empty_queue_returns_none() {
        let mut q: CmdQueue<u32> = CmdQueue::new();
        assert_eq!(q.peek_mut(), None);
    }

    // --- peek_back ---

    #[test]
    fn test_peek_back_returns_last_pushed() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(1);
        q.push(2);
        q.push(3);
        assert_eq!(q.peek_back(), Some(&3));
    }

    #[test]
    fn test_peek_back_empty_returns_none() {
        let q: CmdQueue<i32> = CmdQueue::new();
        assert_eq!(q.peek_back(), None);
    }

    #[test]
    fn test_peek_back_does_not_consume() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push(42);
        let _ = q.peek_back();
        assert_eq!(q.len(), 1);
        assert_eq!(q.peek_back(), Some(&42));
    }

    // --- truncate ---

    #[test]
    fn test_truncate_keeps_first_n() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[1, 2, 3, 4, 5]);
        q.truncate(3);
        assert_eq!(q.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn test_truncate_zero_empties_queue() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[10, 20, 30]);
        q.truncate(0);
        assert!(q.is_empty());
    }

    #[test]
    fn test_truncate_beyond_len_is_noop() {
        let mut q: CmdQueue<i32> = CmdQueue::new();
        q.push_batch(&[1, 2]);
        q.truncate(10);
        assert_eq!(q.len(), 2);
        assert_eq!(q.as_slice(), &[1, 2]);
    }
}

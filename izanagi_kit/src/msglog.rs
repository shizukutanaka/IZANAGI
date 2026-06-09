//! Fixed-capacity message log for roguelike UI.
//!
//! Game events ("You hit the orc for 5 damage.") accumulate here and are
//! rendered by the UI layer. The log is a ring buffer: once `capacity` is
//! reached, the oldest entry is silently dropped so the newest is always
//! visible. Capacity is set at construction and cannot be changed, keeping
//! heap allocation bounded and predictable.
//!
//! `MsgLog` implements `DetHash` so the visible message history can be folded
//! into the world hash — useful for snapshot tests that verify a particular
//! event sequence was produced.

use crate::world_hash::{DetHash, Fnv1a};

/// A bounded FIFO message log backed by a ring buffer.
///
/// Push cost is O(1); iteration is O(n) in oldest-to-newest order.
#[derive(Clone, Debug)]
pub struct MsgLog {
    buf: Vec<String>,
    head: usize,
    len: usize,
}

impl MsgLog {
    /// Create a new log with the given capacity. A capacity of `0` silently
    /// discards every message pushed (useful in headless tests that don't care
    /// about the log).
    pub fn new(capacity: usize) -> Self {
        MsgLog {
            buf: (0..capacity).map(|_| String::new()).collect(),
            head: 0,
            len: 0,
        }
    }

    /// Number of messages currently stored.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Maximum messages the log can hold before dropping the oldest.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Append a message. If the log is at capacity the oldest entry is
    /// overwritten. A capacity-0 log discards immediately.
    pub fn push(&mut self, msg: impl Into<String>) {
        let cap = self.buf.len();
        if cap == 0 {
            return;
        }
        let slot = (self.head + self.len) % cap;
        self.buf[slot] = msg.into();
        if self.len < cap {
            self.len += 1;
        } else {
            // Full: advance head past the now-overwritten oldest entry.
            self.head = (self.head + 1) % cap;
        }
    }

    /// Iterate messages in oldest-to-newest order.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        let cap = self.buf.len();
        (0..self.len).map(move |i| self.buf[(self.head + i) % cap].as_str())
    }

    /// The `n` most recent messages, oldest-first within the slice. If fewer
    /// than `n` are stored, all stored messages are returned.
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &str> {
        let start = self.len.saturating_sub(n);
        let cap = self.buf.len();
        (start..self.len).map(move |i| self.buf[(self.head + i) % cap].as_str())
    }

    /// The most recently pushed message, or `None` if the log is empty.
    #[inline]
    pub fn last(&self) -> Option<&str> {
        if self.len == 0 {
            return None;
        }
        let cap = self.buf.len();
        let idx = (self.head + self.len - 1) % cap;
        Some(self.buf[idx].as_str())
    }

    /// Clear all messages without changing capacity.
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
}

impl DetHash for MsgLog {
    /// Folds the full message history (oldest-to-newest) and the length into
    /// the hasher. Two logs with the same messages in the same order hash
    /// identically regardless of internal `head` position.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.len as u32);
        for msg in self.iter() {
            hasher.write_u32(msg.len() as u32);
            hasher.write_bytes(msg.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_push_and_iter_in_order() {
        let mut log = MsgLog::new(5);
        log.push("a");
        log.push("b");
        log.push("c");
        let msgs: Vec<&str> = log.iter().collect();
        assert_eq!(msgs, ["a", "b", "c"]);
    }

    #[test]
    fn test_overflow_drops_oldest() {
        let mut log = MsgLog::new(3);
        log.push("x");
        log.push("y");
        log.push("z");
        log.push("w"); // drops "x"
        let msgs: Vec<&str> = log.iter().collect();
        assert_eq!(msgs, ["y", "z", "w"]);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn test_zero_capacity_discards_silently() {
        let mut log = MsgLog::new(0);
        log.push("hello");
        assert_eq!(log.len(), 0);
        assert!(log.iter().next().is_none());
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut log = MsgLog::new(4);
        assert!(log.is_empty());
        log.push("a");
        assert!(!log.is_empty());
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_recent_fewer_than_stored() {
        let mut log = MsgLog::new(10);
        for c in ["a", "b", "c", "d", "e"] {
            log.push(c);
        }
        let r: Vec<&str> = log.recent(3).collect();
        assert_eq!(r, ["c", "d", "e"]);
    }

    #[test]
    fn test_recent_more_than_stored_returns_all() {
        let mut log = MsgLog::new(10);
        log.push("p");
        log.push("q");
        let r: Vec<&str> = log.recent(20).collect();
        assert_eq!(r, ["p", "q"]);
    }

    #[test]
    fn test_clear_resets_log() {
        let mut log = MsgLog::new(4);
        log.push("a");
        log.push("b");
        log.clear();
        assert!(log.is_empty());
        log.push("c");
        let msgs: Vec<&str> = log.iter().collect();
        assert_eq!(msgs, ["c"]);
    }

    #[test]
    fn test_last_returns_most_recent() {
        let mut log = MsgLog::new(5);
        assert_eq!(log.last(), None);
        log.push("first");
        assert_eq!(log.last(), Some("first"));
        log.push("second");
        assert_eq!(log.last(), Some("second"));
    }

    #[test]
    fn test_last_after_overflow() {
        let mut log = MsgLog::new(2);
        log.push("a");
        log.push("b");
        log.push("c"); // wraps; visible = ["b","c"]
        assert_eq!(log.last(), Some("c"));
    }

    #[test]
    fn test_det_hash_same_messages_same_hash() {
        let mut a = MsgLog::new(5);
        let mut b = MsgLog::new(8); // different capacity, same content
        for m in ["hello", "world"] {
            a.push(m);
            b.push(m);
        }
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_changes_on_different_messages() {
        let mut a = MsgLog::new(5);
        let mut b = MsgLog::new(5);
        a.push("attack");
        b.push("defend");
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_after_overflow_is_order_dependent() {
        // Log that has wrapped should hash on the *visible* messages only.
        let mut a = MsgLog::new(2);
        a.push("x");
        a.push("y");
        a.push("z"); // wraps; visible = ["y", "z"]

        let mut b = MsgLog::new(2);
        b.push("y");
        b.push("z"); // same visible content

        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_multiple_overflows_stays_correct() {
        let mut log = MsgLog::new(3);
        for i in 0u32..12 {
            log.push(format!("msg{i}"));
        }
        // Only the last 3 should remain.
        let msgs: Vec<&str> = log.iter().collect();
        assert_eq!(msgs, ["msg9", "msg10", "msg11"]);
        assert_eq!(log.len(), 3);
    }
}

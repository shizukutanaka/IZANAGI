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

    /// Remove and return the most recently pushed message (LIFO order), or
    /// `None` if the log is empty. Useful for "undo last log entry" patterns
    /// and round-trip tests that push then pop.
    pub fn pop(&mut self) -> Option<String> {
        if self.len == 0 {
            return None;
        }
        let cap = self.buf.len();
        let idx = (self.head + self.len - 1) % cap;
        self.len -= 1;
        Some(std::mem::take(&mut self.buf[idx]))
    }

    /// Get the message at logical position `index` (0 = oldest). Returns `None`
    /// if `index >= len`. Complements `iter()` when random access is needed
    /// (e.g. rendering a scrollable log with a cursor).
    pub fn get(&self, index: usize) -> Option<&str> {
        if index >= self.len {
            return None;
        }
        let cap = self.buf.len();
        Some(self.buf[(self.head + index) % cap].as_str())
    }

    /// The oldest message in the log, or `None` if empty. Mirrors `last()`.
    #[inline]
    pub fn first(&self) -> Option<&str> {
        self.get(0)
    }

    /// Retain only messages for which `pred` returns `true`, preserving their
    /// original oldest-to-newest order. Rebuilds the ring buffer in-place;
    /// capacity is unchanged.
    pub fn retain<P: Fn(&str) -> bool>(&mut self, pred: P) {
        let kept: Vec<String> = self
            .iter()
            .filter(|&s| pred(s))
            .map(str::to_owned)
            .collect();
        self.head = 0;
        self.len = 0;
        for s in kept {
            self.push(s);
        }
    }

    /// Clear all messages without changing capacity.
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    /// `true` if any stored message contains `needle` as a substring.
    pub fn contains(&self, needle: &str) -> bool {
        self.iter().any(|msg| msg.contains(needle))
    }

    /// Count messages for which `pred` returns `true`.
    pub fn count_where<P: Fn(&str) -> bool>(&self, pred: P) -> usize {
        self.iter().filter(|msg| pred(msg)).count()
    }

    /// Collect references to all messages for which `pred` returns `true`, in
    /// oldest-to-newest order. Non-destructive (unlike [`retain`](Self::retain),
    /// which rebuilds the buffer). Useful for "show only combat messages" views
    /// without mutating the underlying log.
    pub fn filtered<P: Fn(&str) -> bool>(&self, pred: P) -> Vec<&str> {
        self.iter().filter(|msg| pred(msg)).collect()
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
    fn test_get_by_index() {
        let mut log = MsgLog::new(5);
        log.push("a");
        log.push("b");
        log.push("c");
        assert_eq!(log.get(0), Some("a"));
        assert_eq!(log.get(2), Some("c"));
        assert_eq!(log.get(3), None);
    }

    #[test]
    fn test_get_after_overflow() {
        let mut log = MsgLog::new(3);
        log.push("x");
        log.push("y");
        log.push("z");
        log.push("w"); // visible: y z w
        assert_eq!(log.get(0), Some("y"));
        assert_eq!(log.get(2), Some("w"));
    }

    #[test]
    fn test_first_returns_oldest() {
        let mut log = MsgLog::new(5);
        assert_eq!(log.first(), None);
        log.push("oldest");
        log.push("newest");
        assert_eq!(log.first(), Some("oldest"));
    }

    #[test]
    fn test_retain_keeps_matching() {
        let mut log = MsgLog::new(8);
        for msg in ["attack", "move", "attack", "spell"] {
            log.push(msg);
        }
        log.retain(|s| s.starts_with('a'));
        let msgs: Vec<&str> = log.iter().collect();
        assert_eq!(msgs, ["attack", "attack"]);
    }

    #[test]
    fn test_retain_all_false_empties_log() {
        let mut log = MsgLog::new(4);
        log.push("hello");
        log.retain(|_| false);
        assert!(log.is_empty());
    }

    #[test]
    fn test_retain_preserves_order() {
        let mut log = MsgLog::new(6);
        for i in 0u32..5 {
            log.push(format!("{i}"));
        }
        log.retain(|s| s.parse::<u32>().unwrap() % 2 == 0);
        let msgs: Vec<&str> = log.iter().collect();
        assert_eq!(msgs, ["0", "2", "4"]);
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

    #[test]
    fn test_contains_finds_substring() {
        let mut log = MsgLog::new(5);
        log.push("You attack the goblin");
        log.push("The goblin hits you");
        assert!(log.contains("goblin"));
        assert!(log.contains("You attack"));
    }

    #[test]
    fn test_contains_returns_false_when_absent() {
        let mut log = MsgLog::new(5);
        log.push("hello world");
        assert!(!log.contains("dragon"));
    }

    #[test]
    fn test_contains_empty_log_returns_false() {
        let log = MsgLog::new(5);
        assert!(!log.contains("anything"));
    }

    #[test]
    fn test_count_where_counts_matching_messages() {
        let mut log = MsgLog::new(8);
        log.push("hit");
        log.push("miss");
        log.push("hit");
        log.push("crit");
        assert_eq!(log.count_where(|m| m.contains("hit")), 2);
        assert_eq!(log.count_where(|m| m == "miss"), 1);
    }

    #[test]
    fn test_count_where_empty_log_returns_zero() {
        let log = MsgLog::new(5);
        assert_eq!(log.count_where(|_| true), 0);
    }

    #[test]
    fn test_pop_returns_last_message() {
        let mut log = MsgLog::new(4);
        log.push("first");
        log.push("second");
        assert_eq!(log.pop(), Some("second".to_owned()));
        assert_eq!(log.len(), 1);
        assert_eq!(log.last(), Some("first"));
    }

    #[test]
    fn test_pop_on_empty_log_returns_none() {
        let mut log = MsgLog::new(4);
        assert_eq!(log.pop(), None);
    }

    #[test]
    fn test_pop_to_empty() {
        let mut log = MsgLog::new(2);
        log.push("only");
        let msg = log.pop();
        assert_eq!(msg, Some("only".to_owned()));
        assert!(log.is_empty());
    }

    #[test]
    fn test_filtered_returns_matching_only() {
        let mut log = MsgLog::new(8);
        for m in ["attack", "move", "attack", "spell"] {
            log.push(m);
        }
        let hits = log.filtered(|s| s == "attack");
        assert_eq!(hits, ["attack", "attack"]);
    }

    #[test]
    fn test_filtered_preserves_order() {
        let mut log = MsgLog::new(8);
        for m in ["a1", "b", "a2", "c", "a3"] {
            log.push(m);
        }
        let hits = log.filtered(|s| s.starts_with('a'));
        assert_eq!(hits, ["a1", "a2", "a3"]);
    }

    #[test]
    fn test_filtered_empty_when_no_match() {
        let mut log = MsgLog::new(4);
        log.push("hello");
        assert!(log.filtered(|s| s.contains("dragon")).is_empty());
    }
}

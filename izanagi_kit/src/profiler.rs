//! Simple tick-budget profiler for roguelike / headless simulations.
//!
//! `Profiler` records named timing samples (in nanoseconds or arbitrary
//! integer "work units") and answers questions like:
//! - How much time did section X take this tick?
//! - What was the peak / average over the last N ticks?
//! - Which section consumed the most budget?
//!
//! There is no OS clock dependency — callers supply timestamps as `u64`
//! (e.g. from a monotonic counter). The profiler just tracks min/max/sum/count
//! per named section across a rolling window of ticks.
//!
//! `EventLog<E>` is a companion bounded-ring event log that records typed
//! simulation events (damage dealt, item picked up, level entered …) with a
//! tick stamp. Unlike `MsgLog` (human-readable strings) this stores structured
//! data useful for AI replay analysis or test assertions.
//!
//! Both types implement `DetHash`.

use crate::world_hash::{DetHash, Fnv1a};

// ---------------------------------------------------------------------------
// Profiler
// ---------------------------------------------------------------------------

/// One named timing section's accumulated statistics for the current tick.
#[derive(Clone, Debug)]
struct Section {
    name: &'static str,
    total: u64,
    calls: u32,
    peak: u64,
    min: u64,
}

impl Default for Section {
    fn default() -> Self {
        Section {
            name: "",
            total: 0,
            calls: 0,
            peak: 0,
            min: u64::MAX,
        }
    }
}

/// A per-tick section-based profiler.
///
/// Call `begin_tick()` at the start of each tick to reset per-tick counters
/// and record the previous tick's totals into the rolling history.
/// Call `record(section_name, elapsed_ns)` for each timed section.
/// Query `this_tick(name)`, `peak(name)`, `avg(name)` at any point.
///
/// Sections are identified by `&'static str` so no allocation occurs at
/// recording time.
#[derive(Clone, Debug)]
pub struct Profiler {
    sections: Vec<Section>,
    /// Rolling history: ring buffer of (section_index, total) per tick.
    history: Vec<(u32, u64)>, // (section_idx, tick_total)
    history_head: usize,
    history_cap: usize,
    tick: u32,
}

impl Profiler {
    /// Create a profiler that keeps `history_ticks` ticks of rolling history.
    pub fn new(history_ticks: usize) -> Self {
        Profiler {
            sections: Vec::new(),
            history: Vec::new(),
            history_head: 0,
            history_cap: history_ticks.max(1),
            tick: 0,
        }
    }

    /// Advance to the next tick. Flushes current totals into history.
    pub fn begin_tick(&mut self) {
        // Push current totals into rolling history.
        for (i, sec) in self.sections.iter().enumerate() {
            if sec.calls > 0 {
                let entry = (i as u32, sec.total);
                if self.history.len() < self.history_cap {
                    self.history.push(entry);
                } else {
                    self.history[self.history_head] = entry;
                    self.history_head = (self.history_head + 1) % self.history_cap;
                }
            }
        }
        // Reset per-tick totals.
        for sec in &mut self.sections {
            sec.total = 0;
            sec.calls = 0;
        }
        self.tick = self.tick.saturating_add(1);
    }

    /// Record `elapsed` units for `section`. Creates the section if new.
    pub fn record(&mut self, section: &'static str, elapsed: u64) {
        if let Some(s) = self.sections.iter_mut().find(|s| s.name == section) {
            s.total = s.total.saturating_add(elapsed);
            s.calls = s.calls.saturating_add(1);
            if elapsed > s.peak {
                s.peak = elapsed;
            }
            if elapsed < s.min {
                s.min = elapsed;
            }
        } else {
            self.sections.push(Section {
                name: section,
                total: elapsed,
                calls: 1,
                peak: elapsed,
                min: elapsed,
            });
        }
    }

    /// Total elapsed for `section` in the current (not yet flushed) tick.
    pub fn this_tick(&self, section: &str) -> u64 {
        self.sections
            .iter()
            .find(|s| s.name == section)
            .map(|s| s.total)
            .unwrap_or(0)
    }

    /// All-time peak single-call elapsed for `section`.
    pub fn peak(&self, section: &str) -> u64 {
        self.sections
            .iter()
            .find(|s| s.name == section)
            .map(|s| s.peak)
            .unwrap_or(0)
    }

    /// Current tick number (incremented by `begin_tick`).
    pub fn tick(&self) -> u32 {
        self.tick
    }

    /// Known section names.
    pub fn section_names(&self) -> impl Iterator<Item = &str> {
        self.sections.iter().map(|s| s.name)
    }

    /// Returns `true` if the current-tick total for `section` exceeds `budget`.
    /// Zero for an unknown section (never recorded) is never over budget.
    /// Use this for per-frame budget monitoring: "did pathfinding take too long
    /// this tick?" without keeping a manual threshold comparison everywhere.
    #[inline]
    pub fn budget_exceeded(&self, section: &str, budget: u64) -> bool {
        self.this_tick(section) > budget
    }

    /// Rolling average tick-total for `section` over the history window.
    ///
    /// Only ticks where the section was actually recorded are included in the
    /// average (ticks where it was silent don't count). Returns `0` if the
    /// section is unknown or has no history entries yet (i.e. `begin_tick` has
    /// not been called since the first `record`).
    pub fn avg(&self, section: &str) -> u64 {
        let Some(idx) = self.sections.iter().position(|s| s.name == section) else {
            return 0;
        };
        let idx = idx as u32;
        let mut sum = 0u64;
        let mut count = 0u64;
        for &(i, total) in &self.history {
            if i == idx {
                sum = sum.saturating_add(total);
                count += 1;
            }
        }
        if count == 0 {
            0
        } else {
            sum / count
        }
    }

    /// Number of `record()` calls for `section` in the current (unflushed) tick.
    /// Returns `0` for an unknown or silent section.
    pub fn section_count(&self, section: &str) -> u32 {
        self.sections
            .iter()
            .find(|s| s.name == section)
            .map(|s| s.calls)
            .unwrap_or(0)
    }

    /// All-time minimum single-call elapsed for `section`.
    /// Returns `0` for an unknown section (never recorded).
    pub fn min(&self, section: &str) -> u64 {
        self.sections
            .iter()
            .find(|s| s.name == section)
            .map(|s| if s.min == u64::MAX { 0 } else { s.min })
            .unwrap_or(0)
    }
}

impl DetHash for Profiler {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.tick);
        hasher.write_u32(self.sections.len() as u32);
        for s in &self.sections {
            for b in s.name.as_bytes() {
                hasher.write_u32(*b as u32);
            }
            hasher.write_u64(s.total);
            hasher.write_u32(s.calls);
            hasher.write_u64(s.peak);
            hasher.write_u64(s.min);
        }
    }
}

// ---------------------------------------------------------------------------
// EventLog
// ---------------------------------------------------------------------------

/// A tick-stamped event entry.
#[derive(Clone, Debug)]
pub struct LogEntry<E> {
    pub tick: u32,
    pub event: E,
}

/// Bounded ring-buffer event log storing typed simulation events.
///
/// Oldest entries are silently dropped when capacity is reached.
/// Pair with `MsgLog` for human-readable messages; this type stores structured
/// data for analysis, assertions, or AI replay inspection.
#[derive(Clone, Debug)]
pub struct EventLog<E> {
    buf: Vec<Option<LogEntry<E>>>,
    head: usize,
    len: usize,
}

impl<E: Clone> EventLog<E> {
    pub fn new(capacity: usize) -> Self {
        EventLog {
            buf: vec![None; capacity.max(1)],
            head: 0,
            len: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Record an event at `tick`. Oldest entry is silently dropped if full.
    pub fn push(&mut self, tick: u32, event: E) {
        let cap = self.buf.len();
        let idx = (self.head + self.len) % cap;
        if self.len < cap {
            self.buf[idx] = Some(LogEntry { tick, event });
            self.len += 1;
        } else {
            // Overwrite oldest.
            self.buf[self.head] = Some(LogEntry { tick, event });
            self.head = (self.head + 1) % cap;
        }
    }

    /// Iterate entries oldest-first.
    pub fn iter(&self) -> impl Iterator<Item = &LogEntry<E>> {
        let cap = self.buf.len();
        let head = self.head;
        let len = self.len;
        (0..len).filter_map(move |i| self.buf[(head + i) % cap].as_ref())
    }

    /// The most recent `n` entries (up to `len`), oldest first.
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &LogEntry<E>> {
        let skip = self.len.saturating_sub(n);
        self.iter().skip(skip)
    }

    pub fn clear(&mut self) {
        for slot in &mut self.buf {
            *slot = None;
        }
        self.head = 0;
        self.len = 0;
    }

    /// Iterate entries whose tick falls within `[start_tick, end_tick]` (inclusive),
    /// oldest first. Useful for replays and test assertions like "what happened
    /// during turns 5–10?". Returns all entries if the log's ring buffer hasn't
    /// been pruned past this range.
    pub fn filter_by_tick_range(
        &self,
        start_tick: u32,
        end_tick: u32,
    ) -> impl Iterator<Item = &LogEntry<E>> {
        self.iter()
            .filter(move |e| e.tick >= start_tick && e.tick <= end_tick)
    }

    /// Count stored entries for which `pred(&event)` returns `true`.
    /// Memory-efficient alternative to `iter().filter(…).count()` that avoids
    /// constructing an intermediate collection.
    pub fn count_by<F: Fn(&E) -> bool>(&self, pred: F) -> usize {
        self.iter().filter(|entry| pred(&entry.event)).count()
    }

    /// The most recently pushed log entry, or `None` if the log is empty.
    /// Equivalent to `recent(1).last()` but avoids constructing an iterator.
    /// Useful for "show the last event" status lines without allocating.
    pub fn last(&self) -> Option<&LogEntry<E>> {
        if self.len == 0 {
            return None;
        }
        let cap = self.buf.len();
        let idx = (self.head + self.len - 1) % cap;
        self.buf[idx].as_ref()
    }

    /// The oldest (first pushed) entry still retained in the ring, or `None`
    /// if the log is empty. Complement of `last` — useful for "how long ago
    /// did the first visible event happen?" UI annotations.
    pub fn oldest(&self) -> Option<&LogEntry<E>> {
        if self.len == 0 {
            return None;
        }
        self.buf[self.head].as_ref()
    }
}

impl<E: Clone + DetHash> DetHash for EventLog<E> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.len as u32);
        for entry in self.iter() {
            hasher.write_u32(entry.tick);
            entry.event.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    // --- Profiler ---

    #[test]
    fn test_record_and_this_tick() {
        let mut p = Profiler::new(4);
        p.record("update", 100);
        p.record("update", 50);
        assert_eq!(p.this_tick("update"), 150);
    }

    #[test]
    fn test_unknown_section_returns_zero() {
        let p = Profiler::new(4);
        assert_eq!(p.this_tick("missing"), 0);
        assert_eq!(p.peak("missing"), 0);
    }

    #[test]
    fn test_peak_tracks_all_time_max() {
        let mut p = Profiler::new(4);
        p.record("render", 200);
        p.begin_tick();
        p.record("render", 50);
        assert_eq!(p.peak("render"), 200);
    }

    #[test]
    fn test_begin_tick_resets_this_tick() {
        let mut p = Profiler::new(4);
        p.record("ai", 300);
        p.begin_tick();
        assert_eq!(p.this_tick("ai"), 0);
    }

    #[test]
    fn test_tick_counter_increments() {
        let mut p = Profiler::new(4);
        assert_eq!(p.tick(), 0);
        p.begin_tick();
        p.begin_tick();
        assert_eq!(p.tick(), 2);
    }

    #[test]
    fn test_section_names() {
        let mut p = Profiler::new(4);
        p.record("a", 1);
        p.record("b", 2);
        let names: Vec<&str> = p.section_names().collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_avg_over_multiple_ticks() {
        let mut p = Profiler::new(4);
        p.record("work", 100);
        p.begin_tick();
        p.record("work", 200);
        p.begin_tick();
        // avg over 2 flushed ticks: (100 + 200) / 2 = 150
        assert_eq!(p.avg("work"), 150);
    }

    #[test]
    fn test_avg_unknown_section_returns_zero() {
        let p = Profiler::new(4);
        assert_eq!(p.avg("missing"), 0);
    }

    #[test]
    fn test_avg_no_history_returns_zero() {
        let mut p = Profiler::new(4);
        p.record("work", 100); // not yet flushed by begin_tick
        assert_eq!(p.avg("work"), 0);
    }

    #[test]
    fn test_avg_single_tick() {
        let mut p = Profiler::new(8);
        p.record("render", 300);
        p.begin_tick();
        assert_eq!(p.avg("render"), 300);
    }

    #[test]
    fn test_det_hash_same_profiler_same_hash() {
        let mut a = Profiler::new(4);
        let mut b = Profiler::new(4);
        a.record("x", 10);
        b.record("x", 10);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_budget_exceeded_true_when_over() {
        let mut p = Profiler::new(4);
        p.record("ai", 500);
        assert!(p.budget_exceeded("ai", 499));
    }

    #[test]
    fn test_budget_exceeded_false_when_equal_or_under() {
        let mut p = Profiler::new(4);
        p.record("ai", 100);
        assert!(!p.budget_exceeded("ai", 100)); // equal is not exceeded
        assert!(!p.budget_exceeded("ai", 200));
    }

    #[test]
    fn test_budget_exceeded_false_for_unknown_section() {
        let p = Profiler::new(4);
        assert!(!p.budget_exceeded("missing", 0));
    }

    // --- EventLog ---

    #[test]
    fn test_push_and_iter() {
        let mut log: EventLog<u32> = EventLog::new(4);
        log.push(1, 10);
        log.push(2, 20);
        let entries: Vec<u32> = log.iter().map(|e| e.event).collect();
        assert_eq!(entries, vec![10, 20]);
    }

    #[test]
    fn test_oldest_dropped_when_full() {
        let mut log: EventLog<u32> = EventLog::new(3);
        log.push(1, 1);
        log.push(2, 2);
        log.push(3, 3);
        log.push(4, 4); // drops entry 1
        let entries: Vec<u32> = log.iter().map(|e| e.event).collect();
        assert_eq!(entries, vec![2, 3, 4]);
    }

    #[test]
    fn test_recent() {
        let mut log: EventLog<u32> = EventLog::new(5);
        for i in 0..5u32 {
            log.push(i, i * 10);
        }
        let recent: Vec<u32> = log.recent(2).map(|e| e.event).collect();
        assert_eq!(recent, vec![30, 40]);
    }

    #[test]
    fn test_clear_empties_log() {
        let mut log: EventLog<u32> = EventLog::new(4);
        log.push(0, 99);
        log.clear();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_det_hash_same_log_same_hash() {
        let mut a: EventLog<u32> = EventLog::new(4);
        let mut b: EventLog<u32> = EventLog::new(4);
        a.push(1, 42);
        b.push(1, 42);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_filter_by_tick_range_returns_matching_entries() {
        let mut log: EventLog<&str> = EventLog::new(10);
        log.push(1, "a");
        log.push(3, "b");
        log.push(5, "c");
        log.push(7, "d");
        let events: Vec<&str> = log.filter_by_tick_range(3, 5).map(|e| e.event).collect();
        assert_eq!(events, vec!["b", "c"]);
    }

    #[test]
    fn test_filter_by_tick_range_empty_when_no_match() {
        let mut log: EventLog<u32> = EventLog::new(4);
        log.push(1, 10);
        log.push(2, 20);
        let events: Vec<u32> = log.filter_by_tick_range(10, 20).map(|e| e.event).collect();
        assert!(events.is_empty());
    }

    #[test]
    fn test_filter_by_tick_range_inclusive_bounds() {
        let mut log: EventLog<u32> = EventLog::new(4);
        log.push(5, 50);
        log.push(10, 100);
        let events: Vec<u32> = log.filter_by_tick_range(5, 10).map(|e| e.event).collect();
        assert_eq!(events, vec![50, 100]); // both endpoints included
    }

    #[test]
    fn test_det_hash_different_logs_differ() {
        let mut a: EventLog<u32> = EventLog::new(4);
        let mut b: EventLog<u32> = EventLog::new(4);
        a.push(1, 10);
        b.push(1, 20);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    // --- section_count ---

    #[test]
    fn test_section_count_zero_for_unknown() {
        let p = Profiler::new(4);
        assert_eq!(p.section_count("missing"), 0);
    }

    #[test]
    fn test_section_count_increments_with_record() {
        let mut p = Profiler::new(4);
        p.record("ai", 10);
        p.record("ai", 20);
        assert_eq!(p.section_count("ai"), 2);
    }

    #[test]
    fn test_section_count_resets_on_begin_tick() {
        let mut p = Profiler::new(4);
        p.record("ai", 10);
        p.record("ai", 10);
        p.begin_tick();
        assert_eq!(p.section_count("ai"), 0);
    }

    // --- min ---

    #[test]
    fn test_min_zero_for_unknown_section() {
        let p = Profiler::new(4);
        assert_eq!(p.min("missing"), 0);
    }

    #[test]
    fn test_min_tracks_all_time_minimum() {
        let mut p = Profiler::new(4);
        p.record("render", 200);
        p.record("render", 50);
        p.begin_tick();
        p.record("render", 300);
        // min across all ticks should be 50
        assert_eq!(p.min("render"), 50);
    }

    #[test]
    fn test_min_single_record() {
        let mut p = Profiler::new(4);
        p.record("work", 777);
        assert_eq!(p.min("work"), 777);
    }

    #[test]
    fn test_count_by_counts_matching_events() {
        let mut log: EventLog<u32> = EventLog::new(10);
        log.push(1, 10);
        log.push(2, 20);
        log.push(3, 10);
        log.push(4, 30);
        assert_eq!(log.count_by(|&e| e == 10), 2);
        assert_eq!(log.count_by(|&e| e == 20), 1);
        assert_eq!(log.count_by(|_| true), 4);
    }

    #[test]
    fn test_count_by_empty_log_returns_zero() {
        let log: EventLog<u32> = EventLog::new(4);
        assert_eq!(log.count_by(|_| true), 0);
    }

    #[test]
    fn test_count_by_no_match_returns_zero() {
        let mut log: EventLog<u32> = EventLog::new(4);
        log.push(1, 42);
        assert_eq!(log.count_by(|&e| e == 99), 0);
    }

    #[test]
    fn test_last_empty_log_returns_none() {
        let log: EventLog<u32> = EventLog::new(4);
        assert!(log.last().is_none());
    }

    #[test]
    fn test_last_returns_most_recently_pushed() {
        let mut log: EventLog<u32> = EventLog::new(4);
        log.push(1, 10);
        log.push(2, 20);
        log.push(3, 30);
        let entry = log.last().unwrap();
        assert_eq!(entry.tick, 3);
        assert_eq!(entry.event, 30);
    }

    #[test]
    fn test_last_is_consistent_with_recent_one() {
        let mut log: EventLog<u32> = EventLog::new(8);
        for i in 0..6u32 {
            log.push(i, i * 10);
        }
        let via_last = log.last().unwrap();
        let via_recent = log.recent(1).next().unwrap();
        assert_eq!(via_last.tick, via_recent.tick);
        assert_eq!(via_last.event, via_recent.event);
    }

    #[test]
    fn test_oldest_empty_log_is_none() {
        let log: EventLog<u32> = EventLog::new(4);
        assert!(log.oldest().is_none());
    }

    #[test]
    fn test_oldest_is_first_pushed() {
        let mut log: EventLog<u32> = EventLog::new(4);
        log.push(1, 100);
        log.push(2, 200);
        log.push(3, 300);
        let entry = log.oldest().unwrap();
        assert_eq!(entry.tick, 1);
        assert_eq!(entry.event, 100);
    }

    #[test]
    fn test_oldest_updates_when_overwritten() {
        let mut log: EventLog<u32> = EventLog::new(2);
        log.push(1, 10);
        log.push(2, 20);
        // Ring is full; pushing drops oldest (tick 1).
        log.push(3, 30);
        let entry = log.oldest().unwrap();
        assert_eq!(entry.tick, 2);
    }
}

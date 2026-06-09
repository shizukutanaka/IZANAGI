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
#[derive(Clone, Debug, Default)]
struct Section {
    name: &'static str,
    total: u64,
    calls: u32,
    peak: u64,
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
        } else {
            self.sections.push(Section {
                name: section,
                total: elapsed,
                calls: 1,
                peak: elapsed,
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
    fn test_det_hash_different_logs_differ() {
        let mut a: EventLog<u32> = EventLog::new(4);
        let mut b: EventLog<u32> = EventLog::new(4);
        a.push(1, 10);
        b.push(1, 20);
        assert_ne!(hash_state(&a), hash_state(&b));
    }
}

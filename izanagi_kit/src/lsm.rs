//! Log-structured merge index — a write-optimized ordered key-value
//! store as in LSM trees (O'Neil et al. 1996): an in-memory
//! `BTreeMap` memtable absorbs writes; when it fills, it is frozen
//! into a sorted immutable *run* and merged lazily into deeper
//! levels. Reads check the memtable first then the runs newest→oldest,
//! so the newest write always wins. Deletes are tombstones —
//! no in-place mutation means the whole structure is a pure function
//! of the operation log, which is exactly the deterministic contract.
//!
//! ```
//! use izanagi_kit::lsm::Lsm;
//! let mut t = Lsm::new(4); // flush at 4 live memtable entries
//! t.put(3, 30);
//! t.put(1, 10);
//! t.put(4, 40);
//! assert_eq!(t.get(&3), Some(30));
//! t.remove(&3);
//! assert_eq!(t.get(&3), None);
//! ```

use std::collections::BTreeMap;

/// Auto-compaction kicks in once this many runs coexist.
const MAX_RUNS: usize = 4;

/// Ordered `u64 → u64` index with LSM write path.
pub struct Lsm {
    /// Live memtable; `None` value = tombstone.
    mem: BTreeMap<u64, Option<u64>>,
    /// Frozen sorted runs, newest first.
    runs: Vec<Vec<(u64, Option<u64>)>>,
    /// Flush threshold for the memtable.
    capacity: usize,
}

impl Lsm {
    /// Empty index flushing after `capacity` memtable writes.
    pub fn new(capacity: usize) -> Self {
        Self {
            mem: BTreeMap::new(),
            runs: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    /// Insert or overwrite `key`.
    pub fn put(&mut self, key: u64, val: u64) {
        self.mem.insert(key, Some(val));
        if self.mem.len() >= self.capacity {
            self.flush();
        }
    }

    /// Delete `key` via a tombstone (returns previous visibility).
    pub fn remove(&mut self, key: &u64) -> bool {
        let was = self.get(key).is_some();
        self.mem.insert(*key, None);
        if self.mem.len() >= self.capacity {
            self.flush();
        }
        was
    }

    /// Point lookup — memtable first, then runs newest→oldest.
    pub fn get(&self, key: &u64) -> Option<u64> {
        if let Some(v) = self.mem.get(key) {
            return *v;
        }
        for run in &self.runs {
            match run.binary_search_by_key(key, |e| e.0) {
                Ok(i) => return run[i].1,
                Err(_) => continue,
            }
        }
        None
    }

    /// Merged ascending iteration — newest write wins per key,
    /// tombstones elided.
    pub fn iter(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        let mut acc: BTreeMap<u64, Option<u64>> = BTreeMap::new();
        // runs[] is newest-first: first write wins, so `or_insert`
        // keeps the newest value per key.
        for run in &self.runs {
            for &(k, v) in run {
                acc.entry(k).or_insert(v);
            }
        }
        for (&k, &v) in &self.mem {
            acc.insert(k, v);
        }
        acc.into_iter().filter_map(|(k, v)| v.map(|v| (k, v)))
    }

    /// Live (non-tombstone) entry count.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Freeze the memtable into a sorted run and merge when the
    /// tier overflows `MAX_RUNS`.
    pub fn flush(&mut self) {
        if self.mem.is_empty() {
            return;
        }
        let run: Vec<(u64, Option<u64>)> = std::mem::take(&mut self.mem).into_iter().collect();
        self.runs.insert(0, run);
        if self.runs.len() > MAX_RUNS {
            self.compact();
        }
    }

    /// Merge every run into one sorted run — the oldest level.
    pub fn compact(&mut self) {
        let mut merged: BTreeMap<u64, Option<u64>> = BTreeMap::new();
        // Oldest first so newer runs overwrite.
        for run in self.runs.iter().rev() {
            for &(k, v) in run {
                merged.insert(k, v);
            }
        }
        let one: Vec<(u64, Option<u64>)> = merged.into_iter().collect();
        self.runs.clear();
        if !one.is_empty() {
            self.runs.push(one);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basic() {
        let mut t = Lsm::new(8);
        assert!(t.is_empty());
        t.put(2, 20);
        t.put(1, 10);
        assert_eq!(t.get(&1), Some(10));
        assert_eq!(t.get(&9), None);
        assert_eq!(t.iter().collect::<Vec<_>>(), vec![(1, 10), (2, 20)]);
    }

    #[test]
    fn oracle_all_ops() {
        let mut rng = SplitMix64::new(23);
        for cap in [4usize, 16, 100] {
            let mut t = Lsm::new(cap);
            let mut b = BTreeMap::new();
            for _ in 0..1500 {
                let k = rng.below(200) as u64;
                match rng.below(4) {
                    0 | 1 => {
                        let v = rng.next_u64();
                        t.put(k, v);
                        b.insert(k, v);
                    }
                    2 => {
                        t.remove(&k);
                        b.remove(&k);
                    }
                    _ => assert_eq!(t.get(&k), b.get(&k).copied()),
                }
            }
            let got: Vec<(u64, u64)> = t.iter().collect();
            let want: Vec<(u64, u64)> = b.iter().map(|(&k, &v)| (k, v)).collect();
            assert_eq!(got, want);
            // Compaction preserves observable state.
            t.compact();
            let got2: Vec<(u64, u64)> = t.iter().collect();
            assert_eq!(got2, want);
        }
    }

    #[test]
    fn tombstone_newest_wins() {
        let mut t = Lsm::new(2);
        t.put(1, 10);
        t.flush();
        t.put(1, 11);
        t.flush();
        assert_eq!(t.get(&1), Some(11));
        t.remove(&1);
        t.flush();
        assert_eq!(t.get(&1), None);
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn interleaved_flush_iter() {
        let mut t = Lsm::new(3);
        let mut b = BTreeMap::new();
        for k in 0..30u64 {
            t.put(k, k * 10);
            b.insert(k, k * 10);
        }
        let got: Vec<(u64, u64)> = t.iter().collect();
        let want: Vec<(u64, u64)> = b.iter().map(|(&k, &v)| (k, v)).collect();
        assert_eq!(got, want);
    }
}

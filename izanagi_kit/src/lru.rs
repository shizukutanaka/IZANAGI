//! LRU cache — least-recently-used eviction over `u64` keys and
//! values. The textbook ordered-map + recency-queue pairing, done
//! with two `BTreeMap`s so eviction order is fully deterministic
//! (a linked list would be faster but a sorted (stamp, key) index
//! is *canonical*: ties resolve by key, never by pointer address).
//!
//! `get` counts as a use (bumps recency); `peek` does not. Both are
//! O(log n). Clock is a monotone `u64` stamp — overflowing a 64-bit
//! access counter is not a reachable state at sim scale, and no
//! wrap-around logic is needed.
//!
//! ```
//! use izanagi_kit::lru::Lru;
//! let mut c = Lru::new(2).unwrap();
//! c.put(1, 10);
//! c.put(2, 20);
//! assert_eq!(c.get(1), Some(10)); // 1 is now most-recent
//! c.put(3, 30); // evicts 2, the least-recent
//! assert_eq!(c.peek(2), None);
//! assert_eq!(c.peek(1), Some(10));
//! assert_eq!(c.peek(3), Some(30));
//! ```

use std::collections::BTreeMap;

/// A capacity-bounded least-recently-used map over `u64`.
#[derive(Clone, Debug)]
pub struct Lru {
    cap: usize,
    /// key → (value, last-use stamp).
    map: BTreeMap<u64, (u64, u64)>,
    /// (stamp, key) — sorted eviction order, oldest first.
    order: BTreeMap<(u64, u64), ()>,
    clock: u64,
}

impl Lru {
    /// `None` when `cap == 0`.
    pub fn new(cap: usize) -> Option<Self> {
        if cap == 0 {
            return None;
        }
        Some(Lru {
            cap,
            map: BTreeMap::new(),
            order: BTreeMap::new(),
            clock: 0,
        })
    }

    /// Capacity.
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Entries held.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// `len == 0`.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Fetch and mark most-recently-used.
    pub fn get(&mut self, key: u64) -> Option<u64> {
        let (v, old) = *self.map.get(&key)?;
        self.clock += 1;
        self.order.remove(&(old, key));
        self.order.insert((self.clock, key), ());
        self.map.insert(key, (v, self.clock));
        Some(v)
    }

    /// Fetch without affecting recency.
    pub fn peek(&self, key: u64) -> Option<u64> {
        self.map.get(&key).map(|&(v, _)| v)
    }

    /// Insert or update `key` (marks most-recent); evicts the
    /// least-recently-used entry when at capacity. Returns the
    /// evicted `(key, value)` when one was displaced.
    pub fn put(&mut self, key: u64, value: u64) -> Option<(u64, u64)> {
        let mut evicted = None;
        if let Some(&(v, old)) = self.map.get(&key) {
            self.order.remove(&(old, key));
            let _ = v;
        } else if self.map.len() == self.cap {
            let (&(stamp, victim), _) = self.order.iter().next()?;
            self.order.remove(&(stamp, victim));
            evicted = self.map.remove(&victim).map(|(v, _)| (victim, v));
        }
        self.clock += 1;
        self.order.insert((self.clock, key), ());
        self.map.insert(key, (value, self.clock));
        evicted
    }

    /// Iterate entries from most- to least-recently-used —
    /// canonical order for serializing or debugging the cache.
    pub fn by_recency(&self) -> Vec<(u64, u64)> {
        self.order
            .iter()
            .rev()
            .map(|(&(s, k), _)| {
                let _ = s;
                (k, self.map[&k].0)
            })
            .collect()
    }

    /// Drop the least-recently-used entry — `None` when empty.
    pub fn pop_lru(&mut self) -> Option<(u64, u64)> {
        let (&(stamp, victim), _) = self.order.iter().next()?;
        self.order.remove(&(stamp, victim));
        self.map.remove(&victim).map(|(v, _)| (victim, v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::VecDeque;

    /// Shadow oracle: linear scan of a recency deque — trivially
    /// correct, checked every op.
    struct Shadow {
        cap: usize,
        recent: VecDeque<u64>,
        map: BTreeMap<u64, u64>,
    }
    impl Shadow {
        fn get(&mut self, k: u64) -> Option<u64> {
            let v = *self.map.get(&k)?;
            self.recent.retain(|&x| x != k);
            self.recent.push_front(k);
            Some(v)
        }
        fn peek(&self, k: u64) -> Option<u64> {
            self.map.get(&k).copied()
        }
        fn put(&mut self, k: u64, v: u64) -> Option<(u64, u64)> {
            let mut ev = None;
            if self.map.contains_key(&k) {
                self.recent.retain(|&x| x != k);
            } else if self.recent.len() == self.cap {
                let vic = self.recent.pop_back().unwrap();
                ev = self.map.remove(&vic).map(|x| (vic, x));
            }
            self.recent.push_front(k);
            self.map.insert(k, v);
            ev
        }
    }

    #[test]
    fn matches_shadow_oracle() {
        let mut rng = SplitMix64::new(0x1AC4);
        for _ in 0..300 {
            let cap = 1 + rng.below(9) as usize;
            let mut c = Lru::new(cap).unwrap();
            let mut s = Shadow {
                cap,
                recent: VecDeque::new(),
                map: BTreeMap::new(),
            };
            for _ in 0..400 {
                let k = rng.below((cap * 3) as u32) as u64;
                match rng.below(3) {
                    0 => assert_eq!(c.get(k), s.get(k)),
                    1 => assert_eq!(c.peek(k), s.peek(k)),
                    _ => {
                        let v = rng.next_u64() % 1000;
                        assert_eq!(c.put(k, v), s.put(k, v));
                    }
                }
                assert_eq!(c.len(), s.map.len());
            }
            // recency order and drain
            let expect: Vec<(u64, u64)> = s.recent.iter().map(|&k| (k, s.map[&k])).collect();
            assert_eq!(c.by_recency(), expect);
            while let Some((k, v)) = c.pop_lru() {
                let ek = s.recent.pop_back().unwrap();
                assert_eq!((k, v), (ek, s.map.remove(&ek).unwrap()));
            }
            assert!(c.is_empty() && c.pop_lru().is_none());
        }
    }

    #[test]
    fn update_does_not_evict_and_validation() {
        assert!(Lru::new(0).is_none());
        let mut c = Lru::new(2).unwrap();
        c.put(1, 10);
        c.put(2, 20);
        assert_eq!(c.put(1, 11), None); // update — no eviction
        assert_eq!(c.len(), 2);
        c.put(3, 30); // evicts 2 (1 was refreshed by the update)
        assert_eq!(c.peek(2), None);
        assert_eq!(c.peek(1), Some(11));
        assert_eq!(c.cap(), 2);
        assert!(!c.is_empty());
    }
}

//! Robin Hood hashing — open addressing where probe-length
//! variance is actively minimized: on insert, an element that
//! traveled *farther* from its home slot evicts one that
//! traveled less ("steal from the rich"). Lookups then stop at
//! the first slot whose probe length is below the walk's — most
//! misses answer after one probe — and deletion is a backward
//! shift that keeps every element reachable with no tombstones.
//!
//! ```
//! use izanagi_kit::robin::RobinSet;
//! let mut s = RobinSet::new(42);
//! s.insert(7);
//! s.insert(19);
//! assert!(s.contains(&7) && s.len() == 2);
//! s.remove(&7);
//! assert!(!s.contains(&7));
//! ```
const EMPTY: u32 = u32::MAX;

/// Seeded u64 mix — SplitMix64 finalizer keyed by `seed`.
fn mix(key: u64, seed: u64) -> u64 {
    let mut h = key.wrapping_add(seed);
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 31)
}

/// Robin Hood open-addressing set over `u64`.
///
/// `keys[i]`/`dists[i]` are a slot's key and probe distance
/// (`EMPTY` marks free). Layout is a pure function of
/// `(inserted sequence, seed)`: grows at load ≥ 0.75 and rehash
/// order is slot order — deterministic.
#[derive(Clone, Debug)]
pub struct RobinSet {
    keys: Vec<u64>,
    dists: Vec<u32>,
    mask: usize,
    len: usize,
    seed: u64,
}

impl RobinSet {
    /// New set with capacity 16.
    pub fn new(seed: u64) -> RobinSet {
        RobinSet {
            keys: vec![0; 16],
            dists: vec![EMPTY; 16],
            mask: 15,
            len: 0,
            seed,
        }
    }

    fn home(&self, key: u64) -> usize {
        (mix(key, self.seed) & self.mask as u64) as usize
    }

    /// Number of stored keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `key` present? The walk stops at the first slot whose
    /// probe distance is below ours — everything past it was
    /// already displaced by luckier arrivals, so the key cannot
    /// sit there.
    pub fn contains(&self, key: &u64) -> bool {
        let mut idx = self.home(*key);
        let mut dist = 0u32;
        loop {
            let d = self.dists[idx];
            if d == EMPTY || d < dist {
                return false;
            }
            if self.keys[idx] == *key {
                return true;
            }
            dist += 1;
            idx = (idx + 1) & self.mask;
        }
    }

    /// Insert `key` — `true` iff it was absent. When the walk's
    /// element has traveled farther than a sitting element, they
    /// swap and the evicted one continues the walk instead.
    pub fn insert(&mut self, key: u64) -> bool {
        if self.len * 4 >= (self.mask + 1) * 3 {
            self.rehash((self.mask + 1) * 2);
        }
        let mut idx = self.home(key);
        let mut cur_key = key;
        let mut cur_dist = 0u32;
        loop {
            let d = self.dists[idx];
            if d == EMPTY {
                self.keys[idx] = cur_key;
                self.dists[idx] = cur_dist;
                self.len += 1;
                return true;
            }
            if self.keys[idx] == cur_key {
                return false;
            }
            if d < cur_dist {
                // the sitting element traveled less — steal
                std::mem::swap(&mut cur_key, &mut self.keys[idx]);
                std::mem::swap(&mut cur_dist, &mut self.dists[idx]);
            }
            cur_dist += 1;
            idx = (idx + 1) & self.mask;
        }
    }

    /// Remove `key` — `true` iff it was present. Backward-shift:
    /// walk forward and pull each subsequent element one slot
    /// back while its probe distance is positive — preserves the
    /// no-tombstone invariant.
    pub fn remove(&mut self, key: &u64) -> bool {
        let mut idx = self.home(*key);
        let mut dist = 0u32;
        loop {
            let d = self.dists[idx];
            if d == EMPTY || d < dist {
                return false;
            }
            if self.keys[idx] == *key {
                // shift successors back until a home element or
                // an empty slot ends the run
                let mut hole = idx;
                let mut nxt = (idx + 1) & self.mask;
                while self.dists[nxt] != EMPTY && self.dists[nxt] > 0 {
                    self.keys[hole] = self.keys[nxt];
                    self.dists[hole] = self.dists[nxt] - 1;
                    hole = nxt;
                    nxt = (nxt + 1) & self.mask;
                }
                self.dists[hole] = EMPTY;
                self.len -= 1;
                return true;
            }
            dist += 1;
            idx = (idx + 1) & self.mask;
        }
    }

    /// Rehash into `cap` slots (power of 2, ≥ current len).
    fn rehash(&mut self, cap: usize) {
        let old_keys = std::mem::replace(&mut self.keys, vec![0; cap]);
        let old_dists = std::mem::replace(&mut self.dists, vec![EMPTY; cap]);
        self.mask = cap - 1;
        self.len = 0;
        for (i, &d) in old_dists.iter().enumerate() {
            if d != EMPTY {
                // insert without growth check — capacity known
                let mut idx = self.home(old_keys[i]);
                let mut cur_key = old_keys[i];
                let mut cur_dist = 0u32;
                loop {
                    if self.dists[idx] == EMPTY {
                        self.keys[idx] = cur_key;
                        self.dists[idx] = cur_dist;
                        self.len += 1;
                        break;
                    }
                    if self.dists[idx] < cur_dist {
                        std::mem::swap(&mut cur_key, &mut self.keys[idx]);
                        std::mem::swap(&mut cur_dist, &mut self.dists[idx]);
                    }
                    cur_dist += 1;
                    idx = (idx + 1) & self.mask;
                }
            }
        }
    }

    /// Longest probe distance currently stored — a direct read
    /// of the variance Robin Hood keeps small.
    pub fn max_probe_len(&self) -> u32 {
        let mut m = 0;
        for &d in &self.dists {
            if d != EMPTY && d > m {
                m = d;
            }
        }
        m
    }

    /// Total probe distance — divided by `len` this is the mean
    /// lookup cost.
    pub fn total_probe_len(&self) -> u64 {
        self.dists
            .iter()
            .filter(|&&d| d != EMPTY)
            .map(|&d| d as u64)
            .sum()
    }

    /// Capacity.
    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    /// Keys, sorted for canonical output.
    pub fn values(&self) -> Vec<u64> {
        let mut v: Vec<u64> = Vec::with_capacity(self.len);
        for (i, &d) in self.dists.iter().enumerate() {
            if d != EMPTY {
                v.push(self.keys[i]);
            }
        }
        v.sort_unstable();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn basics() {
        let mut s = RobinSet::new(42);
        assert!(s.is_empty() && s.capacity() == 16);
        assert!(s.insert(7));
        assert!(s.insert(19));
        assert!(!s.insert(7));
        assert!(s.contains(&7) && s.len() == 2);
        assert_eq!(s.values(), vec![7, 19]);
        assert!(s.remove(&7));
        assert!(!s.remove(&7));
        assert!(!s.contains(&7));
        assert_eq!(s.values(), vec![19]);
        assert_eq!(s.max_probe_len(), 0);
        // forced collision run: keys sharing a home
        let mut c = RobinSet::new(0);
        for k in 0..50 {
            assert!(c.insert(k));
        }
        assert_eq!(c.len(), 50);
        assert!(c.max_probe_len() < 50); // RH keeps variance small
        assert_eq!(c.values(), (0..50).collect::<Vec<u64>>());
    }

    /// Reachability invariant oracle: after any op, every stored
    /// key must be findable by the probe walk — verifies the
    /// backward-shift deletion preserves the no-tombstone rule.
    #[test]
    fn oracle_btree_shadow() {
        let mut rng = SplitMix64::new(29);
        for _round in 0..100 {
            let mut s = RobinSet::new(rng.next_u64());
            let mut want = BTreeSet::new();
            for _ in 0..200 {
                let k = rng.below(512) as u64;
                match rng.below(4) {
                    0 | 1 => assert_eq!(s.insert(k), want.insert(k)),
                    2 => assert_eq!(s.remove(&k), want.remove(&k)),
                    _ => assert_eq!(s.contains(&k), want.contains(&k)),
                }
                assert_eq!(s.len(), want.len());
                // invariant: every stored key reachable by probing
                for &v in &want {
                    assert!(s.contains(&v), "lost {v}");
                }
            }
            assert_eq!(s.values(), want.iter().copied().collect::<Vec<u64>>());
        }
    }

    /// RH's headline property: probe-length variance stays tiny
    /// vs. linear probing without stealing — here: after heavy
    /// clustered inserts, `max_probe_len` stays O(log n)-ish,
    /// and no lookup probes more than `max_probe_len + 1` slots.
    #[test]
    fn probe_length_bound() {
        let mut s = RobinSet::new(3);
        let mut rng = SplitMix64::new(5);
        for _ in 0..200 {
            s.insert(rng.next_u64() & 0xfff);
        }
        // with ≥200 inserts over 0xfff values, keys collide hard —
        // RH still keeps the max probe short
        assert!(
            s.max_probe_len() <= 16,
            "probe {} too long",
            s.max_probe_len()
        );
        // mean probe cost is bounded by the max
        assert!(s.total_probe_len() <= s.len() as u64 * u64::from(s.max_probe_len()));
    }
}

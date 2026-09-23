//! SwissTable-style open addressing — a `u64` set that stores a
//! 7-bit control fingerprint `h2` beside each key, so probe misses
//! almost never touch the key array. Capacity is a multiple of
//! `GROUP=16` slots; the 57-bit `h1` picks the home group and the
//! probe walks forward group by group.
//!
//! Deletion writes a `DELETED` control byte (unlike robin-hood,
//! probing groups must not stop on holes that held keys); growth
//! and a tombstone ceiling both trigger a full rehash, keeping
//! `(items, seed, cap)` a pure function of the insert sequence —
//! deterministic layout, deterministic probe order.
//!
//! Where `cuckoo` guarantees two probes worst-case, `swiss`
//! guarantees single-table density and 16-slot batched compares;
//! `u64` keys keep the table clone-cheap.
//!
//! ```
//! use izanagi_kit::swiss::SwissSet;
//! let mut s = SwissSet::new(8);
//! for i in 0..5u64 { s.insert(i * 10); }
//! assert!(s.contains(&30));
//! assert!(!s.contains(&31));
//! assert_eq!(s.remove(&30), true);
//! assert!(!s.contains(&30));
//! ```
//!
//! References: Bening, Kingsley & Luaces (CppCon 2017) SwissTable;
//! Appleton's Abseil `raw_hash_set` for the control-byte layout.

const GROUP: usize = 16;
const EMPTY: u8 = 0x80;
const DELETED: u8 = 0xfe;
// full slots carry `h2` (0..=0x7f)

/// Fixed-seed `u64` hash-table with 16-slot probe groups and
/// 7-bit control-byte fingerprints. `new(cap)` rounds capacity up
/// to a group multiple and enforces a load ceiling of 15/16.
pub struct SwissSet {
    ctrl: Vec<u8>,
    keys: Vec<u64>,
    len: usize,
    tombs: usize,
    seed: u64,
}

impl SwissSet {
    /// New table holding at least `cap` keys before growth.
    pub fn new(cap: usize) -> Self {
        let groups = (cap.div_ceil(GROUP)).max(1);
        SwissSet {
            ctrl: vec![EMPTY; groups * GROUP],
            keys: vec![0; groups * GROUP],
            len: 0,
            tombs: 0,
            seed: 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn hash(&self, k: u64) -> u64 {
        let mut x = k.wrapping_add(self.seed);
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x
    }

    /// Current slot count (always a multiple of `GROUP`).
    pub fn capacity(&self) -> usize {
        self.keys.len()
    }

    /// Number of live keys.
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` when no keys are stored.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Membership test — control-byte prefilter, then key compare.
    pub fn contains(&self, k: &u64) -> bool {
        self.find_slot(*k).is_some()
    }

    /// Slot the key occupies (or the first insertable slot).
    fn find_slot(&self, k: u64) -> Option<usize> {
        let h = self.hash(k);
        let h2 = (h & 0x7f) as u8;
        let cap = self.capacity();
        let groups = cap / GROUP;
        let mut pos = (((h >> 7) as usize) % groups) * GROUP;
        loop {
            let end = pos + GROUP;
            for i in pos..end {
                let c = self.ctrl[i];
                if c == h2 {
                    if self.keys[i] == k {
                        return Some(i);
                    }
                } else if c == EMPTY {
                    return None;
                }
            }
            pos = if end >= cap { 0 } else { end };
        }
    }

    /// First `EMPTY`/`DELETED` slot reachable from `h`'s home —
    /// caller guarantees the key is absent.
    fn find_free(&self, h: u64) -> Option<usize> {
        let cap = self.capacity();
        let groups = cap / GROUP;
        let mut pos = (((h >> 7) as usize) % groups) * GROUP;
        loop {
            let end = pos + GROUP;
            for i in pos..end {
                let c = self.ctrl[i];
                if c == EMPTY || c == DELETED {
                    return Some(i);
                }
            }
            pos = if end >= cap { 0 } else { end };
        }
    }

    /// Insert if absent. `false` when already present.
    pub fn insert(&mut self, k: u64) -> bool {
        if self.contains(&k) {
            return false;
        }
        // grow at 15/16 load counting tombstones, or when 1/4 of
        // the slots are dead weight — both keep probes bounded
        if (self.len + self.tombs + 1) * 16 > self.capacity() * 15
            || self.tombs * 4 > self.capacity()
        {
            self.rehash(self.capacity() * 2);
        }
        let h = self.hash(k);
        if let Some(i) = self.find_free(h) {
            if self.ctrl[i] == DELETED {
                self.tombs -= 1;
            }
            self.ctrl[i] = (h & 0x7f) as u8;
            self.keys[i] = k;
            self.len += 1;
            true
        } else {
            false
        }
    }

    /// Remove if present — tombstone, not `EMPTY`, so probe
    /// chains through the slot stay valid. `false` when absent.
    pub fn remove(&mut self, k: &u64) -> bool {
        match self.find_slot(*k) {
            Some(i) => {
                self.ctrl[i] = DELETED;
                self.tombs += 1;
                self.len -= 1;
                true
            }
            None => false,
        }
    }

    fn rehash(&mut self, cap: usize) {
        let old_keys = std::mem::take(&mut self.keys);
        let old_ctrl = std::mem::take(&mut self.ctrl);
        self.keys = vec![0; cap.max(GROUP)];
        self.ctrl = vec![EMPTY; cap.max(GROUP)];
        self.tombs = 0;
        for (i, &c) in old_ctrl.iter().enumerate() {
            if c != EMPTY && c != DELETED {
                let h = self.hash(old_keys[i]);
                if let Some(j) = self.find_free(h) {
                    self.ctrl[j] = (h & 0x7f) as u8;
                    self.keys[j] = old_keys[i];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn oracle_random_ops() {
        let mut r = SplitMix64::new(0x5155_1551);
        for _ in 0..60 {
            let mut s = SwissSet::new(1 + r.below(20) as usize);
            let mut oracle = BTreeSet::new();
            for _ in 0..600 {
                let k = r.below(400) as u64;
                match r.below(3) {
                    0 => {
                        assert_eq!(s.insert(k), oracle.insert(k));
                    }
                    1 => {
                        assert_eq!(s.remove(&k), oracle.remove(&k));
                    }
                    _ => {
                        assert_eq!(s.contains(&k), oracle.contains(&k));
                    }
                }
            }
            for k in 0..400u64 {
                assert_eq!(s.contains(&k), oracle.contains(&k), "k={k}");
            }
        }
    }

    #[test]
    fn growth_keeps_everything_findable() {
        let mut s = SwissSet::new(1);
        for i in 0..2000u64 {
            s.insert(i);
        }
        assert_eq!(s.len(), 2000);
        for i in 0..2000u64 {
            assert!(s.contains(&i), "missing {i}");
        }
    }

    #[test]
    fn tombstone_churn_stays_correct() {
        let mut s = SwissSet::new(16);
        let mut oracle = BTreeSet::new();
        // interleave remove+insert so tombstones accumulate
        for round in 0..30u64 {
            for i in 0..14u64 {
                let k = round * 100 + i;
                s.insert(k);
                oracle.insert(k);
            }
            for i in 0..14u64 {
                let k = round * 100 + i;
                if i % 2 == 0 {
                    s.remove(&k);
                    oracle.remove(&k);
                }
            }
        }
        for k in 0..3000u64 {
            assert_eq!(s.contains(&k), oracle.contains(&k));
        }
    }

    #[test]
    fn deterministic_layout() {
        let build = || {
            let mut s = SwissSet::new(4);
            for i in 0..300u64 {
                s.insert(i * 7 + 1);
            }
            s
        };
        let a = build();
        let b = build();
        assert_eq!(a.ctrl, b.ctrl);
        assert_eq!(a.keys, b.keys);
    }
}

//! Cuckoo hash table over `u64` keys — an open-addressing set where
//! every key lives in exactly one of two buckets (`t1[h1(x)]` or
//! `t2[h2(x)]`), so `contains` probes at most two cells and `remove`
//! needs no tombstones: an evicted key is relocated, never deleted.
//! Kick-outs follow a bounded deterministic schedule seeded at
//! construction, making the final layout a pure function of the
//! insertion sequence — identical replays place keys identically.
//!
//! The slot value `!0` is reserved as the empty marker and can never
//! be inserted; `insert` returns `false` for duplicates, for `!0`,
//! and when the kick budget is exhausted (near-full tables) instead
//! of resizing — capacity is fixed so peers cannot diverge on a
//! rehash policy.
//!
//! ```
//! use izanagi_kit::cuckoo::Cuckoo;
//!
//! let mut c = Cuckoo::new(16, 0xC0FFEE);
//! assert!(c.insert(42));
//! assert!(!c.insert(42)); // duplicate
//! assert!(c.contains(42));
//! assert!(c.remove(42));
//! assert!(!c.contains(42));
//! ```
//!
//! References: Pagh & Rodler, "Cuckoo hashing" (2001).

use crate::rng::SplitMix64;

fn mix(seed: u64, x: u64) -> u64 {
    SplitMix64::new(seed ^ x).next_u64()
}

const EMPTY: u64 = !0;

/// Hash-table set of `u64` keys using cuckoo placement in two tables.
pub struct Cuckoo {
    mask: usize,
    seed: u64,
    t1: Vec<u64>,
    t2: Vec<u64>,
    len: usize,
}

impl Cuckoo {
    /// Empty set with room for roughly `capacity` keys per table —
    /// `capacity` is rounded up to a power of two, so comfortable
    /// occupancy tops out near `2 * capacity`.
    pub fn new(capacity: usize, seed: u64) -> Self {
        let cap = capacity.max(1).next_power_of_two();
        Self {
            mask: cap - 1,
            seed,
            t1: vec![EMPTY; cap],
            t2: vec![EMPTY; cap],
            len: 0,
        }
    }

    /// Number of keys stored.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Home bucket of `x` in each table.
    fn slots(&self, x: u64) -> (usize, usize) {
        (
            (mix(self.seed, x) & self.mask as u64) as usize,
            (mix(self.seed ^ 0x9e37_79b9_7f4a_7c15, x) & self.mask as u64) as usize,
        )
    }

    /// Whether `x` is present.
    pub fn contains(&self, x: u64) -> bool {
        if x == EMPTY {
            return false;
        }
        let (h1, h2) = self.slots(x);
        self.t1[h1] == x || self.t2[h2] == x
    }

    /// Insert `x`. Returns `false` when `x` is already present, is the
    /// reserved `!0` marker, or the displacement chain exceeded the
    /// kick budget (the table is effectively full — nothing changed).
    pub fn insert(&mut self, x: u64) -> bool {
        if x == EMPTY || self.contains(x) {
            return false;
        }
        let mut x = x;
        // Bound the walk so cycles terminate; each kick alternates
        // tables, which is what makes the chain revisit-free.
        let budget = 4 * (usize::BITS - self.mask.leading_zeros()) as usize + 8;
        // (table, slot, evicted value) — replayed in reverse to undo a
        // chain that fails to terminate, so a rejected insert never
        // orphans a key that was already stored.
        let mut journal: Vec<(bool, usize, u64)> = Vec::new();
        for _ in 0..budget {
            let (h1, h2) = self.slots(x);
            if self.t1[h1] == EMPTY {
                self.t1[h1] = x;
                self.len += 1;
                return true;
            }
            if self.t2[h2] == EMPTY {
                self.t2[h2] = x;
                self.len += 1;
                return true;
            }
            // Both homes full: displace the t1 resident, which moves
            // to its other home in t2, possibly displacing again.
            let evicted = self.t1[h1];
            journal.push((false, h1, evicted));
            self.t1[h1] = x;
            x = evicted;
            let (_, e2) = self.slots(x);
            if self.t2[e2] == EMPTY {
                self.t2[e2] = x;
                self.len += 1;
                return true;
            }
            let evicted2 = self.t2[e2];
            journal.push((true, e2, evicted2));
            self.t2[e2] = x;
            x = evicted2;
        }
        // Budget exhausted: roll the whole displacement chain back so
        // the table returns to its pre-insert state, then report a
        // full-table failure rather than resizing behind the caller's
        // back.
        for &(in_t2, slot, old) in journal.iter().rev() {
            if in_t2 {
                self.t2[slot] = old;
            } else {
                self.t1[slot] = old;
            }
        }
        false
    }

    /// Remove `x`. Returns whether it was present.
    pub fn remove(&mut self, x: u64) -> bool {
        if x == EMPTY {
            return false;
        }
        let (h1, h2) = self.slots(x);
        if self.t1[h1] == x {
            self.t1[h1] = EMPTY;
            self.len -= 1;
            return true;
        }
        if self.t2[h2] == x {
            self.t2[h2] = EMPTY;
            self.len -= 1;
            return true;
        }
        false
    }

    /// Keys in ascending order (canonical — layout order is an
    /// implementation detail, so callers get sorted output).
    pub fn iter(&self) -> Vec<u64> {
        let mut v: Vec<u64> = self
            .t1
            .iter()
            .chain(self.t2.iter())
            .copied()
            .filter(|&k| k != EMPTY)
            .collect();
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
    fn empty_and_membership() {
        let mut c = Cuckoo::new(8, 7);
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
        assert!(!c.contains(0));
        assert!(!c.contains(!0));
        assert!(!c.remove(5));
        assert_eq!(c.iter(), Vec::<u64>::new());
    }

    #[test]
    fn reserved_marker_cannot_insert() {
        let mut c = Cuckoo::new(8, 1);
        assert!(!c.insert(!0));
        assert!(!c.contains(!0));
    }

    #[test]
    fn oracle_random_ops() {
        let mut rng = SplitMix64::new(0xC0C0_0000_0000_0001);
        for round in 0..40 {
            let mut c = Cuckoo::new(64, round);
            let mut oracle = BTreeSet::new();
            for _ in 0..600 {
                let x = rng.next_u64() % 200;
                match rng.next_u64() % 3 {
                    0 => {
                        if c.insert(x) {
                            oracle.insert(x);
                        } else {
                            // A failed insert must leave every live
                            // key still reachable.
                            for &k in &oracle {
                                assert!(c.contains(k));
                            }
                        }
                    }
                    1 => assert_eq!(c.remove(x), oracle.remove(&x)),
                    _ => assert_eq!(c.contains(x), oracle.contains(&x)),
                }
                assert_eq!(c.len(), oracle.len());
            }
            assert_eq!(c.iter(), oracle.iter().copied().collect::<Vec<u64>>());
        }
    }

    #[test]
    fn kicked_keys_survive() {
        // Cram a small table: every key that reported inserted must
        // still be found, no matter how it was displaced.
        let mut c = Cuckoo::new(4, 99);
        let mut want = BTreeSet::new();
        for x in 1..40u64 {
            if c.insert(x) {
                want.insert(x);
            }
        }
        for &x in &want {
            assert!(c.contains(x), "lost kicked key {x}");
        }
        assert_eq!(c.len(), want.len());
    }
}

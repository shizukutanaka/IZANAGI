//! Generational slot map — stable `u64` handles over a dense
//! store. A handle packs `(slot: u32, generation: u32)`; removing
//! bumps the generation so every stale copy of the handle is
//! *structurally* rejected — no use-after-free, ever, which is the
//! entire reason slot maps exist for entity stores. Free slots
//! recycle in LIFO order (canonical for a given op sequence); a
//! slot whose generation wraps `u32::MAX` is permanently retired
//! rather than allowed to alias.
//!
//! Iteration is in slot order — canonical, insertion-order
//! independent. The engine-side analogue of
//! [`crate::sparse_set`] for pure data.
//!
//! ```
//! use izanagi_kit::slotmap::Slotmap;
//! let mut s = Slotmap::new();
//! let h = s.insert(10).unwrap();
//! assert_eq!(s.get(h), Some(10));
//! assert_eq!(s.remove(h), Some(10));
//! assert_eq!(s.get(h), None); // stale handle rejected
//! let h2 = s.insert(20).unwrap();
//! assert_ne!(h, h2);          // same slot, newer generation
//! ```

/// An entry in the dense store: live `Some(v)` or a tombstone.
#[derive(Clone, Copy, Debug)]
struct Slot {
    gen: u32,
    val: Option<u64>,
}

/// A dense generational map from `u64` handles to `u64` values.
#[derive(Clone, Debug, Default)]
pub struct Slotmap {
    slots: Vec<Slot>,
    free: Vec<u32>,
    len: usize,
}

impl Slotmap {
    /// Empty map.
    pub fn new() -> Self {
        Slotmap {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    /// Live entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// `len == 0`.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Allocate a handle for `v` — returns the packed `u64`.
    /// `None` only if all 2³² slots and generations are exhausted
    /// (unreachable at sim scale).
    pub fn insert(&mut self, v: u64) -> Option<u64> {
        if let Some(slot) = self.free.pop() {
            let s = &mut self.slots[slot as usize];
            s.val = Some(v);
            self.len += 1;
            Some(((slot as u64) << 32) | s.gen as u64)
        } else {
            if self.slots.len() >= u32::MAX as usize {
                return None;
            }
            let slot = self.slots.len() as u32;
            self.slots.push(Slot {
                gen: 0,
                val: Some(v),
            });
            self.len += 1;
            Some((slot as u64) << 32)
        }
    }

    /// Whether `handle` currently resolves.
    pub fn contains(&self, handle: u64) -> bool {
        let (slot, gen) = split(handle);
        self.slots
            .get(slot as usize)
            .is_some_and(|s| s.gen == gen && s.val.is_some())
    }

    /// Fetch `handle`'s value — `None` when stale or unknown.
    pub fn get(&self, handle: u64) -> Option<u64> {
        let (slot, gen) = split(handle);
        let s = self.slots.get(slot as usize)?;
        if s.gen == gen {
            s.val
        } else {
            None
        }
    }

    /// Overwrite `handle`'s value — `false` when stale or unknown.
    pub fn set(&mut self, handle: u64, v: u64) -> bool {
        let (slot, gen) = split(handle);
        match self.slots.get_mut(slot as usize) {
            Some(s) if s.gen == gen && s.val.is_some() => {
                s.val = Some(v);
                true
            }
            _ => false,
        }
    }

    /// Remove `handle` — its value, or `None` when stale/unknown.
    /// The slot recycles for the next `insert` unless its
    /// generation wrapped, in which case it retires permanently.
    pub fn remove(&mut self, handle: u64) -> Option<u64> {
        let (slot, gen) = split(handle);
        let s = self.slots.get_mut(slot as usize)?;
        if s.gen != gen {
            return None;
        }
        let v = s.val.take()?;
        self.len -= 1;
        if s.gen == u32::MAX {
            // generation space exhausted — retire, never recycle
            return Some(v);
        }
        s.gen += 1;
        self.free.push(slot);
        Some(v)
    }

    /// Live `(handle, value)` pairs in slot order — canonical.
    pub fn entries(&self) -> Vec<(u64, u64)> {
        let mut out = Vec::with_capacity(self.len);
        for (i, s) in self.slots.iter().enumerate() {
            if let Some(v) = s.val {
                out.push((((i as u64) << 32) | s.gen as u64, v));
            }
        }
        out
    }
}

fn split(h: u64) -> (u32, u32) {
    ((h >> 32) as u32, h as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn matches_shadow_oracle_and_stale_handles_die() {
        let mut rng = SplitMix64::new(0x5107);
        let mut s = Slotmap::new();
        // shadow: handle -> val for every handle ever issued
        let mut live: BTreeMap<u64, u64> = BTreeMap::new();
        let mut stale: BTreeSet<u64> = BTreeSet::new();
        for _ in 0..2000 {
            match rng.below(5) {
                0..=1 => {
                    let v = rng.next_u64() % 1000;
                    let h = s.insert(v).unwrap();
                    live.insert(h, v);
                }
                2 => {
                    // remove a random live or stale handle
                    let pool: Vec<u64> = live.keys().chain(stale.iter()).copied().collect();
                    if pool.is_empty() {
                        continue;
                    }
                    let h = pool[rng.below(pool.len() as u32) as usize];
                    let want = live.remove(&h);
                    assert_eq!(s.remove(h), want);
                    if want.is_none() {
                        assert!(stale.insert(h) || !s.contains(h));
                    } else {
                        stale.insert(h);
                    }
                }
                3 => {
                    let pool: Vec<u64> = live.keys().chain(stale.iter()).copied().collect();
                    if pool.is_empty() {
                        continue;
                    }
                    let h = pool[rng.below(pool.len() as u32) as usize];
                    assert_eq!(s.get(h), live.get(&h).copied());
                    assert_eq!(s.contains(h), live.contains_key(&h));
                }
                _ => {
                    let pool: Vec<u64> = live.keys().copied().collect();
                    if pool.is_empty() {
                        continue;
                    }
                    let h = pool[rng.below(pool.len() as u32) as usize];
                    let v = rng.next_u64() % 1000;
                    assert!(s.set(h, v));
                    live.insert(h, v);
                }
            }
            assert_eq!(s.len(), live.len());
        }
        // canonical slot order dump
        let expect: Vec<(u64, u64)> = live.iter().map(|(&h, &v)| (h, v)).collect();
        let mut got = s.entries();
        got.sort_unstable();
        assert_eq!(got, {
            let mut e = expect;
            e.sort_unstable();
            e
        });
        assert_eq!(s.entries().len(), live.len());
        assert!(!s.is_empty() || live.is_empty());
    }

    #[test]
    fn recycle_gets_new_generation() {
        let mut s = Slotmap::new();
        let h1 = s.insert(1).unwrap();
        s.remove(h1);
        let h2 = s.insert(2).unwrap();
        assert_eq!(h1 >> 32, h2 >> 32); // same slot
        assert_ne!(h1, h2); // newer gen
        assert_eq!(s.get(h1), None);
        assert_eq!(s.get(h2), Some(2));
        assert!(!s.set(h1, 9));
        // draining empties
        assert_eq!(s.remove(h2), Some(2));
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        // unknown handle rejected everywhere
        assert_eq!(s.get(0xFFFF_FFFF_0000_0000), None);
        assert_eq!(s.remove(42), None);
        assert!(!s.set(42, 1));
        assert!(!s.contains(42));
    }
}

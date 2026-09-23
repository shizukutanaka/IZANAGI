//! TLSF — Two-Level Segregate Fit, the `O(1)` general-purpose
//! allocator behind real-time kernels (RTEMS, Zephyr's tlsf
//! heap) and game engines that cannot tolerate `malloc`'s
//! unpredictable latency. Where `buddy` rounds every request to
//! a power of two — up to 2× internal waste — TLSF bins sizes
//! logarithmically at the first level and linearly at the
//! second, keeping waste under ~4%.
//!
//! Determinism: every free list is address-sorted and
//! allocation takes the *lowest* address in the smallest
//! fitting bin, so the pool layout is a pure function of the
//! operation sequence — identical on every peer. `free`
//! coalesces eagerly with both physical neighbors, so no two
//! adjacent blocks are ever simultaneously free.
//!
//! Blocks live at offsets `0..total`; the minimum block is 16
//! bytes. Failed operations return `None`/`false` rather than
//! panicking.
//!
//! ```
//! use izanagi_kit::tlsf::Tlsf;
//! let mut al = Tlsf::new(1024);
//! let a = al.alloc(100).unwrap();
//! let b = al.alloc(200).unwrap();
//! al.free(a);
//! assert_eq!(al.free_bytes(), 1024 - al.block_size(b).unwrap_or(0));
//! al.free(b);
//! assert_eq!(al.free_bytes(), 1024);
//! ```
//!
//! References: Masmano, Ripoll & Crespo (2004) "A constant-time
//! dynamic storage allocator for real-time systems"; Matt
//! Conte's public-domain `tlsf` implementation for the bin
//! mapping.

use std::collections::{BTreeMap, BTreeSet};

/// Minimum allocatable block — also the smallest remainder that
/// may be carved off a split.
pub const MIN: u32 = 16;

/// Second-level index bits: `1 << SLI` linear bins per
/// power-of-two level.
const SLI: u32 = 4;
const SL: u32 = 1 << SLI;
/// Sizes below this live in flat first-level bins.
const SMALL: u32 = 1 << (SLI + 1); // 32

/// `(fl, sl)` bin for storing a *free* block of `size` bytes.
fn bin_of(size: u32) -> (u8, u8) {
    if size < SMALL {
        (0, size.min(SMALL - 1) as u8)
    } else {
        let fl = 31 - size.leading_zeros();
        // sl indexes the linear subdivision of (2^fl, 2^{fl+1})
        let sl = ((size >> (fl - SLI)) - SL) as u8;
        (fl as u8, sl)
    }
}

/// TLSF allocator over offsets `0..total`.
#[derive(Clone, Debug)]
pub struct Tlsf {
    total: u32,
    /// All blocks in address order: offset → (size, free?).
    blocks: BTreeMap<u32, (u32, bool)>,
    /// Free blocks per `(fl, sl)` bin, address-sorted.
    bins: BTreeMap<(u8, u8), BTreeSet<u32>>,
}

impl Tlsf {
    /// Allocator over `total` bytes (`0` → empty pool).
    pub fn new(total: u32) -> Self {
        let mut s = Self {
            total,
            blocks: BTreeMap::new(),
            bins: BTreeMap::new(),
        };
        if total >= MIN {
            s.blocks.insert(0, (total, true));
            s.bins.entry(bin_of(total)).or_default().insert(0);
        }
        s
    }

    /// Pool size.
    pub fn total(&self) -> u32 {
        self.total
    }

    fn insert_free(&mut self, addr: u32, size: u32) {
        self.blocks.insert(addr, (size, true));
        self.bins.entry(bin_of(size)).or_default().insert(addr);
    }

    fn take_free(&mut self, addr: u32) -> Option<u32> {
        let (size, free) = *self.blocks.get(&addr)?;
        if free {
            if let Some(set) = self.bins.get_mut(&bin_of(size)) {
                set.remove(&addr);
            }
            self.blocks.insert(addr, (size, false));
            Some(size)
        } else {
            None
        }
    }

    /// Allocate `size` bytes; returns the block offset.
    pub fn alloc(&mut self, size: u32) -> Option<u32> {
        let want = size.max(MIN);
        if want > self.total {
            return None;
        }
        let (fl, sl) = bin_of(want);
        // Scan bins from `want`'s own bin upward — a bin covers a
        // *range* of sizes, so each candidate must still be
        // checked to fit; the lowest fitting address wins. (The
        // classic TLSF round-up of `search_bin` would skip exact
        // fits sitting in `want`'s bin.)
        let addr = self
            .bins
            .range((fl, sl)..)
            .find_map(|(_, set)| set.iter().find(|&&a| self.blocks[&a].0 >= want).copied())?;
        let bsize = self.take_free(addr)?;
        // split when the remainder can stand alone
        let rem = bsize - want;
        let used = if rem >= MIN { want } else { bsize };
        self.blocks.insert(addr, (used, false));
        if rem >= MIN {
            self.insert_free(addr + used, rem);
        }
        Some(addr)
    }

    /// Free the block at `addr`; false if it isn't a live
    /// allocation.
    pub fn free(&mut self, addr: u32) -> bool {
        let (mut size, free) = match self.blocks.get(&addr) {
            Some(&(s, f)) if !f => (s, f),
            _ => return false,
        };
        let _ = free;
        // merge with the next block when free
        let next = addr + size;
        if let Some(&(ns, true)) = self.blocks.get(&next) {
            self.take_free(next);
            self.blocks.remove(&next);
            size += ns;
        }
        // merge with the previous block when free
        if let Some((&pa, &(ps, true))) = self.blocks.range(..addr).next_back() {
            self.take_free(pa);
            self.blocks.remove(&addr);
            size += ps;
            self.insert_free(pa, size);
        } else {
            self.insert_free(addr, size);
        }
        true
    }

    /// Size of the block containing `addr` (live or free).
    pub fn block_size(&self, addr: u32) -> Option<u32> {
        self.blocks.get(&addr).map(|&(s, _)| s)
    }

    /// Is `addr` the start of a live allocation?
    pub fn is_live(&self, addr: u32) -> bool {
        matches!(self.blocks.get(&addr), Some(&(_, false)))
    }

    /// Total bytes in free blocks.
    pub fn free_bytes(&self) -> u32 {
        self.bins
            .values()
            .flat_map(|s| s.iter())
            .filter_map(|a| self.blocks.get(a))
            .map(|&(s, _)| s)
            .sum()
    }

    /// Largest single free block.
    pub fn largest_free(&self) -> u32 {
        self.bins
            .values()
            .flat_map(|s| s.iter())
            .filter_map(|a| self.blocks.get(a))
            .map(|&(s, _)| s)
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    /// Shadow oracle: live allocations as (addr → size).
    struct Shadow {
        live: BTreeMap<u32, u32>,
    }
    impl Shadow {
        fn fits(&self, addr: u32, size: u32) -> bool {
            self.live
                .range(..=addr)
                .next_back()
                .map(|(&a, &s)| addr >= a + s)
                .unwrap_or(true)
                && self
                    .live
                    .range(addr + 1..)
                    .next()
                    .map(|(&a, _)| addr + size <= a)
                    .unwrap_or(true)
        }
    }

    /// Canonical invariant: blocks tile `0..total` exactly and
    /// no two adjacent blocks are both free.
    fn check_layout(al: &Tlsf) {
        let mut at = 0u32;
        let mut prev_free = false;
        for (&a, &(s, f)) in &al.blocks {
            assert_eq!(a, at, "block tiling gap at {a}");
            assert!(s >= MIN);
            assert!(!(prev_free && f), "adjacent free blocks at {a}");
            prev_free = f;
            at += s;
        }
        assert_eq!(at, al.total);
    }

    #[test]
    fn alloc_free_recovers_pool() {
        let mut al = Tlsf::new(1024);
        let a = al.alloc(100).unwrap_or(0);
        let b = al.alloc(200).unwrap_or(0);
        assert!(al.is_live(a));
        assert!(al.is_live(b));
        assert!(al.free(a));
        assert!(al.free(b));
        assert_eq!(al.free_bytes(), 1024);
        assert_eq!(al.largest_free(), 1024);
        check_layout(&al);
    }

    #[test]
    fn free_twice_refused() {
        let mut al = Tlsf::new(256);
        let a = al.alloc(64).unwrap_or(0);
        assert!(al.free(a));
        assert!(!al.free(a));
        assert!(!al.free(13));
    }

    #[test]
    fn exhaustion_returns_none() {
        let mut al = Tlsf::new(128);
        assert!(al.alloc(64).is_some());
        assert!(al.alloc(64).is_some());
        assert!(al.alloc(64).is_none());
        assert!(al.alloc(65).is_none());
        check_layout(&al);
    }

    #[test]
    fn oracle_random_ops() {
        let mut r = SplitMix64::new(0x715F);
        for _round in 0..40 {
            let total = 1 << (10 + r.next_u64() % 3);
            let mut al = Tlsf::new(total);
            let mut sh = Shadow {
                live: BTreeMap::new(),
            };
            for _ in 0..1500 {
                if r.next_u64() % 3 == 0 && !sh.live.is_empty() {
                    // free a random live block
                    let k = (r.next_u64() as usize) % sh.live.len();
                    let &a = sh.live.keys().nth(k).unwrap_or(&0);
                    assert!(al.free(a));
                    sh.live.remove(&a);
                } else {
                    let size = 1 + (r.next_u64() % 300) as u32;
                    match al.alloc(size) {
                        Some(a) => {
                            let bs = al.block_size(a).unwrap_or(0);
                            assert!(bs >= size, "block {bs} < request {size}");
                            assert!(sh.fits(a, bs), "overlap at {a}..{}", a + bs);
                            sh.live.insert(a, bs);
                        }
                        None => {
                            // refusal must be honest: no free block
                            // can hold a properly-rounded request
                            assert!(al.largest_free() < size.max(MIN));
                        }
                    }
                }
                check_layout(&al);
            }
            // free everything → single coalesced span
            for &a in sh.live.keys().collect::<Vec<_>>() {
                assert!(al.free(a));
            }
            assert_eq!(al.free_bytes(), total);
            check_layout(&al);
        }
    }

    #[test]
    fn deterministic_layout() {
        let ops = [
            (100u32, true),
            (40, true),
            (0, false),
            (60, true),
            (1, false),
        ];
        let run = |ops: &[(u32, bool)]| {
            let mut al = Tlsf::new(512);
            let mut addrs = Vec::new();
            for &(s, is_alloc) in ops {
                if is_alloc {
                    let a = al.alloc(s).unwrap_or(!0);
                    addrs.push(a);
                } else if let Some(&a) = addrs.get(s as usize) {
                    al.free(a);
                }
            }
            al.blocks
                .iter()
                .map(|(&a, &(s, f))| (a, s, f))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(&ops), run(&ops));
    }
}

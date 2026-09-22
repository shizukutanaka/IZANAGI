//! Binary buddy allocator — the classic log₂-segregated allocator
//! behind fixed-size arena allocators, TLSF's ancestor, and the
//! Linux kernel's page-level buddy system. Split on demand, merge
//! on free, all pointers power-of-two-aligned.
//!
//! Determinism: every free list is kept sorted by address and
//! allocation takes the *lowest* free block of the *smallest*
//! fitting order, so the allocator state is a pure function of the
//! operation sequence — identical on every peer. `free` coalesces
//! eagerly: a freed block whose buddy is also free merges upward,
//! so the free lists always satisfy the canonical invariant that
//! **no two buddies are simultaneously free**.
//!
//! Blocks live at offsets `0..total` where `total` and the minimum
//! block size `min` are powers of two. Failed operations return
//! `None`/`false` rather than panicking.
//!
//! ```
//! use izanagi_kit::buddy::Buddy;
//! let mut al = Buddy::new(256, 16).unwrap();
//! let a = al.alloc(40).unwrap(); // rounds to a 64-block
//! assert_eq!(a % 64, 0);
//! let b = al.alloc(16).unwrap();
//! al.free(a);
//! al.free(b);
//! assert_eq!(al.free_bytes(), 256);
//! ```

use std::collections::BTreeMap;

/// Binary buddy allocator over `0..total` bytes.
#[derive(Clone, Debug)]
pub struct Buddy {
    total: u32,
    min: u32,
    /// `free_lists[k]` = sorted free block offsets of size `min << k`.
    free: Vec<Vec<u32>>,
    /// Live allocations: address → order index.
    live: BTreeMap<u32, usize>,
}

impl Buddy {
    /// New arena of `total` bytes with minimum block `min` — both
    /// must be powers of two with `min <= total`, else `None`.
    pub fn new(total: u32, min: u32) -> Option<Self> {
        if total == 0 || !total.is_power_of_two() || !min.is_power_of_two() || min > total {
            return None;
        }
        let levels = (total / min).trailing_zeros() as usize + 1;
        let mut free = vec![Vec::new(); levels];
        free[levels - 1].push(0); // one whole arena block
        Some(Buddy {
            total,
            min,
            free,
            live: BTreeMap::new(),
        })
    }

    /// Arena capacity in bytes.
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Allocate `size` bytes — returns the block offset, `None` when
    /// no block can satisfy it (fragmentation or exhaustion). The
    /// block is a power of two at least `min`, naturally aligned.
    pub fn alloc(&mut self, size: u32) -> Option<u32> {
        if size == 0 || size > self.total {
            return None;
        }
        let want = size.max(self.min).next_power_of_two();
        let k = (want / self.min).trailing_zeros() as usize;
        if k >= self.free.len() {
            return None;
        }
        // Smallest free block of order ≥ k.
        let mut src = k;
        while src < self.free.len() && self.free[src].is_empty() {
            src += 1;
        }
        if src == self.free.len() {
            return None;
        }
        // Lowest-address block (lists are sorted).
        let addr = self.free[src].remove(0);
        // Split down to order k, freeing the right halves.
        while src > k {
            src -= 1;
            let buddy = addr + (self.min << src);
            self.free[src].push(buddy);
            self.free[src].sort_unstable();
        }
        self.live.insert(addr, k);
        Some(addr)
    }

    /// Release a block allocated by [`Buddy::alloc`] — `false` when
    /// `addr` is not currently allocated (double-free included).
    /// Coalesces with free buddies eagerly.
    pub fn free(&mut self, addr: u32) -> bool {
        let mut k = match self.live.remove(&addr) {
            Some(k) => k,
            None => return false,
        };
        let mut addr = addr;
        loop {
            let size = self.min << k;
            let buddy = addr ^ size;
            match self.free[k].iter().position(|&b| b == buddy) {
                Some(pos) if buddy + size <= self.total => {
                    self.free[k].remove(pos);
                    addr = addr.min(buddy);
                    k += 1;
                }
                _ => break,
            }
        }
        self.free[k].push(addr);
        self.free[k].sort_unstable();
        true
    }

    /// The block size backing a live allocation — `None` when
    /// `addr` isn't allocated.
    pub fn block_size(&self, addr: u32) -> Option<u32> {
        self.live.get(&addr).map(|&k| self.min << k)
    }

    /// Total free bytes across all orders.
    pub fn free_bytes(&self) -> u32 {
        let mut sum = 0u32;
        for (k, list) in self.free.iter().enumerate() {
            sum += list.len() as u32 * (self.min << k);
        }
        sum
    }

    /// Largest allocatable request in bytes — the size of the
    /// biggest free block (fragmentation-aware, not just free_bytes).
    pub fn largest_free(&self) -> u32 {
        for k in (0..self.free.len()).rev() {
            if !self.free[k].is_empty() {
                return self.min << k;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Shadow oracle: byte-array occupancy + alignment/geometry
    /// checks on every allocation.
    fn check_state(al: &Buddy, used: &[u8], live: &BTreeMap<u32, u32>) {
        // free_bytes consistency
        let occupied: u32 = used.iter().map(|&c| c as u32).sum();
        assert_eq!(al.free_bytes(), al.total() - occupied);
        // every live block is actually allocated & aligned
        for (&addr, &size) in live {
            assert_eq!(addr % size, 0, "unaligned {addr} size {size}");
            assert!(size.is_power_of_two());
            for i in addr..addr + size {
                assert_eq!(used[i as usize], 1, "live block not marked");
            }
        }
    }

    #[test]
    fn alloc_free_matches_shadow_oracle() {
        let mut rng = SplitMix64::new(0xB0DD);
        for _ in 0..300 {
            let pow = 4 + rng.below(8); // total 16..2048
            let total = 1u32 << pow;
            let min = 1u32 << rng.below(pow.min(4) + 1);
            let mut al = Buddy::new(total, min).unwrap();
            let mut used = vec![0u8; total as usize];
            let mut live = BTreeMap::<u32, u32>::new();
            for _ in 0..200 {
                if rng.below(2) == 0 || live.is_empty() {
                    // alloc
                    let req = 1 + rng.below(total.min(128));
                    match al.alloc(req) {
                        Some(addr) => {
                            let sz = al.block_size(addr).unwrap();
                            assert!(sz >= req);
                            assert_eq!(addr % sz, 0);
                            assert_eq!(
                                used[addr as usize..(addr + sz) as usize].iter().sum::<u8>(),
                                0
                            );
                            for i in addr..addr + sz {
                                used[i as usize] = 1;
                            }
                            live.insert(addr, sz);
                        }
                        None => {
                            // Refusal must be honest: no request ≤
                            // largest_free can be refused.
                            assert!(req > al.largest_free());
                        }
                    }
                } else {
                    // free a random live block
                    let idx = rng.below(live.len() as u32) as usize;
                    let addr = *live.keys().nth(idx).unwrap();
                    let sz = live[&addr];
                    assert!(al.free(addr));
                    for i in addr..addr + sz {
                        used[i as usize] = 0;
                    }
                    live.remove(&addr);
                }
                check_state(&al, &used, &live);
            }
            // Canonical invariant: every remaining free byte must
            // be allocatable — drain to min blocks and confirm the
            // arena is fully covered (catches missed coalescing).
            while let Some(addr) = al.alloc(1) {
                let sz = al.block_size(addr).unwrap();
                for i in addr..addr + sz {
                    used[i as usize] = 1;
                }
                live.insert(addr, sz);
            }
            assert_eq!(used.iter().map(|&c| c as u32).sum::<u32>(), total);
        }
    }

    #[test]
    fn splitting_coalescing_and_alignment() {
        let mut al = Buddy::new(64, 16).unwrap();
        let a = al.alloc(16).unwrap();
        let b = al.alloc(16).unwrap();
        let c = al.alloc(32).unwrap();
        assert_eq!(a, 0);
        assert_eq!(b, 16);
        assert_eq!(c, 32);
        assert_eq!(al.free_bytes(), 0);
        assert!(al.alloc(1).is_none());
        // Free a, b — they coalesce to [0,32); c still holds [32,64).
        al.free(b);
        al.free(a);
        assert_eq!(al.free_bytes(), 32);
        assert_eq!(al.largest_free(), 32);
        let d = al.alloc(20).unwrap(); // needs 32 — gets [0,32)
        assert_eq!(d, 0);
        assert_eq!(al.free_bytes(), 0);
        // free c then d — everything re-merges.
        al.free(c);
        al.free(d);
        assert_eq!(al.free_bytes(), 64);
        assert_eq!(al.largest_free(), 64);
    }

    #[test]
    fn validation_and_double_free() {
        assert!(Buddy::new(0, 4).is_none());
        assert!(Buddy::new(48, 16).is_none()); // total not pow2
        assert!(Buddy::new(16, 32).is_none()); // min > total
        assert!(Buddy::new(64, 3).is_none()); // min not pow2
        let mut al = Buddy::new(64, 16).unwrap();
        assert!(al.alloc(0).is_none());
        assert!(al.alloc(128).is_none());
        let a = al.alloc(8).unwrap();
        assert!(al.free(a));
        assert!(!al.free(a)); // double free rejected
        assert!(!al.free(999)); // never allocated
        assert_eq!(al.block_size(999), None);
        assert_eq!(al.total(), 64);
    }
}

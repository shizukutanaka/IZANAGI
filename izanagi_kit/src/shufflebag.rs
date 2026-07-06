//! Bag randomizer — draw without replacement, auto-refilling each cycle.
//!
//! [`random_table`](crate::random_table) draws *with* replacement (every roll
//! is independent, so the same value can repeat many times in a row and other
//! values can drought for a long stretch). [`SplitMix64::sample_n`](crate::SplitMix64::sample_n)
//! draws without replacement but is one-shot — it does not cycle.
//!
//! A [`ShuffleBag`] is the persistent "bag randomizer" used for Tetris-style
//! piece sequences, drought-free loot, and music shuffles: it holds a multiset
//! of items and [`draw`](ShuffleBag::draw)s a uniformly-random one *without
//! replacement*; when the bag empties it automatically refills from the
//! original contents. Over each full cycle every item appears exactly as many
//! times as it was added — random order, guaranteed even distribution.
//!
//! ```
//! use izanagi_kit::shufflebag::ShuffleBag;
//! use izanagi_kit::SplitMix64;
//!
//! let mut rng = SplitMix64::new(42);
//! let mut bag = ShuffleBag::new(vec!['a', 'b', 'c']);
//!
//! // One full cycle yields each item exactly once, in some random order.
//! let mut cycle = [bag.draw(&mut rng).unwrap(),
//!                  bag.draw(&mut rng).unwrap(),
//!                  bag.draw(&mut rng).unwrap()];
//! cycle.sort_unstable();
//! assert_eq!(cycle, ['a', 'b', 'c']);
//!
//! // The bag is now empty; the next draw refills it automatically.
//! assert_eq!(bag.remaining(), 0);
//! assert!(bag.draw(&mut rng).is_some());
//! assert_eq!(bag.remaining(), 2); // refilled to 3, then one drawn
//! ```
//!
//! Determinism: draws consume the supplied [`SplitMix64`] in a fixed order
//! (one `below` draw per non-trivial pick; a size-1 bag draws nothing), so the
//! sequence is replay-identical. [`ShuffleBag`] implements
//! [`DetHash`](crate::world_hash::DetHash) over both the template and the live
//! bag, folding them into the replay checksum.

use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};

/// A draw-without-replacement bag that auto-refills from its original contents.
#[derive(Clone, Debug, Default)]
pub struct ShuffleBag<T> {
    /// The immutable template: one full cycle's worth of items.
    contents: Vec<T>,
    /// Items not yet drawn this cycle. Refilled from `contents` when emptied.
    current: Vec<T>,
}

impl<T: Clone> ShuffleBag<T> {
    /// Create a bag from `contents` (the per-cycle multiset). The bag starts
    /// full, ready to draw.
    pub fn new(contents: Vec<T>) -> Self {
        let current = contents.clone();
        ShuffleBag { contents, current }
    }

    /// The number of items in one full cycle (the template size).
    #[inline]
    pub fn cycle_len(&self) -> usize {
        self.contents.len()
    }

    /// `true` if the bag has no contents at all — [`draw`](Self::draw) always
    /// returns `None`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    /// Items still undrawn in the current cycle.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.current.len()
    }

    /// `true` if the current cycle is exhausted; the next [`draw`](Self::draw)
    /// will refill before picking.
    #[inline]
    pub fn cycle_exhausted(&self) -> bool {
        self.current.is_empty()
    }

    /// The undrawn items of the current cycle, in internal order (not the draw
    /// order). Useful for save/inspection.
    #[inline]
    pub fn peek_remaining(&self) -> &[T] {
        &self.current
    }

    /// Refill the current cycle from the template immediately, discarding any
    /// undrawn items. After this, `remaining() == cycle_len()`.
    pub fn refill(&mut self) {
        self.current.clear();
        self.current.extend_from_slice(&self.contents);
    }

    /// Add `item` to the template (all future cycles) **and** to the current
    /// bag (so it can be drawn this cycle too). Grows the cycle length by one.
    pub fn add(&mut self, item: T) {
        self.contents.push(item.clone());
        self.current.push(item);
    }

    /// Draw a uniformly-random item without replacement. When the current cycle
    /// is empty it refills from the template first. Returns `None` only when the
    /// bag has no contents at all.
    ///
    /// Consumes one [`SplitMix64::below`] draw per non-trivial pick (a size-1
    /// bag consumes none, matching the RNG's degenerate-bound contract).
    pub fn draw(&mut self, rng: &mut SplitMix64) -> Option<T> {
        if self.contents.is_empty() {
            return None;
        }
        if self.current.is_empty() {
            self.refill();
        }
        let n = self.current.len();
        let idx = if n > 1 {
            rng.below(n as u32) as usize
        } else {
            0
        };
        Some(self.current.swap_remove(idx))
    }
}

impl<T: Clone + DetHash> DetHash for ShuffleBag<T> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.contents.len() as u32);
        for t in &self.contents {
            t.det_hash(hasher);
        }
        hasher.write_u32(self.current.len() as u32);
        for t in &self.current {
            t.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_starts_full() {
        let bag = ShuffleBag::new(vec![1u32, 2, 3]);
        assert_eq!(bag.cycle_len(), 3);
        assert_eq!(bag.remaining(), 3);
        assert!(!bag.is_empty());
        assert!(!bag.cycle_exhausted());
    }

    #[test]
    fn test_empty_bag_draws_none() {
        let mut bag: ShuffleBag<u32> = ShuffleBag::new(Vec::new());
        let mut rng = SplitMix64::new(1);
        assert!(bag.is_empty());
        assert_eq!(bag.draw(&mut rng), None);
        assert_eq!(bag.draw(&mut rng), None);
    }

    #[test]
    fn test_full_cycle_is_a_permutation() {
        let mut bag = ShuffleBag::new(vec![1u32, 2, 3, 4, 5]);
        let mut rng = SplitMix64::new(7);
        let mut drawn: Vec<u32> = (0..5).map(|_| bag.draw(&mut rng).unwrap()).collect();
        drawn.sort_unstable();
        assert_eq!(
            drawn,
            vec![1, 2, 3, 4, 5],
            "one cycle must be a permutation"
        );
        assert_eq!(bag.remaining(), 0);
        assert!(bag.cycle_exhausted());
    }

    #[test]
    fn test_cycle_handles_duplicates_by_multiplicity() {
        let mut bag = ShuffleBag::new(vec!['a', 'a', 'b']);
        let mut rng = SplitMix64::new(99);
        let mut drawn: Vec<char> = (0..3).map(|_| bag.draw(&mut rng).unwrap()).collect();
        drawn.sort_unstable();
        assert_eq!(drawn, vec!['a', 'a', 'b'], "duplicates preserved per cycle");
    }

    #[test]
    fn test_auto_refill_after_exhaustion() {
        let mut bag = ShuffleBag::new(vec![1u32, 2, 3]);
        let mut rng = SplitMix64::new(3);
        for _ in 0..3 {
            bag.draw(&mut rng);
        }
        assert_eq!(bag.remaining(), 0);
        bag.draw(&mut rng); // triggers refill
        assert_eq!(bag.remaining(), 2, "refilled to 3 then drew one");
    }

    #[test]
    fn test_two_cycles_balanced() {
        let mut bag = ShuffleBag::new(vec![1u32, 2, 3]);
        let mut rng = SplitMix64::new(123);
        let mut counts = std::collections::BTreeMap::new();
        for _ in 0..6 {
            *counts.entry(bag.draw(&mut rng).unwrap()).or_insert(0u32) += 1;
        }
        assert_eq!(counts.get(&1), Some(&2));
        assert_eq!(counts.get(&2), Some(&2));
        assert_eq!(counts.get(&3), Some(&2));
    }

    #[test]
    fn test_single_element_bag_draws_it_every_time_without_drawing() {
        let mut bag = ShuffleBag::new(vec![42u32]);
        let mut rng = SplitMix64::new(5);
        let state = rng.state();
        for _ in 0..4 {
            assert_eq!(bag.draw(&mut rng), Some(42));
        }
        assert_eq!(rng.state(), state, "size-1 bag must consume no RNG draws");
    }

    #[test]
    fn test_draw_is_deterministic() {
        let seq = |seed: u64| {
            let mut bag = ShuffleBag::new(vec![1u32, 2, 3, 4]);
            let mut rng = SplitMix64::new(seed);
            (0..12)
                .map(|_| bag.draw(&mut rng).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(seq(77), seq(77));
        assert_ne!(seq(1), seq(2));
    }

    #[test]
    fn test_refill_resets_remaining() {
        let mut bag = ShuffleBag::new(vec![1u32, 2, 3]);
        let mut rng = SplitMix64::new(8);
        bag.draw(&mut rng);
        assert_eq!(bag.remaining(), 2);
        bag.refill();
        assert_eq!(bag.remaining(), 3);
    }

    #[test]
    fn test_add_extends_template_and_bag() {
        let mut bag = ShuffleBag::new(vec![1u32]);
        bag.add(2);
        assert_eq!(bag.cycle_len(), 2);
        assert_eq!(bag.remaining(), 2);
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let a = ShuffleBag::new(vec![1u32, 2, 3]);
        let b = ShuffleBag::new(vec![1u32, 2, 3]);
        assert_eq!(hash_state(&a), hash_state(&b));
        let mut c = a.clone();
        let mut rng = SplitMix64::new(1);
        c.draw(&mut rng); // mutates `current`
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "drawing must change the hash"
        );
    }
}

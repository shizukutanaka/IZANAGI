//! Count-min sketch — "about how many times did this key arrive?" in
//! `O(depth·width)` counters that never grow with the distinct-key
//! count. Event-rate audits (packets per peer), loot-drop histograms,
//! hot-cell detection over millions of tiles without a giant map.
//! The sketch is a pure function of `(seed, depth, width, stream)`.
//! Estimates never undercount: collisions only ever add, so
//! `estimate(key) >= true_count(key)` always — one-sided error.
//!
//! ```
//! use izanagi_kit::cms::CountMin;
//! let mut s = CountMin::new(4, 64, 0x5EED);
//! for _ in 0..10 {
//!     s.add(b"hit");
//! }
//! s.add(b"miss");
//! assert!(s.estimate(b"hit") >= 10);
//! assert!(s.estimate(b"miss") >= 1);
//! ```

use crate::world_hash::Fnv1a;

/// Count-min sketch: `depth` rows of `width` counters; each row uses a
/// different seed, so an overestimate requires a collision in *every*
/// row at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountMin {
    depth: usize,
    width: usize,
    seed: u64,
    /// Row-major `depth * width` counters.
    cells: Vec<u64>,
    total: u64,
}

impl CountMin {
    /// `depth` hash rows of `width` buckets (`0` in either → empty).
    pub fn new(depth: usize, width: usize, seed: u64) -> Self {
        CountMin {
            depth,
            width,
            seed,
            cells: vec![0; depth.saturating_mul(width)],
            total: 0,
        }
    }

    /// Rows.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Buckets per row.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Total items added.
    pub fn total(&self) -> u64 {
        self.total
    }

    fn slot(&self, row: usize, key: &[u8]) -> usize {
        let mut h = Fnv1a::new();
        h.write_u64(self.seed);
        h.write_u64(row as u64);
        h.write_bytes(key);
        row * self.width + (h.finish() % self.width as u64) as usize
    }

    /// Increment the count of `key` by 1.
    pub fn add(&mut self, key: &[u8]) {
        self.add_count(key, 1);
    }

    /// Increment the count of `key` by `n` (bulk arrival).
    pub fn add_count(&mut self, key: &[u8], n: u64) {
        if self.width == 0 {
            return;
        }
        for row in 0..self.depth {
            let i = self.slot(row, key);
            self.cells[i] = self.cells[i].saturating_add(n);
        }
        self.total = self.total.saturating_add(n);
    }

    /// Lower bound on `key`'s true count — never undercounts.
    pub fn estimate(&self, key: &[u8]) -> u64 {
        if self.width == 0 || self.depth == 0 {
            return 0;
        }
        (0..self.depth)
            .map(|r| self.cells[self.slot(r, key)])
            .min()
            .unwrap_or(0)
    }

    /// Merge two sketches — element-wise addition. Dimensions and seed
    /// must match; the result is `None` otherwise (rather than a
    /// silently meaningless counter map).
    pub fn merge(&self, other: &CountMin) -> Option<CountMin> {
        if self.depth != other.depth || self.width != other.width || self.seed != other.seed {
            return None;
        }
        Some(CountMin {
            depth: self.depth,
            width: self.width,
            seed: self.seed,
            cells: self
                .cells
                .iter()
                .zip(other.cells.iter())
                .map(|(&a, &b)| a.saturating_add(b))
                .collect(),
            total: self.total.saturating_add(other.total),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn never_undercounts_and_merge_equals_stream() {
        let mut rng = SplitMix64::new(0xFEED_5EED);
        let mut exact: BTreeMap<u64, u64> = BTreeMap::new();
        let mut s = CountMin::new(4, 256, 9);
        for _ in 0..20_000 {
            let key = rng.below(500) as u64;
            *exact.entry(key).or_default() += 1;
            s.add(&key.to_le_bytes());
        }
        // One-sided error: estimate >= truth; empirical excess small.
        let mut excess = 0u64;
        for (&key, &truth) in &exact {
            let e = s.estimate(&key.to_le_bytes());
            assert!(e >= truth, "key {key}: est {e} < truth {truth}");
            excess += e - truth;
        }
        assert!(excess < 20_000, "excess {excess}");
        // Merge == one stream.
        let mut a = CountMin::new(4, 256, 9);
        let mut b = CountMin::new(4, 256, 9);
        let mut flip = false;
        for (&key, &n) in &exact {
            if flip {
                a.add_count(&key.to_le_bytes(), n);
            } else {
                b.add_count(&key.to_le_bytes(), n);
            }
            flip = !flip;
        }
        let m = a.merge(&b);
        assert!(m.is_some());
        let m = m.unwrap_or_else(|| CountMin::new(0, 0, 0));
        for (&key, &truth) in &exact {
            assert!(m.estimate(&key.to_le_bytes()) >= truth);
        }
        assert_eq!(m.total(), s.total());
        // Dimension mismatch refuses.
        assert!(s.merge(&CountMin::new(4, 128, 9)).is_none());
        assert!(s.merge(&CountMin::new(4, 256, 10)).is_none());
    }

    #[test]
    fn empty_width_is_noop() {
        let mut s = CountMin::new(4, 0, 1);
        s.add(b"x");
        assert_eq!(s.estimate(b"x"), 0);
        assert_eq!(CountMin::new(0, 8, 1).estimate(b"x"), 0);
    }
}

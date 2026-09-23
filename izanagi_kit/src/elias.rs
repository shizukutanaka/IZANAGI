//! Elias–Fano monotone integer sequence — compresses a sorted
//! `u64` list into `n·l` verbatim low bits plus an `n + U` bit
//! unary bitmap for the high parts, where `l = ⌊log2(u/n)⌋` and
//! `U = u >> l`. Gives `access`/`rank`/`successor` in a few word
//! operations without any floating point.
//!
//! Construction sorts a copy of the input, so the structure is a
//! pure function of the multiset — insertion order never leaks
//! into the encoding, which keeps encoders on different peers
//! bit-identical.
//!
//! ```
//! use izanagi_kit::elias::EliasFano;
//!
//! let ef = EliasFano::new(&[5, 3, 11, 3, 100]);
//! assert_eq!(ef.len(), 5);
//! assert_eq!(ef.access(0), Some(3));
//! assert_eq!(ef.access(4), Some(100));
//! assert_eq!(ef.rank(11), 3); // elements < 11
//! assert_eq!(ef.successor(4), Some(2)); // a[2] = 5
//! ```
//!
//! References: Elias (1974); Fano (1971); Vigna, "Quasi-succinct
//! indices" (2013); the `sux` and `sux4j` implementations.

/// Succinct monotone sequence over `u64`.
pub struct EliasFano {
    /// Element count.
    n: usize,
    /// Low-bit width `l` in `0..=63`.
    lb: u32,
    /// `u >> l` — number of zeros in the high bitmap.
    u_hi: u64,
    /// `n + u_hi` — logical bit length of `hi`.
    hi_bits: u64,
    /// `low[i] = a[i] & ((1<<l)-1)` stored verbatim.
    low: Vec<u64>,
    /// Unary high bitmap: bit `h_i + i` set for each element,
    /// where `h_i = a[i] >> l`.
    hi: Vec<u64>,
}

impl EliasFano {
    /// Builds the index over `sorted(vals)` — a copy is sorted
    /// internally so callers need not pre-sort.
    pub fn new(vals: &[u64]) -> EliasFano {
        let mut a = vals.to_vec();
        a.sort_unstable();
        let n = a.len();
        if n == 0 {
            return EliasFano {
                n: 0,
                lb: 0,
                u_hi: 0,
                hi_bits: 0,
                low: Vec::new(),
                hi: Vec::new(),
            };
        }
        let u = a[n - 1];
        // l = floor(log2(u / n)) clamped below 64.
        let q = u / n as u64;
        let lb = if q == 0 { 0 } else { 63 - q.leading_zeros() };
        let u_hi = u >> lb;
        let hi_bits = n as u64 + u_hi;
        let words = hi_bits.div_ceil(64) as usize;
        let mut hi = vec![0u64; words];
        let low_mask = if lb == 0 { 0 } else { (1u64 << lb) - 1 };
        let mut low = Vec::with_capacity(n);
        for (i, &v) in a.iter().enumerate() {
            low.push(v & low_mask);
            let pos = (v >> lb) + i as u64;
            hi[(pos / 64) as usize] |= 1u64 << (pos % 64);
        }
        EliasFano {
            n,
            lb,
            u_hi,
            hi_bits,
            low,
            hi,
        }
    }

    /// Number of stored elements.
    pub fn len(&self) -> usize {
        self.n
    }

    /// Whether the sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Bit position of the `k`-th **set** bit in `hi` (0-based).
    fn select1(&self, mut k: u64) -> u64 {
        for (wi, &w) in self.hi.iter().enumerate() {
            let c = w.count_ones() as u64;
            if k < c {
                let mut w = w;
                for _ in 0..k {
                    w &= w - 1;
                }
                return wi as u64 * 64 + w.trailing_zeros() as u64;
            }
            k -= c;
        }
        // Unreachable for k < n.
        self.hi.len() as u64 * 64
    }

    /// Bit position of the `k`-th **zero** bit in `hi` (0-based).
    /// `k` counts zeros inside the logical `hi_bits` window;
    /// only `k < u_hi` has an answer.
    fn select0(&self, mut k: u64) -> u64 {
        for (wi, &w) in self.hi.iter().enumerate() {
            let start = wi as u64 * 64;
            let word_bits = (self.hi_bits - start).min(64);
            let inv = if word_bits == 64 {
                !w
            } else {
                !w & ((1u64 << word_bits) - 1)
            };
            let c = inv.count_ones() as u64;
            if k < c {
                let mut inv = inv;
                for _ in 0..k {
                    inv &= inv - 1;
                }
                return start + inv.trailing_zeros() as u64;
            }
            k -= c;
        }
        self.hi.len() as u64 * 64
    }

    /// Set bits after the last zero — exactly the count of
    /// elements with `hi == u_hi`, since the bitmap's tail after
    /// the final zero is the maximal-high-value run.
    fn ones_after_last_zero(&self) -> usize {
        let mut cnt = 0usize;
        for wi in (0..self.hi.len()).rev() {
            let start = wi as u64 * 64;
            let word_bits = (self.hi_bits - start).min(64);
            let w = if word_bits == 64 {
                self.hi[wi]
            } else {
                self.hi[wi] & ((1u64 << word_bits) - 1)
            };
            let inv = if word_bits == 64 {
                !w
            } else {
                !w & ((1u64 << word_bits) - 1)
            };
            if inv != 0 {
                let top_zero = 63 - inv.leading_zeros();
                let above = if top_zero == 63 {
                    0
                } else {
                    (w >> (top_zero + 1)).count_ones()
                };
                return cnt + above as usize;
            }
            cnt += w.count_ones() as usize;
        }
        cnt
    }

    /// `a[i]`, or `None` for `i >= n`.
    pub fn access(&self, i: usize) -> Option<u64> {
        if i >= self.n {
            return None;
        }
        let hi_i = self.select1(i as u64) - i as u64;
        Some((hi_i << self.lb) | self.low[i])
    }

    /// Number of elements strictly less than `v`.
    pub fn rank(&self, v: u64) -> usize {
        if self.n == 0 {
            return 0;
        }
        let h = v >> self.lb;
        let l = v & ((1u64 << self.lb) - 1);
        // Ones before the h-th zero = elements with hi <= h, so
        // elements with hi < h sit before the (h-1)-th zero.
        // h == u_hi has no further zero: use the tail one-run.
        let mut i = if h == 0 {
            0
        } else if h > self.u_hi {
            self.n
        } else if h == self.u_hi {
            self.n - self.ones_after_last_zero()
        } else {
            (self.select0(h - 1) - (h - 1)) as usize
        };
        while i < self.n {
            let hi_i = self.select1(i as u64) - i as u64;
            if hi_i != h || self.low[i] >= l {
                break;
            }
            i += 1;
        }
        i
    }

    /// Index of the first element `>= v`, or `None` when `v`
    /// exceeds every element.
    pub fn successor(&self, v: u64) -> Option<usize> {
        let i = self.rank(v);
        if i >= self.n {
            None
        } else {
            Some(i)
        }
    }

    /// Index of the first element `> v`.
    pub fn successor_strict(&self, v: u64) -> Option<usize> {
        let mut i = self.rank(v);
        while i < self.n && self.access(i) == Some(v) {
            i += 1;
        }
        if i < self.n {
            Some(i)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn empty_and_singleton() {
        let e = EliasFano::new(&[]);
        assert!(e.is_empty());
        assert_eq!(e.access(0), None);
        assert_eq!(e.rank(0), 0);
        assert_eq!(e.successor(0), None);

        let e = EliasFano::new(&[7]);
        assert_eq!(e.access(0), Some(7));
        assert_eq!(e.access(1), None);
        assert_eq!(e.rank(7), 0);
        assert_eq!(e.rank(8), 1);
        assert_eq!(e.successor(7), Some(0));
        assert_eq!(e.successor_strict(7), None);
    }

    #[test]
    fn all_zeros_and_duplicates() {
        let e = EliasFano::new(&[0, 0, 0, 5, 5]);
        assert_eq!(e.access(0), Some(0));
        assert_eq!(e.access(3), Some(5));
        assert_eq!(e.rank(0), 0);
        assert_eq!(e.rank(1), 3);
        assert_eq!(e.rank(5), 3);
        assert_eq!(e.rank(6), 5);
        assert_eq!(e.successor_strict(5), None);
        assert_eq!(e.successor(5), Some(3));
    }

    #[test]
    fn matches_vec_oracle_random() {
        let mut rng = SplitMix64::new(0xE1A5_F4A0);
        for _ in 0..200 {
            let n = 1 + (rng.next_u64() % 60) as usize;
            let span = rng.next_u64() % 3;
            let vals: Vec<u64> = (0..n)
                .map(|_| match span {
                    0 => rng.next_u64() % 8,      // dense, many dups
                    1 => rng.next_u64() % 10_000, // medium
                    _ => rng.next_u64(),          // full range
                })
                .collect();
            let mut sorted = vals.clone();
            sorted.sort_unstable();
            let ef = EliasFano::new(&vals);
            assert_eq!(ef.len(), n);
            for (i, &v) in sorted.iter().enumerate() {
                assert_eq!(ef.access(i), Some(v), "access({i}) over {sorted:?}");
            }
            assert_eq!(ef.access(n), None);
            for _ in 0..20 {
                let q = rng.next_u64() % sorted[n - 1].saturating_add(2).max(1);
                let want = sorted.partition_point(|&x| x < q);
                assert_eq!(ef.rank(q), want, "rank({q}) over {sorted:?}");
                let ws = sorted.iter().position(|&x| x >= q);
                assert_eq!(ef.successor(q), ws, "successor({q}) over {sorted:?}");
            }
        }
    }

    #[test]
    fn successor_strict_matches_oracle() {
        let mut rng = SplitMix64::new(0xBEEF);
        for _ in 0..100 {
            let n = 1 + (rng.next_u64() % 40) as usize;
            let vals: Vec<u64> = (0..n).map(|_| rng.next_u64() % 500).collect();
            let mut sorted = vals.clone();
            sorted.sort_unstable();
            let ef = EliasFano::new(&vals);
            for q in [0u64, 1, 250, 499, 500, 10_000] {
                let want = sorted.iter().position(|&x| x > q);
                assert_eq!(
                    ef.successor_strict(q),
                    want,
                    "succ_strict({q}) over {sorted:?}"
                );
            }
        }
    }

    #[test]
    fn insertion_order_invariant() {
        let ea = EliasFano::new(&[9u64, 1, 9, 4, 1]);
        let eb = EliasFano::new(&[1u64, 9, 4, 1, 9]);
        assert_eq!(ea.hi, eb.hi);
        assert_eq!(ea.low, eb.low);
        for i in 0..5 {
            assert_eq!(ea.access(i), eb.access(i));
        }
    }
}

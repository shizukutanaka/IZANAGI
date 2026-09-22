//! Wavelet matrix over `u32` sequences (Claude & Navarro 2012): a
//! `bits × n` layered bitmap that answers `access`, `rank`, range
//! frequency, and `k`-th-smallest `quantile` queries in `O(bits)` —
//! all integer, allocation-once, byte-order independent. Complements
//! `suffix`/`rmq` for "how many of this value in this window" and
//! "median of a range" queries on tile ids, entity kinds, or key
//! streams.
//!
//! ```
//! use izanagi_kit::wavelet::WaveletMatrix;
//! let wm = WaveletMatrix::new(&[5, 1, 5, 2, 1, 5]);
//! assert_eq!(wm.access(2), Some(5));
//! assert_eq!(wm.rank(5, 6), Some(3));
//! assert_eq!(wm.range_freq(0, 6, 1, 6), Some(6));
//! assert_eq!(wm.quantile(0, 6, 3), Some(5));
//! ```

/// Wavelet matrix: `levels[d]` stores the prefix-ones table for the
/// bit-plane at bit `levels.len()-1-d` (MSB first). `zeros[d]` is the
/// count of zeros at that plane — the boundary where the stable
/// partition splits zero/one runs.
pub struct WaveletMatrix {
    n: usize,
    bits: u32,
    /// `ones[d][i]` = number of set bits in plane `d` before index `i`.
    ones: Vec<Vec<u32>>,
    /// `zeros[d]` = number of zero bits on plane `d` (split point).
    zeros: Vec<usize>,
}

impl WaveletMatrix {
    /// Build over `data`. The number of levels is the minimum needed
    /// to represent the maximum element (32 for elements ≥ 2³¹).
    pub fn new(data: &[u32]) -> Self {
        let n = data.len();
        let max = data.iter().copied().max().unwrap_or(0);
        let bits = u32::BITS - max.leading_zeros();
        let mut ones = Vec::with_capacity(bits as usize);
        let mut zeros = Vec::with_capacity(bits as usize);
        let mut cur: Vec<u32> = data.to_vec();
        for d in (0..bits).rev() {
            let mut pref = Vec::with_capacity(n + 1);
            pref.push(0u32);
            for &v in &cur {
                let bit = (v >> d) & 1;
                pref.push(pref[pref.len() - 1] + bit);
            }
            let zero_cnt = n - pref[n] as usize;
            // Stable partition: zeros then ones.
            let mut next = Vec::with_capacity(n);
            for &v in &cur {
                if (v >> d) & 1 == 0 {
                    next.push(v);
                }
            }
            for &v in &cur {
                if (v >> d) & 1 == 1 {
                    next.push(v);
                }
            }
            ones.push(pref);
            zeros.push(zero_cnt);
            cur = next;
        }
        Self {
            n,
            bits,
            ones,
            zeros,
        }
    }

    /// Number of stored elements.
    pub fn len(&self) -> usize {
        self.n
    }

    /// `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// `data[i]` reconstructed bit-by-bit. `None` when out of range.
    pub fn access(&self, i: usize) -> Option<u32> {
        if i >= self.n {
            return None;
        }
        let mut v = 0u32;
        let mut i = i;
        for (lv, (pref, &z)) in self.ones.iter().zip(&self.zeros).enumerate() {
            let d = self.bits - 1 - lv as u32;
            let bit = pref[i + 1] - pref[i];
            if bit == 1 {
                v |= 1 << d;
                i = z + pref[i] as usize;
            } else {
                i -= pref[i] as usize;
            }
        }
        Some(v)
    }

    /// Count of `x` among `data[0..i]`. `None` when `i > len`.
    /// (`x` larger than every stored element ranks 0.)
    pub fn rank(&self, x: u32, i: usize) -> Option<usize> {
        if i > self.n {
            return None;
        }
        if self.bits == 0 {
            return Some(if x == 0 { i } else { 0 });
        }
        if (x as u64) >= (1u64 << self.bits) {
            return Some(0);
        }
        // Track the matched section [l, r): equal elements occupy a
        // contiguous block, and r - l at the bottom is the count.
        let (mut l, mut r) = (0usize, i);
        for (lv, (pref, &z)) in self.ones.iter().zip(&self.zeros).enumerate() {
            let d = self.bits - 1 - lv as u32;
            if (x >> d) & 1 == 1 {
                l = z + pref[l] as usize;
                r = z + pref[r] as usize;
            } else {
                l -= pref[l] as usize;
                r -= pref[r] as usize;
            }
        }
        Some(r - l)
    }

    /// Count of elements `< x` in `data[l..r]`. `None` when `l > r`
    /// or `r > len`.
    pub fn freq_less(&self, l: usize, r: usize, x: u32) -> Option<usize> {
        if l > r || r > self.n {
            return None;
        }
        if self.bits == 0 {
            // Every stored element is 0.
            return Some(if x > 0 { r - l } else { 0 });
        }
        if x == 0 {
            return Some(0);
        }
        if (x as u64) >= (1u64 << self.bits) {
            return Some(r - l);
        }
        let (mut l, mut r) = (l, r);
        let mut ans = 0usize;
        for (lv, (pref, &z)) in self.ones.iter().zip(&self.zeros).enumerate() {
            let d = self.bits - 1 - lv as u32;
            if (x >> d) & 1 == 1 {
                // Zero-side entries in [l,r) all beat x on this bit.
                ans += (r - l) - (pref[r] - pref[l]) as usize;
                l = z + pref[l] as usize;
                r = z + pref[r] as usize;
            } else {
                l -= pref[l] as usize;
                r -= pref[r] as usize;
            }
        }
        Some(ans)
    }

    /// Count of elements in `[lo, hi)` within `data[l..r]`.
    /// `None` on bad bounds; `lo >= hi` answers 0.
    pub fn range_freq(&self, l: usize, r: usize, lo: u32, hi: u32) -> Option<usize> {
        if l > r || r > self.n {
            return None;
        }
        if lo >= hi {
            return Some(0);
        }
        Some(self.freq_less(l, r, hi)? - self.freq_less(l, r, lo)?)
    }

    /// The `k`-th smallest element of `data[l..r]` (0-indexed).
    /// `None` on bad bounds or `k >= r - l`.
    pub fn quantile(&self, l: usize, r: usize, k: usize) -> Option<u32> {
        if l > r || r > self.n || k >= r - l {
            return None;
        }
        let (mut l, mut r, mut k) = (l, r, k);
        let mut v = 0u32;
        for (lv, (pref, &z)) in self.ones.iter().zip(&self.zeros).enumerate() {
            let d = self.bits - 1 - lv as u32;
            let zc = (r - l) - (pref[r] - pref[l]) as usize;
            if k < zc {
                l -= pref[l] as usize;
                r -= pref[r] as usize;
            } else {
                v |= 1 << d;
                k -= zc;
                l = z + pref[l] as usize;
                r = z + pref[r] as usize;
            }
        }
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute-force oracle over the raw vector.
    fn oracle_rank(data: &[u32], x: u32, i: usize) -> usize {
        data[..i].iter().filter(|&&v| v == x).count()
    }
    fn oracle_freq_less(data: &[u32], l: usize, r: usize, x: u32) -> usize {
        data[l..r].iter().filter(|&&v| v < x).count()
    }
    fn oracle_quantile(data: &[u32], l: usize, r: usize, k: usize) -> u32 {
        let mut w = data[l..r].to_vec();
        w.sort_unstable();
        w[k]
    }

    #[test]
    fn all_queries_match_brute_force() {
        let mut rng = SplitMix64::new(0x4A53);
        for _ in 0..120 {
            let n = (rng.below(40) + 1) as usize;
            let vmax = 1u32 << (rng.below(5) + 1); // small alphabets too
            let data: Vec<u32> = (0..n).map(|_| rng.below(vmax)).collect();
            let wm = WaveletMatrix::new(&data);
            for (i, &d) in data.iter().enumerate() {
                assert_eq!(wm.access(i), Some(d));
            }
            for _ in 0..30 {
                let l = rng.below(n as u32) as usize;
                let r = l + rng.below((n - l + 1) as u32) as usize;
                let x = rng.below(vmax * 2);
                assert_eq!(wm.rank(x, r), Some(oracle_rank(&data, x, r)));
                assert_eq!(
                    wm.freq_less(l, r, x),
                    Some(oracle_freq_less(&data, l, r, x))
                );
                let lo = rng.below(vmax);
                let hi = lo + rng.below(vmax - lo + 1);
                assert_eq!(
                    wm.range_freq(l, r, lo, hi),
                    Some(
                        oracle_freq_less(&data, l, r, hi)
                            .saturating_sub(oracle_freq_less(&data, l, r, lo))
                    )
                );
                if r > l {
                    let k = rng.below((r - l) as u32) as usize;
                    assert_eq!(wm.quantile(l, r, k), Some(oracle_quantile(&data, l, r, k)));
                }
            }
        }
    }

    #[test]
    fn boundary_cases() {
        let empty = WaveletMatrix::new(&[]);
        assert!(empty.is_empty());
        assert_eq!(empty.access(0), None);
        assert_eq!(empty.rank(0, 0), Some(0));
        assert_eq!(empty.quantile(0, 0, 0), None);
        // All zeros → zero bit-planes.
        let z = WaveletMatrix::new(&[0, 0, 0]);
        assert_eq!(z.rank(0, 3), Some(3));
        assert_eq!(z.rank(1, 3), Some(0));
        assert_eq!(z.quantile(0, 3, 1), Some(0));
        assert_eq!(z.range_freq(0, 3, 0, 1), Some(3));
        // Full-width values exercise the top bit plane.
        let hi = WaveletMatrix::new(&[0, u32::MAX, 1]);
        assert_eq!(hi.access(1), Some(u32::MAX));
        assert_eq!(hi.freq_less(0, 3, u32::MAX), Some(2));
        assert_eq!(hi.quantile(0, 3, 2), Some(u32::MAX));
        // Bad ranges rejected.
        assert_eq!(hi.rank(0, 4), None);
        assert_eq!(hi.freq_less(2, 1, 0), None);
    }

    #[test]
    fn rank_select_consistency() {
        // rank(x, access-inverted position) counts a prefix ending at
        // that occurrence — validate via per-value occurrence order.
        let data = vec![3u32, 1, 3, 1, 3, 0, 2];
        let wm = WaveletMatrix::new(&data);
        for &v in &[0u32, 1, 2, 3, 7] {
            for i in 0..=data.len() {
                let expected = data[..i].iter().filter(|&&x| x == v).count();
                assert_eq!(wm.rank(v, i), Some(expected));
            }
        }
    }
}

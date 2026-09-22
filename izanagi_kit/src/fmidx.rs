//! FM-index — substring counting and locating over the cyclic
//! Burrows–Wheeler transform.
//!
//! Ferragina–Manzini backward search answers "how many rotations start
//! with `pat`" in `O(|pat|)` rank queries on the BWT column — no
//! per-match scan. [`FmIndex`] stores the cyclic [`crate::bwt`]
//! output plus a `C` table (per-byte prefix counts) and `Occ`
//! checkpoints every 32 rows; `locate` resolves the matching rows to
//! text positions with the same suffix-array sampling the BWT was
//! built from (K=1 sampling — full positions, no LF walking).
//!
//! Occurrences are **cyclic**: a match starting at `p` compares
//! `text[p]`, `text[(p+1) mod n]`, … — so a pattern may wrap the end
//! of the text. This is the semantics the cyclic BWT gives for free
//! and matches rotation-style use (circular buffers, token rings);
//! callers wanting strictly linear matches filter `p + pat.len() <= n`.
//!
//! ```
//! use izanagi_kit::fmidx::FmIndex;
//! let idx = FmIndex::new(b"abracadabra").unwrap();
//! assert_eq!(idx.count(b"abra"), 2); // positions 0 and 7
//! assert_eq!(idx.locate(b"bra"), vec![1, 8]);
//! assert_eq!(idx.count(b"xyz"), 0);
//! ```

use crate::suffix::SuffixArray;

/// Occupancy checkpoint stride (rows between full 256-entry tallies).
const OCC_STEP: usize = 32;

/// FM-index over a cyclic BWT: C table + spaced Occ checkpoints +
/// the row→rotation-start suffix array.
#[derive(Clone, Debug)]
pub struct FmIndex {
    /// BWT last column.
    l: Vec<u8>,
    /// `c[b]` = number of bytes in the text strictly less than `b`.
    c: [u32; 256],
    /// `occ[k][b]` = count of `b` in `l[..k * OCC_STEP]`; `occ` has
    /// `n / STEP + 1` entries (entry 0 is all zeros).
    occ: Vec<[u32; 256]>,
    /// Rotation start for each BWT row (`sa[i]` = text position whose
    /// rotation sorts at row `i`).
    sa: Vec<u32>,
}

impl FmIndex {
    /// Build over `text` — `None` on empty input (same contract as
    /// [`crate::bwt::bwt`]). `O(n log n)` via the doubled-string
    /// suffix array.
    pub fn new(text: &[u8]) -> Option<Self> {
        let n = text.len();
        if n == 0 || n > u32::MAX as usize {
            return None;
        }
        let mut dd = Vec::with_capacity(2 * n);
        dd.extend_from_slice(text);
        dd.extend_from_slice(text);
        let sa_dd = SuffixArray::new(&dd);
        let mut l = Vec::with_capacity(n);
        let mut sa = Vec::with_capacity(n);
        for &p in sa_dd.sa() {
            if p < n {
                sa.push(p as u32);
                l.push(text[(p + n - 1) % n]);
            }
        }
        let mut c = [0u32; 256];
        for &b in &l {
            c[b as usize] += 1;
        }
        // Prefix-accumulate into the C table.
        let mut acc = 0u32;
        for e in c.iter_mut() {
            let t = *e;
            *e = acc;
            acc += t;
        }
        // occ[k] = counts in l[..k*STEP]; the last entry (when
        // len % STEP == 0) holds whole-column counts for rank(., n).
        let mut occ = vec![[0u32; 256]; l.len().div_ceil(OCC_STEP) + 1];
        for (k, chunk) in l.chunks(OCC_STEP).enumerate() {
            occ[k + 1] = occ[k];
            for &b in chunk {
                occ[k + 1][b as usize] += 1;
            }
        }
        Some(FmIndex { l, c, occ, sa })
    }

    /// Text length (== BWT column length).
    pub fn len(&self) -> usize {
        self.l.len()
    }

    /// `true` when empty — `new` rejects empty input, so always `false`.
    pub fn is_empty(&self) -> bool {
        self.l.is_empty()
    }

    /// Number of `b` in `l[..i]`.
    fn rank(&self, b: u8, i: usize) -> u32 {
        let k = i / OCC_STEP;
        let mut n = self.occ[k][b as usize];
        for &x in &self.l[k * OCC_STEP..i] {
            if x == b {
                n += 1;
            }
        }
        n
    }

    /// Backward-search row range `[lo, hi)` matching `pat` — internal.
    fn range(&self, pat: &[u8]) -> (usize, usize) {
        let (mut lo, mut hi) = (0usize, self.l.len());
        for &b in pat.iter().rev() {
            lo = self.c[b as usize] as usize + self.rank(b, lo) as usize;
            hi = self.c[b as usize] as usize + self.rank(b, hi) as usize;
            if lo >= hi {
                break;
            }
        }
        (lo, hi)
    }

    /// Number of cyclic occurrences — `pat` empty matches every
    /// rotation (`count == len`).
    pub fn count(&self, pat: &[u8]) -> usize {
        let (lo, hi) = self.range(pat);
        hi - lo
    }

    /// Sorted text positions where `pat` occurs cyclically.
    pub fn locate(&self, pat: &[u8]) -> Vec<u32> {
        let (lo, hi) = self.range(pat);
        let mut out: Vec<u32> = self.sa[lo..hi].to_vec();
        out.sort_unstable();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Oracle: count/positions by direct cyclic comparison at every start.
    fn oracle(text: &[u8], pat: &[u8]) -> Vec<u32> {
        let n = text.len();
        let mut out = Vec::new();
        for s in 0..n {
            if (0..pat.len()).all(|k| text[(s + k) % n] == pat[k]) {
                out.push(s as u32);
            }
        }
        out
    }

    #[test]
    fn matches_naive_oracle() {
        let mut rng = SplitMix64::new(0xFA1D);
        for _ in 0..300 {
            let n = (rng.below(20) + 1) as usize;
            let text: Vec<u8> = (0..n).map(|_| rng.below(4) as u8 + b'a').collect();
            let idx = FmIndex::new(&text).unwrap();
            for _ in 0..4 {
                let m = (rng.below(7) + 1) as usize;
                let pat: Vec<u8> = (0..m).map(|_| rng.below(4) as u8 + b'a').collect();
                let want = oracle(&text, &pat);
                assert_eq!(idx.count(&pat), want.len(), "text={text:?} pat={pat:?}");
                assert_eq!(idx.locate(&pat), want, "text={text:?} pat={pat:?}");
            }
        }
    }

    #[test]
    fn cyclic_wraps_and_empty_patterns() {
        let idx = FmIndex::new(b"ab").unwrap();
        // "ba" occurs only cyclically (position 1 wraps to position 0).
        assert_eq!(idx.locate(b"ba"), vec![1]);
        assert_eq!(idx.count(b""), 2, "empty pattern matches all rotations");
        let idx = FmIndex::new(b"aaa").unwrap();
        assert_eq!(idx.locate(b"aaaa"), vec![0, 1, 2], "wraps multiple times");
        assert_eq!(idx.locate(b"a"), vec![0, 1, 2]);
        let idx = FmIndex::new(b"ababab").unwrap();
        // "bab" at 1,3; at 5 wraps to (5,0,1) = 'b','a','b' — cyclic match.
        assert_eq!(idx.locate(b"bab"), vec![1, 3, 5]);
        let s: BTreeSet<u32> = idx.locate(b"ababab").into_iter().collect();
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn rejects_empty() {
        assert!(FmIndex::new(b"").is_none());
        let idx = FmIndex::new(b"z").unwrap();
        assert_eq!(idx.len(), 1);
        assert!(!idx.is_empty());
        assert_eq!(idx.count(b"z"), 1);
        assert_eq!(idx.count(b"zz"), 1, "single-byte text wraps");
    }
}

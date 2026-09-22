//! Bit-parallel string search — the Shift-And / Bitap algorithm
//! (Baeza–Yates & Gonnet 1992, the basis of `agrep` and `approx-grep`).
//!
//! The pattern is compiled into a 256-entry bitmask table `masks[c]`:
//! bit `i` set iff `pattern[i] == c`. Scanning text is one `u64`
//! shift-OR-AND per byte:
//!
//! ```text
//! state = ((state << 1) | 1) & masks[c]
//! ```
//!
//! Bit `i` of `state` then means "a suffix of the text seen so far
//! equals `pattern[..=i]`", so a hit is bit `m−1` being set. The whole
//! simulation is data-parallel and branch-free — no backtracking,
//! which is why it generalizes to **fuzzy** (Hamming-distance-`k`)
//! matching: keep `k+1` state rows, where row `r` tolerates `r`
//! substitutions via the transition `| (old[r−1] << 1)`.
//!
//! Limit: patterns of at most 64 bytes (one `u64` machine word —
//! multi-word variants exist but the single-word case is the one
//! the literature calls Shift-And). Longer patterns fail closed
//! (`None`). Occurrences are reported by **end position** — the index
//! just past the last matched byte, the same convention
//! [`crate::kmp`] uses.
//!
//! ```
//! use izanagi_kit::bitap::Bitap;
//! let b = Bitap::new(b"needle").unwrap();
//! assert_eq!(b.search(b"find the needle here"), vec![15]);
//! // One substitution tolerated: "needlf" still matches.
//! assert_eq!(b.fuzzy_search(b"find the needlf here", 1), vec![15]);
//! ```

/// A compiled pattern — exact and Hamming-fuzzy `u64`-word search.
#[derive(Clone, Debug)]
pub struct Bitap {
    masks: [u64; 256],
    m: usize,
}

impl Bitap {
    /// Compile `pattern` — `None` when it is empty or longer than 64
    /// bytes (the single-word bound; longer patterns belong to a
    /// different algorithm like [`crate::kmp`] or [`crate::ahocor`]).
    pub fn new(pattern: &[u8]) -> Option<Self> {
        let m = pattern.len();
        if m == 0 || m > 64 {
            return None;
        }
        let mut masks = [0u64; 256];
        for (i, &c) in pattern.iter().enumerate() {
            masks[c as usize] |= 1u64 << i;
        }
        Some(Bitap { masks, m })
    }

    /// Pattern length in bytes.
    pub fn len(&self) -> usize {
        self.m
    }

    /// Always `false` — `new` rejects empty patterns.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Exact matches — end positions (index past the hit) in
    /// ascending order. Overlapping occurrences all reported.
    pub fn search(&self, text: &[u8]) -> Vec<usize> {
        let top = 1u64 << (self.m - 1);
        let mut state = 0u64;
        let mut hits = Vec::new();
        for (i, &c) in text.iter().enumerate() {
            state = ((state << 1) | 1) & self.masks[c as usize];
            if state & top != 0 {
                hits.push(i + 1);
            }
        }
        hits
    }

    /// Hamming-fuzzy matches — end positions where the window ending
    /// at `i` differs from the pattern in at most `k` positions.
    /// `k = 0` equals [`Bitap::search`]; `k ≥ m` matches every window
    /// (each position can be substituted). Overlaps all reported.
    pub fn fuzzy_search(&self, text: &[u8], k: usize) -> Vec<usize> {
        let k = k.min(self.m);
        let top = 1u64 << (self.m - 1);
        // states[r] = matched prefix bits with at most r substitutions.
        let mut states = vec![0u64; k + 1];
        let mut hits = Vec::new();
        for (i, &c) in text.iter().enumerate() {
            let mask = self.masks[c as usize];
            let mut prev = states[0];
            states[0] = ((states[0] << 1) | 1) & mask;
            for st in states.iter_mut().skip(1) {
                let old = *st;
                // Either the char matches (same-row advance) or we
                // spend a substitution (advance from row r-1). Both
                // terms seed bit 0: the empty prefix is always true.
                *st = (((old << 1) | 1) & mask) | ((prev << 1) | 1);
                prev = old;
            }
            if states[k] & top != 0 {
                hits.push(i + 1);
            }
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Naive oracle: every window's Hamming distance to the pattern.
    fn naive(text: &[u8], pat: &[u8], k: usize) -> Vec<usize> {
        let m = pat.len();
        let mut hits = Vec::new();
        for end in m..=text.len() {
            let win = &text[end - m..end];
            let d = win.iter().zip(pat.iter()).filter(|(a, b)| a != b).count();
            if d <= k {
                hits.push(end);
            }
        }
        hits
    }

    fn random_string(rng: &mut SplitMix64, len: usize, alphabet: usize) -> Vec<u8> {
        (0..len)
            .map(|_| b'a' + rng.below(alphabet as u32) as u8)
            .collect()
    }

    #[test]
    fn exact_and_fuzzy_match_naive_oracle() {
        let mut rng = SplitMix64::new(0xB17A);
        for _ in 0..400 {
            let m = 1 + rng.below(12) as usize;
            let pat = random_string(&mut rng, m, 4);
            let tlen = rng.below(80) as usize;
            let text = random_string(&mut rng, tlen, 4);
            let b = Bitap::new(&pat).unwrap();
            assert_eq!(b.search(&text), naive(&text, &pat, 0));
            let k = rng.below(4) as usize;
            assert_eq!(
                b.fuzzy_search(&text, k),
                naive(&text, &pat, k),
                "fuzzy k={k} pat={pat:?} text={text:?}"
            );
        }
    }

    #[test]
    fn planted_hits_and_overlap() {
        let b = Bitap::new(b"aa").unwrap();
        // aaa contains two overlapping occurrences ending at 2 and 3.
        assert_eq!(b.search(b"aaa"), vec![2, 3]);
        assert_eq!(b.search(b"bbb"), Vec::<usize>::new());
        let b = Bitap::new(b"a").unwrap();
        assert_eq!(b.search(b"axaxa"), vec![1, 3, 5]);
    }

    #[test]
    fn fuzzy_finds_substituted_needle() {
        let b = Bitap::new(b"needle").unwrap();
        assert_eq!(b.len(), 6);
        assert!(!b.is_empty());
        assert_eq!(b.search(b"find the needlf here"), Vec::<usize>::new());
        assert_eq!(b.fuzzy_search(b"find the needlf here", 1), vec![15]);
        assert_eq!(b.fuzzy_search(b"find the nxxdle here", 2), vec![15]);
        // Insertions/deletions are NOT tolerated — Hamming only —
        // but the shifted window "needl " still matches at k=1 by
        // substituting the trailing space for the final 'e'.
        assert_eq!(b.fuzzy_search(b"find the needl here", 1), vec![15]);
        assert!(b.fuzzy_search(b"find the need here", 1).is_empty());
    }

    #[test]
    fn boundaries_and_validation() {
        assert!(Bitap::new(b"").is_none());
        assert!(Bitap::new(&[0u8; 65]).is_none());
        let b = Bitap::new(&[0u8; 64]).unwrap(); // exactly one word
        assert_eq!(b.len(), 64);
        assert_eq!(b.search(&[0u8; 64]), vec![64]);
        // k >= m: every full-length window matches.
        let b = Bitap::new(b"ab").unwrap();
        assert_eq!(b.fuzzy_search(b"xyzw", 2), vec![2, 3, 4]);
        // Text shorter than the pattern: no hits.
        assert!(b.search(b"a").is_empty());
    }
}

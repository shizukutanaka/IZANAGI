//! Rabin–Karp rolling hashes and content-defined chunking — the
//! fingerprint layer for [`delta`](crate::delta)-style sync: fixed
//! window hashes let you answer "where did this byte stream change?"
//! by comparing fingerprints instead of contents, and CDC cuts
//! variable-length chunks at content-defined boundaries so a local
//! edit only perturbs nearby chunks.
//!
//! The hash is a mod-2^64 polynomial `Σ bᵢ·B^(n−1−i)` — all wrapping
//! arithmetic, so results are bit-identical everywhere. Chunk
//! boundaries are emitted when `hash & mask == 0` inside the
//! `[min, max]` enforcement window.
//!
//! ```
//! use izanagi_kit::rolling::Rolling;
//! let mut r = Rolling::with_capacity(4);
//! for &b in b"abcd" { r.push(b); }
//! let h = r.hash();
//! r.pop();
//! r.push(b'e');
//! let mut r2 = Rolling::new();
//! for &b in b"bcd" { r2.push(b); }
//! r2.push(b'e');
//! assert_eq!(r.hash(), r2.hash());
//! ```

use std::collections::VecDeque;

/// Polynomial base — odd constant, chosen so byte inputs spread the
/// low bits that CDC masks inspect.
const BASE: u64 = 0x0001_0000_0100_01B3;

/// A fixed-window rolling hash over bytes.
pub struct Rolling {
    q: VecDeque<u8>,
    h: u64,
    pow: Vec<u64>, // pow[i] = BASE^i, grown on demand
}

impl Rolling {
    /// Window with a small default capacity (grows on demand).
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    /// Window pre-sized for `cap` bytes — the CDC use case.
    pub fn with_capacity(cap: usize) -> Self {
        let mut pow = Vec::with_capacity(cap + 1);
        pow.push(1u64);
        for i in 0..cap {
            let p = pow[i].wrapping_mul(BASE);
            pow.push(p);
        }
        Self {
            q: VecDeque::with_capacity(cap),
            h: 0,
            pow,
        }
    }

    /// Append `b` to the window.
    pub fn push(&mut self, b: u8) {
        self.h = self.h.wrapping_mul(BASE).wrapping_add(b as u64);
        self.q.push_back(b);
        // pow must cover len-1 on the next pop.
        if self.pow.len() < self.q.len() {
            let p = self.pow[self.pow.len() - 1].wrapping_mul(BASE);
            self.pow.push(p);
        }
    }

    /// Remove the oldest byte. `None` when the window is empty.
    pub fn pop(&mut self) -> Option<u8> {
        let b = self.q.pop_front()?;
        // h currently = Σ bᵢ·B^{n−1−i}; dropping the front removes
        // b₀·B^{n−1} = b₀·pow[n−1] (pow[len] exists — push ensured it).
        self.h = self
            .h
            .wrapping_sub((b as u64).wrapping_mul(self.pow[self.q.len()]));
        Some(b)
    }

    /// Current window hash.
    pub fn hash(&self) -> u64 {
        self.h
    }

    /// Bytes currently in the window.
    pub fn len(&self) -> usize {
        self.q.len()
    }

    /// Whether the window is empty.
    pub fn is_empty(&self) -> bool {
        self.q.is_empty()
    }
}

impl Default for Rolling {
    fn default() -> Self {
        Self::new()
    }
}

/// One-shot polynomial hash of `data`.
pub fn hash_bytes(data: &[u8]) -> u64 {
    let mut h = 0u64;
    for &b in data {
        h = h.wrapping_mul(BASE).wrapping_add(b as u64);
    }
    h
}

/// All start positions of `pat` in `text` via rolling fingerprints,
/// confirmed byte-wise — hash collisions can only add candidates, and
/// the confirm pass removes them, so the result is exact.
pub fn find_all(text: &[u8], pat: &[u8]) -> Vec<usize> {
    if pat.is_empty() {
        return (0..=text.len()).collect();
    }
    if pat.len() > text.len() {
        return Vec::new();
    }
    let target = hash_bytes(pat);
    let mut r = Rolling::with_capacity(pat.len());
    let mut out = Vec::new();
    for &b in text.iter().take(pat.len()) {
        r.push(b);
    }
    for start in 0..=(text.len() - pat.len()) {
        if start > 0 {
            r.pop();
            r.push(text[start + pat.len() - 1]);
        }
        if r.hash() == target && &text[start..start + pat.len()] == pat {
            out.push(start);
        }
    }
    out
}

/// Content-defined chunk boundaries. `mask` sets the expected chunk
/// size (`mask = 2^k − 1` ⇒ ~2^k bytes on average); boundaries are
/// enforced to keep every chunk inside `[min, max]` (the final chunk
/// may be shorter than `min`). Returns cut positions in ascending
/// order; `0` and `data.len()` are implicit, not included.
///
/// ```
/// use izanagi_kit::rolling::chunks;
/// let data = vec![0u8; 10_000];
/// let cuts = chunks(&data, 128, 255, 1024);
/// for w in cuts.windows(2) { assert!(w[1] - w[0] <= 1024); }
/// ```
pub fn chunks(data: &[u8], min: usize, mask: u64, max: usize) -> Vec<usize> {
    const W: usize = 48; // fingerprint window (rsync-style)
    let mut cuts = Vec::new();
    let mut r = Rolling::with_capacity(W);
    let mut base = 0usize; // start of the open chunk
    for (i, &b) in data.iter().enumerate() {
        r.push(b);
        if r.len() > W {
            r.pop();
        }
        let len = i + 1 - base;
        if len < min {
            continue;
        }
        if (r.hash() & mask) == 0 || len >= max {
            cuts.push(i + 1);
            base = i + 1;
            r = Rolling::with_capacity(W);
        }
    }
    cuts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn naive_all(text: &[u8], pat: &[u8]) -> Vec<usize> {
        if pat.is_empty() {
            return (0..=text.len()).collect();
        }
        (0..=(text.len().saturating_sub(pat.len())))
            .filter(|&i| text[i..].starts_with(pat))
            .collect()
    }

    #[test]
    fn rolling_matches_recompute() {
        let mut rng = SplitMix64::new(0xB01D);
        for _ in 0..300 {
            let n = rng.below(40) as usize;
            let data: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
            let mut r = Rolling::with_capacity(8);
            for &b in &data {
                r.push(b);
                if r.len() > 8 {
                    r.pop();
                }
            }
            let w = r.len();
            assert_eq!(r.hash(), hash_bytes(&data[data.len() - w..]));
        }
        // pop drains in order (Default == new).
        let mut r = Rolling::default();
        for &b in b"xy" {
            r.push(b);
        }
        assert_eq!(r.pop(), Some(b'x'));
        assert_eq!(r.pop(), Some(b'y'));
        assert_eq!(r.pop(), None);
        assert_eq!(r.hash(), 0);
    }

    #[test]
    fn find_all_matches_naive() {
        let mut rng = SplitMix64::new(0xCAFE);
        for _ in 0..400 {
            let text: Vec<u8> = (0..rng.below(80))
                .map(|_| (rng.next_u64() % 3) as u8 + b'a')
                .collect();
            let pat: Vec<u8> = (0..rng.below(6))
                .map(|_| (rng.next_u64() % 3) as u8 + b'a')
                .collect();
            assert_eq!(find_all(&text, &pat), naive_all(&text, &pat));
        }
    }

    #[test]
    fn chunks_are_deterministic_and_bounded() {
        let mut rng = SplitMix64::new(0xCDC0);
        for _ in 0..60 {
            let n = rng.below(6000) as usize;
            let data: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
            let (min, mask, max) = (64usize, 255u64, 1024usize);
            let cuts = chunks(&data, min, mask, max);
            assert_eq!(cuts, chunks(&data, min, mask, max));
            let mut prev = 0usize;
            for &c in &cuts {
                let l = c - prev;
                assert!(l >= min && l <= max, "chunk {prev}..{c} len {l}");
                prev = c;
            }
            // Last (open) chunk may be short but never exceeds max.
            assert!(data.len() - prev <= max);
        }
    }

    #[test]
    fn local_edit_perturbs_only_a_local_region() {
        let mut rng = SplitMix64::new(0xED17);
        let n = 8192usize;
        let mut data: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
        let cuts_a = chunks(&data, 64, 255, 1024);
        // Flip one byte in the middle; boundaries strictly before the
        // fingerprint window of the edit must survive verbatim.
        let edit = 4000usize;
        data[edit] ^= 0x5A;
        let cuts_b = chunks(&data, 64, 255, 1024);
        let safe = edit.saturating_sub(48 + 1024);
        let prefix_a: Vec<usize> = cuts_a.iter().copied().take_while(|&c| c <= safe).collect();
        let prefix_b: Vec<usize> = cuts_b.iter().copied().take_while(|&c| c <= safe).collect();
        assert_eq!(prefix_a, prefix_b);
    }
}

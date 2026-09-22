//! Knuth–Morris–Pratt single-pattern byte search — O(text) matching
//! after O(pattern) preprocessing, with no random-access backtracking.
//! [`crate::ahocor`] covers the multi-pattern case; this is the cheap
//! one-pattern layer for delimiters, headers, and protocol sentinels,
//! and the streaming variant accepts bytes one at a time so it works
//! on packet chunks whose boundaries split a match.
//!
//! ```
//! use izanagi_kit::kmp::Kmp;
//! let k = Kmp::new(b"aa");
//! assert_eq!(k.find_all(b"baaaa"), vec![1, 2, 3]);
//! assert_eq!(k.find(b"abcdef"), None);
//! ```

/// A preprocessed single-pattern matcher.
pub struct Kmp {
    pat: Vec<u8>,
    /// `fail[j]` = length of the longest proper border of pat[..j].
    fail: Vec<u32>,
}

impl Kmp {
    /// Preprocess `pat`. An empty pattern matches at every position
    /// (std `str::find` semantics).
    pub fn new(pat: &[u8]) -> Self {
        let mut fail = vec![0u32; pat.len() + 1];
        let mut j = 0usize;
        for i in 1..pat.len() {
            while j > 0 && pat[i] != pat[j] {
                j = fail[j] as usize;
            }
            if pat[i] == pat[j] {
                j += 1;
            }
            fail[i + 1] = j as u32;
        }
        Self {
            pat: pat.to_vec(),
            fail,
        }
    }

    /// The pattern this matcher was built from.
    pub fn pattern(&self) -> &[u8] {
        &self.pat
    }

    /// Start index of the first match, or `None`. An empty pattern
    /// matches at 0.
    pub fn find(&self, text: &[u8]) -> Option<usize> {
        if self.pat.is_empty() {
            return Some(0);
        }
        let mut j = 0usize;
        for (i, &b) in text.iter().enumerate() {
            j = self.step(j, b);
            if j == self.pat.len() {
                return Some(i + 1 - self.pat.len());
            }
        }
        None
    }

    /// All match start indices in order, including overlapping hits.
    /// An empty pattern yields `0..=text.len()` (std semantics).
    pub fn find_all(&self, text: &[u8]) -> Vec<usize> {
        let mut out = Vec::new();
        if self.pat.is_empty() {
            out.extend(0..=text.len());
            return out;
        }
        let mut j = 0usize;
        for (i, &b) in text.iter().enumerate() {
            j = self.step(j, b);
            if j == self.pat.len() {
                out.push(i + 1 - self.pat.len());
                j = self.fail[j] as usize;
            }
        }
        out
    }

    /// Number of matches (count of [`find_all`](Self::find_all)).
    pub fn count(&self, text: &[u8]) -> usize {
        self.find_all(text).len()
    }

    /// Start a streaming matcher: feed bytes one at a time, get match
    /// end-driven results without materializing the whole text.
    pub fn stream(&self) -> Stream<'_> {
        Stream {
            kmp: self,
            j: 0,
            pos: 0,
        }
    }

    /// Advance state `j` by one byte; returns the new matched length.
    fn step(&self, mut j: usize, b: u8) -> usize {
        while j > 0 && (j >= self.pat.len() || b != self.pat[j]) {
            j = self.fail[j] as usize;
        }
        if j < self.pat.len() && b == self.pat[j] {
            j + 1
        } else {
            j
        }
    }
}

/// Streaming matcher — feed bytes via [`feed`](Stream::feed).
/// Position is tracked so a match spanning chunk boundaries still
/// reports the correct absolute start index.
pub struct Stream<'a> {
    kmp: &'a Kmp,
    j: usize,
    pos: usize,
}

impl Stream<'_> {
    /// Feed one byte; `Some(start)` when a match just completed.
    /// An empty pattern matches at every position (returns `Some(pos)`).
    pub fn feed(&mut self, b: u8) -> Option<usize> {
        let i = self.pos;
        self.pos += 1;
        if self.kmp.pat.is_empty() {
            return Some(i);
        }
        self.j = self.kmp.step(self.j, b);
        if self.j == self.kmp.pat.len() {
            let start = i + 1 - self.kmp.pat.len();
            self.j = self.kmp.fail[self.j] as usize;
            return Some(start);
        }
        None
    }

    /// Bytes consumed so far.
    pub fn pos(&self) -> usize {
        self.pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn naive_all(text: &[u8], pat: &[u8]) -> Vec<usize> {
        if pat.is_empty() {
            return (0..=text.len()).collect();
        }
        if pat.len() > text.len() {
            return Vec::new();
        }
        (0..=text.len() - pat.len())
            .filter(|&i| text[i..i + pat.len()] == *pat)
            .collect()
    }

    #[test]
    fn find_all_match_naive_scan() {
        let mut rng = SplitMix64::new(0x0F5D);
        for _ in 0..600 {
            let tlen = rng.below(300) as usize;
            let plen = rng.below(8) as usize;
            let alpha = 1 + rng.below(4); // tiny alphabet → many overlaps
            let text: Vec<u8> = (0..tlen).map(|_| rng.below(alpha) as u8 + b'a').collect();
            let pat: Vec<u8> = (0..plen).map(|_| rng.below(alpha) as u8 + b'a').collect();
            let k = Kmp::new(&pat);
            assert_eq!(k.find_all(&text), naive_all(&text, &pat));
            assert_eq!(k.find(&text), naive_all(&text, &pat).first().copied());
            assert_eq!(k.count(&text), naive_all(&text, &pat).len());
        }
    }

    #[test]
    fn stream_matches_batch_for_every_chunking() {
        let mut rng = SplitMix64::new(0x5EED);
        for _ in 0..300 {
            let tlen = rng.below(200) as usize;
            let text: Vec<u8> = (0..tlen).map(|_| rng.below(3) as u8 + b'x').collect();
            let pat: Vec<u8> = (0..1 + rng.below(6) as usize)
                .map(|_| rng.below(3) as u8 + b'x')
                .collect();
            let k = Kmp::new(&pat);
            let mut s = k.stream();
            let streamed: Vec<usize> = text.iter().filter_map(|&b| s.feed(b)).collect();
            assert_eq!(streamed, k.find_all(&text));
            assert_eq!(s.pos(), text.len());
        }
    }

    #[test]
    fn overlapping_and_degenerate_cases() {
        assert_eq!(Kmp::new(b"aa").find_all(b"aaaa"), vec![0, 1, 2]);
        assert_eq!(Kmp::new(b"").find(b"abc"), Some(0));
        assert_eq!(Kmp::new(b"").find_all(b"ab"), vec![0, 1, 2]);
        assert_eq!(Kmp::new(b"abc").find(b"ab"), None);
        assert_eq!(Kmp::new(b"longer").find(b"short"), None);
        // Self-overlapping pattern "abab" in "ababab".
        assert_eq!(Kmp::new(b"abab").find_all(b"ababab"), vec![0, 2]);
    }
}

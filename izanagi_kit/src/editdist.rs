//! Myers' bit-vector edit distance — Levenshtein distance and
//! `≤ k` approximate search in `O(⌈m/64⌉ · n)` word operations.
//! `bitap` covers *Hamming* fuzzy search (substitutions only);
//! this module covers *edit* distance (insertions and deletions
//! too), and unlike `diff::levenshtein`'s quadratic table it
//! spends one column's work per word, not per cell.
//!
//! The automaton keeps the vertical frontier of the DP table as
//! two bitmasks `Pv`/`Mv` (where `d` grows/shrinks moving down a
//! column); each input byte advances it one column. A pattern
//! of `≤ 64` bytes runs in a single `u64` state; longer
//! patterns fall back to the reference DP inside the same API.
//!
//! ```
//! use izanagi_kit::editdist;
//! assert_eq!(editdist::dist(b"kitten", b"sitting"), 3);
//! // "sit" within edit distance 1 of every window
//! let ends = editdist::find_leq(b"sit", b"bit sit sat", 1);
//! assert!(ends.contains(&7));
//! ```
//!
//! References: Myers (1999) "A fast bit-vector algorithm for
//! approximate string matching based on dynamic programming";
//! Navarro & Raffinot "Flexible Pattern Matching in Strings" §6.

/// Exact Levenshtein distance between `a` and `b`.
pub fn dist(a: &[u8], b: &[u8]) -> u32 {
    if a.is_empty() {
        return b.len() as u32;
    }
    if b.is_empty() {
        return a.len() as u32;
    }
    if a.len() <= 64 {
        myers_scan(a, b, false).last().copied().unwrap_or(0)
    } else {
        crate::diff::levenshtein(a, b)
    }
}

/// Every position `i` in `text` where `dist(pattern,
/// text[..=i] suffix)` is at most `k` — i.e. end positions of
/// approximate occurrences, the `bitap` API lifted from Hamming
/// to edit distance.
pub fn find_leq(pat: &[u8], text: &[u8], k: u32) -> Vec<usize> {
    if pat.is_empty() {
        // the empty suffix always has distance 0 <= k
        return (0..text.len()).collect();
    }
    if pat.len() <= 64 {
        myers_scan(pat, text, true)
            .iter()
            .enumerate()
            .filter_map(|(i, &d)| (d <= k).then_some(i))
            .collect()
    } else {
        // DP fallback for long patterns: same semantics, O(mn)
        let m = pat.len();
        let mut prev: Vec<u32> = (0..=m as u32).collect();
        let mut cur = vec![0u32; m + 1];
        let mut out = Vec::new();
        for (i, &t) in text.iter().enumerate() {
            cur[0] = 0; // empty suffix may start anywhere
            for j in 1..=m {
                cur[j] = (prev[j - 1] + u32::from(pat[j - 1] != t))
                    .min(prev[j] + 1)
                    .min(cur[j - 1] + 1);
            }
            if cur[m] <= k {
                out.push(i);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        out
    }
}

/// Myers' one-`u64` automaton over `text` — returns the running
/// edit distance after each text byte (length `text.len()`).
///
/// `free_start` selects the LEFT-column boundary of the DP:
/// full `dist` needs `D[i][0] = i` (deleting a text prefix costs
/// its length — the `Ph<<1 | 1` injection), while `find_leq`'s
/// occurrences may begin anywhere so `D[i][0] = 0` (inject 0).
/// The top row `D[0][j] = j` (`Pv = ~0`, score `m`) is the same
/// either way — the empty text has only the empty suffix.
fn myers_scan(pat: &[u8], text: &[u8], free_start: bool) -> Vec<u32> {
    let m = pat.len();
    let mut peq = [0u64; 256];
    for (i, &c) in pat.iter().enumerate() {
        peq[c as usize] |= 1u64 << i;
    }
    let mut pv = !0u64;
    let mut mv = 0u64;
    let mut d = m as u32;
    let top = 1u64 << (m - 1);
    let mask = if m >= 64 { !0 } else { (1u64 << m) - 1 };
    let mut out = Vec::with_capacity(text.len());
    for &t in text {
        let eq = peq[t as usize] & mask;
        let xv = eq | mv;
        let xh = (((eq & pv).wrapping_add(pv)) ^ pv) | eq;
        let mut ph = mv | !(xh | pv);
        let mh = pv & xh;
        if ph & top != 0 {
            d += 1;
        }
        if mh & top != 0 {
            d -= 1;
        }
        if free_start {
            ph <<= 1;
        } else {
            ph = (ph << 1) | 1;
        }
        let mh2 = mh << 1;
        pv = mh2 | !(xv | ph);
        mv = ph & xv;
        pv &= mask;
        mv &= mask;
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Independent oracle: textbook O(mn) Levenshtein table.
    fn dp_dist(a: &[u8], b: &[u8]) -> u32 {
        crate::diff::levenshtein(a, b)
    }

    /// Independent oracle for `find_leq`: full DP of pattern
    /// prefixes vs text prefixes — d[i][j] = edit distance of
    /// pat[..j] vs text[..i] ending at i.
    fn dp_find_leq(pat: &[u8], text: &[u8], k: u32) -> Vec<usize> {
        let m = pat.len();
        let mut prev: Vec<u32> = (0..=m as u32).collect();
        let mut cur = vec![0u32; m + 1];
        let mut out = Vec::new();
        for (i, &t) in text.iter().enumerate() {
            cur[0] = 0;
            for j in 1..=m {
                cur[j] = (prev[j - 1] + u32::from(pat[j - 1] != t))
                    .min(prev[j] + 1)
                    .min(cur[j - 1] + 1);
            }
            if cur[m] <= k {
                out.push(i);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        out
    }

    #[test]
    fn known_distances() {
        assert_eq!(dist(b"kitten", b"sitting"), 3);
        assert_eq!(dist(b"", b""), 0);
        assert_eq!(dist(b"abc", b""), 3);
        assert_eq!(dist(b"", b"abc"), 3);
        assert_eq!(dist(b"flaw", b"lawn"), 2);
        // 64-byte boundary
        let a = vec![b'a'; 64];
        assert_eq!(dist(&a, &a), 0);
        assert_eq!(dist(&a, &[b'b'; 64][..]), 64);
    }

    #[test]
    fn find_leq_finds_approximate_ends() {
        // positions where "sit" occurs with <= 1 edit
        let ends = find_leq(b"sit", b"bit sit sat", 1);
        assert_eq!(ends, dp_find_leq(b"sit", b"bit sit sat", 1));
        assert!(ends.contains(&2));
        assert!(ends.contains(&6));
        assert!(ends.contains(&10));
    }

    #[test]
    fn random_oracle() {
        let mut r = SplitMix64::new(0xED17);
        for _ in 0..600 {
            let m = 1 + (r.next_u64() % 20) as usize;
            let n = (r.next_u64() % 60) as usize;
            let alpha = 1 + (r.next_u64() % 4) as usize; // tiny alphabets stress ties
            let pat: Vec<u8> = (0..m)
                .map(|_| b'a' + (r.next_u64() % alpha as u64) as u8)
                .collect();
            let text: Vec<u8> = (0..n)
                .map(|_| b'a' + (r.next_u64() % alpha as u64) as u8)
                .collect();
            assert_eq!(dist(&pat, &text), dp_dist(&pat, &text));
            let k = (r.next_u64() % 5) as u32;
            assert_eq!(find_leq(&pat, &text, k), dp_find_leq(&pat, &text, k));
        }
    }

    #[test]
    fn boundary_64() {
        let mut r = SplitMix64::new(0x64);
        for _ in 0..200 {
            let m = 60 + (r.next_u64() % 5) as usize; // 60..64
            let n = (r.next_u64() % 80) as usize;
            let pat: Vec<u8> = (0..m).map(|_| (r.next_u64() % 256) as u8).collect();
            let text: Vec<u8> = (0..n).map(|_| (r.next_u64() % 256) as u8).collect();
            assert_eq!(dist(&pat, &text), dp_dist(&pat, &text));
        }
    }

    #[test]
    fn long_pattern_falls_back() {
        let a: Vec<u8> = (0..100).map(|i| (i * 7 % 251) as u8).collect();
        let b: Vec<u8> = (0..100).map(|i| ((i * 7 % 251) + 1) as u8).collect();
        assert_eq!(dist(&a, &b), dp_dist(&a, &b));
        assert_eq!(find_leq(&a, &b, 5), dp_find_leq(&a, &b, 5));
    }
}

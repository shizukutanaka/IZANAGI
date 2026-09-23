//! Multi-word Myers bit-vector edit distance — arbitrary-length
//! patterns. [`crate::editdist`] keeps the DP frontier in one
//! `u64` and falls back to the quadratic table past 64 bytes;
//! this module runs the *same* automaton over a `Vec<u64>`
//! frontier, so a 256-byte pattern still costs only
//! `O(⌈m/64⌉)` word operations per text byte.
//!
//! The three carry chains that make a multi-word automaton
//! behave like one wide bitmask:
//! - `(Eq & Pv) + Pv` is a multi-precision add — its carry out
//!   of word `w` feeds word `w + 1` (a one-word add per word
//!   silently drops cross-word borrows).
//! - `Ph << 1` / `Mh << 1` shift the top bit of word `w − 1`
//!   into bit 0 of word `w`; word 0 gets the boundary
//!   injection (`1` for full `dist`, `0` for `find_leq`).
//! - The score update reads the *last* word's bit `m − 1`,
//!   the only cell whose horizontal delta bounds `d(m, i)`.
//!
//! ```
//! use izanagi_kit::bigedit;
//! let a = b"kittens and mittens sitting on the fence".repeat(3);
//! let b = b"kittens and mittens sitting on the fence".repeat(3);
//! assert_eq!(bigedit::dist(&a, &b), 0);
//! assert!(bigedit::dist(b"kitten", b"sitting") == 3);
//! ```
//!
//! References: Myers (1999) §4 multi-word extension; Hyyrö
//! (2003) "A bit-vector algorithm for computing Levenshtein
//! and Damerau edit distances" for the carry discipline.

/// Exact Levenshtein distance between `a` and `b`.
pub fn dist(a: &[u8], b: &[u8]) -> u32 {
    if a.is_empty() {
        return b.len() as u32;
    }
    if b.is_empty() {
        return a.len() as u32;
    }
    scan(a, b, false).last().copied().unwrap_or(0)
}

/// Every position `i` in `text` where some suffix of
/// `text[..=i]` is within edit distance `k` of `pat` — the
/// `find_leq` API lifted to arbitrary pattern length.
pub fn find_leq(pat: &[u8], text: &[u8], k: u32) -> Vec<usize> {
    if pat.is_empty() {
        return (0..text.len()).collect();
    }
    scan(pat, text, true)
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| (d <= k).then_some(i))
        .collect()
}

/// The running edit distance after each text byte — the whole
/// `D[m][i]` column of the DP, `⌈m/64⌉` words of state per
/// step. `free_start` picks the left boundary column:
/// `D[i][0] = i` for `dist`, `0` for `find_leq`.
fn scan(pat: &[u8], text: &[u8], free_start: bool) -> Vec<u32> {
    let m = pat.len();
    let w = m.div_ceil(64);
    let rem = m % 64;
    let last_mask: u64 = if rem == 0 { !0 } else { (1u64 << rem) - 1 };
    // peq[c] = bitvector of positions where pattern == c
    let mut peq = vec![0u64; 256 * w];
    for (i, &c) in pat.iter().enumerate() {
        peq[c as usize * w + i / 64] |= 1u64 << (i % 64);
    }
    let mut pv = vec![!0u64; w];
    let mut mv = vec![0u64; w];
    pv[w - 1] &= last_mask;
    let mut d = m as u32;
    let mut eq = vec![0u64; w];
    let mut xv = vec![0u64; w];
    let mut xh = vec![0u64; w];
    let mut ph = vec![0u64; w];
    let mut mh = vec![0u64; w];
    let mut out = Vec::with_capacity(text.len());
    for &t in text {
        let row = &peq[t as usize * w..(t as usize + 1) * w];
        for j in 0..w {
            eq[j] = if j == w - 1 {
                row[j] & last_mask
            } else {
                row[j]
            };
            xv[j] = eq[j] | mv[j];
        }
        // multi-precision add (eq & pv) + pv, carry chained upward
        let mut carry = 0u64;
        for j in 0..w {
            let a = eq[j] & pv[j];
            let (s1, c1) = a.overflowing_add(pv[j]);
            let (s2, c2) = s1.overflowing_add(carry);
            xh[j] = (s2 ^ pv[j]) | eq[j];
            carry = u64::from(c1 || c2);
        }
        for j in 0..w {
            ph[j] = mv[j] | !(xh[j] | pv[j]);
            mh[j] = pv[j] & xh[j];
        }
        // score from the last word's top valid bit (row m − 1)
        let top_bit = 1u64 << ((m - 1) % 64);
        if ph[w - 1] & top_bit != 0 {
            d += 1;
        }
        if mh[w - 1] & top_bit != 0 {
            d -= 1;
        }
        // shift ph/mh left by one with cross-word carries
        let mut ph_in = u64::from(!free_start);
        let mut mh_in = 0u64;
        for j in 0..w {
            let ph_out = ph[j] >> 63;
            let mh_out = mh[j] >> 63;
            let phs = (ph[j] << 1) | ph_in;
            let mhs = (mh[j] << 1) | mh_in;
            pv[j] = mhs | !(xv[j] | phs);
            mv[j] = phs & xv[j];
            ph_in = ph_out;
            mh_in = mh_out;
        }
        pv[w - 1] &= last_mask;
        mv[w - 1] &= last_mask;
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// O(mn) reference DP for `find_leq` semantics.
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
    fn basics() {
        assert_eq!(dist(b"kitten", b"sitting"), 3);
        assert_eq!(dist(b"", b"abc"), 3);
        assert_eq!(dist(b"abc", b""), 3);
        assert_eq!(dist(b"same", b"same"), 0);
    }

    #[test]
    fn agrees_with_single_word() {
        // m <= 64 must match editdist exactly — same automaton,
        // different implementation
        let mut rng = SplitMix64::new(31);
        for _ in 0..300 {
            let la = (rng.below(60) + 1) as usize;
            let lb = (rng.below(60) + 1) as usize;
            let a: Vec<u8> = (0..la).map(|_| b'a' + rng.below(6) as u8).collect();
            let b: Vec<u8> = (0..lb).map(|_| b'a' + rng.below(6) as u8).collect();
            assert_eq!(
                dist(&a, &b),
                crate::diff::levenshtein(&a, &b),
                "a={:?} b={:?}",
                a,
                b
            );
        }
    }

    #[test]
    fn long_pattern_oracle() {
        let mut rng = SplitMix64::new(32);
        for _ in 0..200 {
            let la = (rng.below(140) + 60) as usize; // forces 2+ words
            let lb = (rng.below(140) + 1) as usize;
            let a: Vec<u8> = (0..la).map(|_| b'a' + rng.below(4) as u8).collect();
            let b: Vec<u8> = (0..lb).map(|_| b'a' + rng.below(4) as u8).collect();
            assert_eq!(dist(&a, &b), crate::diff::levenshtein(&a, &b));
        }
    }

    #[test]
    fn find_leq_oracle() {
        let mut rng = SplitMix64::new(33);
        for _ in 0..120 {
            let lm = (rng.below(90) + 40) as usize;
            let ln = (rng.below(120) + 1) as usize;
            let pat: Vec<u8> = (0..lm).map(|_| b'a' + rng.below(4) as u8).collect();
            let text: Vec<u8> = (0..ln).map(|_| b'a' + rng.below(4) as u8).collect();
            let k = rng.below(8);
            assert_eq!(find_leq(&pat, &text, k), dp_find_leq(&pat, &text, k));
        }
    }

    #[test]
    fn word_boundary_stress() {
        // patterns exactly at 64/128/129 bytes exercise the
        // cross-word carry and last-word mask paths
        for &m in &[63usize, 64, 65, 127, 128, 129, 192] {
            let mut rng = SplitMix64::new(m as u64);
            let a: Vec<u8> = (0..m).map(|_| b'a' + rng.below(3) as u8).collect();
            let b: Vec<u8> = (0..m).map(|_| b'a' + rng.below(3) as u8).collect();
            assert_eq!(dist(&a, &b), crate::diff::levenshtein(&a, &b));
        }
    }
}

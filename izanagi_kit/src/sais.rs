//! SA-IS — induced-sorting suffix array construction in `O(n)`
//! for byte alphabets, where `suffix::SuffixArray` uses the
//! `O(n log n)` prefix-doubling scheme. The algorithm buckets
//! suffixes by first character, classifies each position as
//! S-type or L-type, sorts the LMS (leftmost-S) substrings by
//! induced passes, names them into a reduced string, and recurses
//! once — then induces the full order back out.
//!
//! The public surface is deliberately the same shape as
//! `SuffixArray::new`: `sais(s) -> Vec<usize>` where `sa[i]` is
//! the start index of the `i`-th smallest suffix. Bytes are
//! widened to `u32` in `1..=256` and a sentinel `0` appended
//! internally, so any `&[u8]` works.
//!
//! ```
//! use izanagi_kit::sais::sais;
//! assert_eq!(sais(b"mississippi"), vec![10, 7, 4, 1, 0, 9, 8, 6, 3, 5, 2]);
//! ```
//!
//! References: Nong (2013), "SA-IS: Suffix Array Construction
//! in O(N) Time"; Ge Nong's reference implementation; the
//! `divsufsort` source for bucket/scan details.

/// Suffix array of `s` via SA-IS. `sa[i]` = start index of the
/// `i`-th smallest suffix of `s`. `O(n)` time, `O(n)` words.
pub fn sais(s: &[u8]) -> Vec<usize> {
    let n = s.len();
    if n == 0 {
        return Vec::new();
    }
    // widen bytes to 1..=256, append sentinel 0
    let mut t: Vec<u32> = s.iter().map(|&b| b as u32 + 1).collect();
    t.push(0);
    let sa = sais_rec(&t, 257);
    // drop the sentinel entry (position n) from the front
    sa.into_iter().filter(|&i| i != n).collect()
}

/// Core: SA of `s` ending in the implicit sentinel (last symbol
/// must be the unique minimum). Symbols in `0..alpha`.
fn sais_rec(s: &[u32], alpha: usize) -> Vec<usize> {
    let n = s.len();
    if n == 1 {
        return vec![0];
    }
    // S[i] = true iff suffix i is S-type (s[i..] < s[i+1..])
    // sentinel is S-type; scan right to left
    let mut is_s = vec![false; n];
    is_s[n - 1] = true;
    for i in (0..n - 1).rev() {
        is_s[i] = s[i] < s[i + 1] || (s[i] == s[i + 1] && is_s[i + 1]);
    }
    // LMS positions: S-type whose predecessor is L-type
    let is_lms = |i: usize| i > 0 && is_s[i] && !is_s[i - 1];

    // bucket boundaries: counts then prefix sums
    let mut counts = vec![0usize; alpha];
    for &c in s {
        counts[c as usize] += 1;
    }
    let mut heads = vec![0usize; alpha];
    let mut tails = vec![0usize; alpha];
    let mut acc2 = 0usize;
    for c in 0..alpha {
        heads[c] = acc2;
        acc2 += counts[c];
        tails[c] = acc2;
    }

    let mut sa = vec![!0usize; n];

    // place LMS at bucket tails (in order), then induce
    let lms_order: Vec<usize> = (0..n).filter(|&i| is_lms(i)).collect();
    for &i in lms_order.iter().rev() {
        let c = s[i] as usize;
        tails[c] -= 1;
        sa[tails[c]] = i;
    }
    induce(&mut sa, s, &is_s, &heads, &counts);

    if lms_order.is_empty() {
        return sa;
    }

    // name LMS substrings in sorted order
    let sorted_lms: Vec<usize> = sa.iter().copied().filter(|&i| is_lms(i)).collect();
    let mut name = vec![0usize; n];
    let mut names = 0usize;
    name[sorted_lms[0]] = 0;
    for w in 1..sorted_lms.len() {
        let (a, b2) = (sorted_lms[w - 1], sorted_lms[w]);
        if !same_lms(s, &is_s, a, b2) {
            names += 1;
        }
        name[b2] = names;
    }
    // reduced string: names in original LMS position order
    let s1: Vec<u32> = lms_order.iter().map(|&i| name[i] as u32).collect();
    let alpha1 = names + 1;
    let sa1 = if alpha1 == s1.len() {
        // names unique — direct order
        let mut sa1 = vec![0usize; s1.len()];
        for (i, &nm) in s1.iter().enumerate() {
            sa1[nm as usize] = i;
        }
        sa1
    } else {
        sais_rec(&s1, alpha1)
    };

    // final induce: order LMS positions by sa1, place at tails,
    // then induce L and S passes
    let mut sa = vec![!0usize; n];
    let mut tails2 = vec![0usize; alpha];
    let mut acc3 = 0usize;
    for c in 0..alpha {
        acc3 += counts[c];
        tails2[c] = acc3;
    }
    for &j in sa1.iter().rev() {
        let i = lms_order[j];
        let c = s[i] as usize;
        tails2[c] -= 1;
        sa[tails2[c]] = i;
    }
    induce(&mut sa, s, &is_s, &heads, &counts);
    sa
}

/// Are the LMS substrings starting at `a` and `b` identical?
/// LMS substring = chars up to and including the next LMS
/// position (or end of string).
fn same_lms(s: &[u32], is_s: &[bool], a: usize, b: usize) -> bool {
    if a == b {
        return true;
    }
    let is_lms = |i: usize| i > 0 && i < s.len() && is_s[i] && !is_s[i - 1];
    let mut i = 0usize;
    loop {
        if a + i >= s.len() || b + i >= s.len() {
            return false;
        }
        let (ca, cb) = (s[a + i], s[b + i]);
        let (la, lb) = (is_lms(a + i), is_lms(b + i));
        if ca != cb || la != lb {
            return false;
        }
        if i > 0 && la {
            return true; // both hit their next LMS together
        }
        i += 1;
    }
}

/// The two induced passes: L-type left-to-right into bucket
/// heads, S-type right-to-left into bucket tails rebuilt from
/// `counts`.
fn induce(sa: &mut [usize], s: &[u32], is_s: &[bool], heads: &[usize], counts: &[usize]) {
    let n = s.len();
    let alpha = heads.len();
    // L pass: for each sa[j]=p with p>0 and p-1 L-type, put p-1
    // at the head of its bucket, advancing the head
    let mut h = heads.to_vec();
    for j in 0..n {
        let p = sa[j];
        if p != !0 && p > 0 {
            let q = p - 1;
            if !is_s[q] {
                let c = s[q] as usize;
                sa[h[c]] = q;
                h[c] += 1;
            }
        }
    }
    // S pass: right-to-left into tails
    let mut t = vec![0usize; alpha];
    let mut acc = 0usize;
    for c in 0..alpha {
        acc += counts[c];
        t[c] = acc;
    }
    for j in (0..n).rev() {
        let p = sa[j];
        if p != !0 && p > 0 {
            let q = p - 1;
            if is_s[q] {
                let c = s[q] as usize;
                t[c] -= 1;
                sa[t[c]] = q;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn naive(s: &[u8]) -> Vec<usize> {
        let mut v: Vec<usize> = (0..s.len()).collect();
        v.sort_by(|&a, &b| s[a..].cmp(&s[b..]));
        v
    }

    #[test]
    fn known() {
        assert_eq!(sais(b""), Vec::<usize>::new());
        assert_eq!(sais(b"a"), vec![0]);
        assert_eq!(sais(b"mississippi"), vec![10, 7, 4, 1, 0, 9, 8, 6, 3, 5, 2]);
        assert_eq!(sais(b"banana"), vec![5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn matches_naive_oracle() {
        let mut r = SplitMix64::new(0x5A15_A515);
        for _ in 0..400 {
            let n = 1 + r.below(60) as usize;
            let alpha = 1 + r.below(5);
            let s: Vec<u8> = (0..n).map(|_| (r.below(alpha)) as u8).collect();
            assert_eq!(sais(&s), naive(&s), "s={s:?}");
        }
        // worst shapes: constant, alternating, all-distinct
        for n in 1..80usize {
            let s = vec![7u8; n];
            assert_eq!(sais(&s), naive(&s), "constant n={n}");
            let s: Vec<u8> = (0..n).map(|i| (i % 3) as u8).collect();
            assert_eq!(sais(&s), naive(&s), "periodic n={n}");
            let s: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            assert_eq!(sais(&s), naive(&s));
        }
    }

    #[test]
    fn full_byte_range() {
        let mut r = SplitMix64::new(1);
        for _ in 0..100 {
            let n = 1 + r.below(50) as usize;
            let s: Vec<u8> = (0..n).map(|_| r.below(256) as u8).collect();
            assert_eq!(sais(&s), naive(&s));
        }
    }
}

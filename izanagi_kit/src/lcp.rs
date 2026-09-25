//! Kasai LCP array — the suffix array's companion: `lcp[i]` = the
//! longest common prefix of the suffixes at `sa[i−1]` and `sa[i]`
//! (`lcp[0] = 0`). `O(n)` via the rank-array trick: walking the text
//! in suffix order means the previous LCP can only shrink by one.
//!
//! Turns the `sais` module's sorted-suffix index into something you
//! can actually query: longest repeated substring, distinct-substring
//! count, and longest common substring between two texts all read
//! straight off the array.
//!
//! ```
//! use izanagi_kit::lcp::kasai;
//! use izanagi_kit::sais::sais;
//!
//! let lcp = kasai(b"banana", &sais(b"banana"));
//! // "ana" is the longest repeat (appears twice).
//! assert_eq!(lcp.iter().copied().max(), Some(3));
//! ```

/// `O(n)` Kasai construction: rank `r[pos]` = position of suffix `pos`
/// inside `sa`, then walk `pos = 0..n` and extend from
/// `lcp[r[pos]−1] − 1`. Panics (via indexing) if `sa` is not a
/// permutation of `0..n`.
pub fn kasai(s: &[u8], sa: &[usize]) -> Vec<usize> {
    let n = s.len();
    let mut rank = vec![0usize; n];
    for (i, &p) in sa.iter().enumerate() {
        rank[p] = i;
    }
    let mut lcp = vec![0usize; sa.len()];
    let mut h = 0usize;
    for pos in 0..n {
        let r = rank[pos];
        if r == 0 {
            continue;
        }
        let other = sa[r - 1];
        while pos + h < n && other + h < n && s[pos + h] == s[other + h] {
            h += 1;
        }
        lcp[r] = h;
        h = h.saturating_sub(1);
    }
    lcp
}

/// Longest substring occurring at least twice — `(offset, length)`
/// of the first maximal occurrence, or `None` when nothing repeats.
/// Offset is a start position in `s`.
pub fn longest_repeated(s: &[u8], sa: &[usize]) -> Option<(usize, usize)> {
    let lcp = kasai(s, sa);
    let (i, len) = lcp
        .iter()
        .enumerate()
        .max_by_key(|(_, &l)| l)
        .map(|(i, l)| (i, *l))?;
    if len == 0 {
        return None;
    }
    // Both sa[i] and sa[i−1] witness the repeat; report sa[i].
    Some((sa[i], len))
}

/// Count of distinct non-empty substrings: `n(n+1)/2 − Σ lcp`
/// (each suffix contributes `n − sa[i]` new substrings minus the
/// overlap already counted). Exact `usize` arithmetic — the classic
/// use of the LCP array.
pub fn distinct_substrings(s: &[u8], sa: &[usize]) -> usize {
    let n = s.len();
    let total = n * (n + 1) / 2;
    let shared: usize = kasai(s, sa).iter().sum();
    total - shared
}

/// Longest common substring of `a` and `b`: `(len, pos_in_a, pos_in_b)`
/// via the standard trick — `a + sep + b` with a separator absent
/// from both, then the maximal LCP whose adjacent suffixes straddle
/// the boundary. `None` if nothing matches.
pub fn longest_common(a: &[u8], b: &[u8]) -> Option<(usize, usize, usize)> {
    // Pick a byte absent from both inputs as the separator.
    let sep = (0u16..=255)
        .map(|c| c as u8)
        .find(|c| !a.contains(c) && !b.contains(c))?;
    let mut cat = Vec::with_capacity(a.len() + 1 + b.len());
    cat.extend_from_slice(a);
    cat.push(sep);
    cat.extend_from_slice(b);
    let sa = crate::sais::sais(&cat);
    let lcp = kasai(&cat, &sa);
    let boundary = a.len();
    let side = |pos: usize| pos > boundary; // false → in a
    let mut best: Option<(usize, usize, usize)> = None;
    for i in 1..sa.len() {
        if lcp[i] == 0 || side(sa[i]) == side(sa[i - 1]) {
            continue;
        }
        let len = lcp[i];
        let in_a = if side(sa[i]) { sa[i - 1] } else { sa[i] };
        let in_b = if side(sa[i]) { sa[i] } else { sa[i - 1] };
        if best.map_or(true, |(l, _, _)| len > l) {
            best = Some((len, in_a, in_b - boundary - 1));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sais::sais;

    /// O(n²) oracle: direct pairwise comparison of adjacent suffixes.
    fn oracle(s: &[u8], sa: &[usize]) -> Vec<usize> {
        sa.iter()
            .enumerate()
            .map(|(i, &p)| {
                if i == 0 {
                    0
                } else {
                    let q = sa[i - 1];
                    let mut h = 0;
                    while p + h < s.len() && q + h < s.len() && s[p + h] == s[q + h] {
                        h += 1;
                    }
                    h
                }
            })
            .collect()
    }

    #[test]
    fn matches_brute_oracle() {
        let mut g = crate::rng::SplitMix64::new(0x1CD);
        for _case in 0..40 {
            let n = g.range(0, 30) as usize;
            let s: Vec<u8> = (0..n).map(|_| b"ab"[g.below(2) as usize]).collect();
            let sa = sais(&s);
            assert_eq!(kasai(&s, &sa), oracle(&s, &sa), "s={s:?}");
        }
        assert_eq!(kasai(b"", &sais(b"")), Vec::<usize>::new());
        assert_eq!(
            kasai(b"aaaa", &sais(b"aaaa")),
            oracle(b"aaaa", &sais(b"aaaa"))
        );
    }

    #[test]
    fn longest_repeated_and_distinct_substrings() {
        let (off, len) = longest_repeated(b"banana", &sais(b"banana")).unwrap();
        assert_eq!(len, 3);
        assert_eq!(&b"banana"[off..off + len], b"ana");
        // "aaaa": repeats of length 3.
        assert_eq!(longest_repeated(b"aaaa", &sais(b"aaaa")), Some((0, 3)));
        // Distinct substrings of "ababa": {a,ab,aba,abab,ababa,b,ba,bab,baba}×…
        // brute: enumerate.
        let s = b"ababa";
        let mut set = std::collections::BTreeSet::new();
        for i in 0..s.len() {
            for j in i + 1..=s.len() {
                set.insert(&s[i..j]);
            }
        }
        assert_eq!(distinct_substrings(s, &sais(s)), set.len());
        assert_eq!(longest_repeated(b"xyz", &sais(b"xyz")), None);
    }

    #[test]
    fn longest_common_substring_between_texts() {
        let (len, pa, pb) = longest_common(b"xabcdefy", b"qqcdefrr").unwrap();
        assert_eq!(len, 4);
        assert_eq!(&b"xabcdefy"[pa..pa + len], b"cdef");
        assert_eq!(&b"qqcdefrr"[pb..pb + len], b"cdef");
        assert_eq!(longest_common(b"aaa", b"bbb"), None);
        // Whole-string containment.
        let (len, _, _) = longest_common(b"needle", b"the needle works").unwrap();
        assert_eq!(len, 6);
    }

    #[test]
    fn deterministic_twice() {
        let s = b"mississippi";
        let sa = sais(s);
        assert_eq!(kasai(s, &sa), kasai(s, &sa));
        assert_eq!(distinct_substrings(s, &sa), distinct_substrings(s, &sa));
    }
}

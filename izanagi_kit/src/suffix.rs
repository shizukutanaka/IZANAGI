//! Suffix array + Kasai LCP — `O(n log n)` construction by
//! prefix-doubling with rank compression, giving byte-substring
//! queries the kit's string layer lacked: `search` locates every
//! occurrence in `O(pat log n + hits)`, `lcp`/`longest_repeated`
//! expose shared-prefix structure, and `distinct_substrings` counts
//! unique substrings — useful for corpus analysis (what did `markov`
//! actually memorize) and content dedup.
//!
//! ```
//! use izanagi_kit::suffix::SuffixArray;
//! let sa = SuffixArray::new(b"mississippi");
//! assert_eq!(sa.search(b"ss"), vec![2, 5]);
//! assert_eq!(sa.longest_repeated(), Some(4)); // "issi"
//! ```

use std::cmp::Ordering;

/// Suffix array with LCP support. `sa[i]` is the start of the
/// `i`-th lexicographic suffix.
pub struct SuffixArray {
    data: Vec<u8>,
    sa: Vec<usize>,
    /// `lcp[i]` = longest common prefix of `sa[i]` and `sa[i-1]`
    /// (`lcp[0]` is 0).
    lcp: Vec<usize>,
}

impl SuffixArray {
    /// Build over `data` in `O(n log² n)` — prefix doubling with
    /// rank compression, so memory is `O(n)` words.
    pub fn new(data: &[u8]) -> Self {
        let n = data.len();
        if n == 0 {
            return Self {
                data: vec![],
                sa: vec![],
                lcp: vec![],
            };
        }
        // rank[i] = data[i] + 1 initially; sort (rank[i], rank[i+k]+1)
        // with 0 as the past-end sentinel.
        let mut rank: Vec<u32> = data.iter().map(|&b| b as u32 + 1).collect();
        let mut sa: Vec<usize> = (0..n).collect();
        let mut k = 1usize;
        loop {
            sa.sort_by_key(|&i| (rank[i], if i + k < n { rank[i + k] + 1 } else { 0 }));
            let mut nr = vec![0u32; n];
            let mut r = 0u32;
            nr[sa[0]] = 0;
            for w in sa.windows(2) {
                let (a, b) = (w[0], w[1]);
                let ka = (rank[a], if a + k < n { rank[a + k] + 1 } else { 0 });
                let kb = (rank[b], if b + k < n { rank[b + k] + 1 } else { 0 });
                if kb != ka {
                    r += 1;
                }
                nr[b] = r;
            }
            let distinct = r as usize + 1;
            rank = nr;
            if distinct == n || k >= n {
                break;
            }
            k *= 2;
        }
        let mut rnk = vec![0usize; n];
        for (i, &s) in sa.iter().enumerate() {
            rnk[s] = i;
        }
        // Kasai LCP between adjacent sa entries.
        let mut lcp = vec![0usize; n];
        let mut h = 0usize;
        for i in 0..n {
            let r = rnk[i];
            if r == 0 {
                continue;
            }
            let j = sa[r - 1];
            while i + h < n && j + h < n && data[i + h] == data[j + h] {
                h += 1;
            }
            lcp[r] = h;
            h = h.saturating_sub(1);
        }
        Self {
            data: data.to_vec(),
            sa,
            lcp,
        }
    }

    /// Input length.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the input was empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The suffix array itself.
    pub fn sa(&self) -> &[usize] {
        &self.sa
    }

    /// Raw LCP array (`lcp[i]` = LCP of `sa[i-1]`, `sa[i]`).
    pub fn lcp_array(&self) -> &[usize] {
        &self.lcp
    }

    /// LCP of the suffixes at `sa` positions `i` and `j`.
    /// `O(|i−j|)` — for hot loops range-min over [`Self::lcp_array`]
    /// via [`rmq::SparseTable`](crate::rmq).
    pub fn lcp(&self, i: usize, j: usize) -> usize {
        if i == j {
            return self.data.len() - self.sa[i];
        }
        let (a, b) = (i.min(j), i.max(j));
        self.lcp[a + 1..=b].iter().copied().min().unwrap_or(0)
    }

    /// All start positions of `pat`, ascending —
    /// `O(pat·log n + hits)`.
    pub fn search(&self, pat: &[u8]) -> Vec<usize> {
        if pat.is_empty() {
            return (0..=self.data.len()).collect();
        }
        let lo = self
            .sa
            .partition_point(|&s| self.cmp_prefix(s, pat) == Ordering::Less);
        let hi = self
            .sa
            .partition_point(|&s| self.cmp_prefix(s, pat) != Ordering::Greater);
        let mut out: Vec<usize> = self.sa[lo..hi].to_vec();
        out.sort_unstable();
        out
    }

    /// Compare `pat` against the prefix of suffix `s`: `Less` when the
    /// suffix is strictly below `pat`, `Equal` when `pat` is a prefix
    /// of the suffix, `Greater` otherwise.
    fn cmp_prefix(&self, s: usize, pat: &[u8]) -> Ordering {
        for (k, &c) in pat.iter().enumerate() {
            match self.data.get(s + k) {
                None => return Ordering::Less, // suffix ran out — shorter
                Some(&b) if b < c => return Ordering::Less,
                Some(&b) if b > c => return Ordering::Greater,
                _ => {}
            }
        }
        Ordering::Equal
    }

    /// Length of the longest substring occurring ≥2 times.
    pub fn longest_repeated(&self) -> Option<usize> {
        self.lcp.iter().copied().max().filter(|&v| v > 0)
    }

    /// Number of distinct non-empty substrings —
    /// `Σ (n − sa[i] − lcp[i])`.
    pub fn distinct_substrings(&self) -> usize {
        self.sa
            .iter()
            .zip(&self.lcp)
            .map(|(&s, &l)| self.data.len() - s - l)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn naive_sa(data: &[u8]) -> Vec<usize> {
        let mut v: Vec<usize> = (0..data.len()).collect();
        v.sort_by(|&a, &b| data[a..].cmp(&data[b..]));
        v
    }

    fn naive_lcp(data: &[u8], i: usize, j: usize) -> usize {
        let mut k = 0;
        while i + k < data.len() && j + k < data.len() && data[i + k] == data[j + k] {
            k += 1;
        }
        k
    }

    #[test]
    fn sa_matches_naive_sort() {
        let mut rng = SplitMix64::new(0x5A11);
        for _ in 0..200 {
            let n = rng.below(50) as usize;
            let data: Vec<u8> = (0..n).map(|_| (rng.next_u64() % 4) as u8 + b'a').collect();
            let sa = SuffixArray::new(&data);
            assert_eq!(sa.sa(), naive_sa(&data).as_slice());
        }
        assert_eq!(SuffixArray::new(b"").sa(), &[] as &[usize]);
        assert_eq!(SuffixArray::new(b"a").sa(), &[0]);
    }

    #[test]
    fn lcp_and_search_match_oracles() {
        let mut rng = SplitMix64::new(0x4C50);
        for _ in 0..200 {
            let n = rng.below(45) as usize;
            let data: Vec<u8> = (0..n).map(|_| (rng.next_u64() % 3) as u8 + b'x').collect();
            let sa = SuffixArray::new(&data);
            // lcp(i,j) between sa positions equals naive common prefix.
            if n == 0 {
                continue;
            }
            for _ in 0..20 {
                let i = rng.below(n as u32) as usize;
                let j = rng.below(n as u32) as usize;
                assert_eq!(
                    sa.lcp(i, j),
                    naive_lcp(&data, sa.sa[i], sa.sa[j]),
                    "sa[{i}]={} sa[{j}]={}",
                    sa.sa[i],
                    sa.sa[j]
                );
            }
            // search == naive find-all.
            let pat: Vec<u8> = (0..rng.below(5))
                .map(|_| (rng.next_u64() % 3) as u8 + b'x')
                .collect();
            let naive: Vec<usize> = (0..=n.saturating_sub(pat.len()))
                .filter(|&i| pat.is_empty() || data[i..].starts_with(&pat))
                .collect();
            assert_eq!(sa.search(&pat), naive);
            // distinct_substrings == BTreeSet count.
            let mut set = BTreeSet::new();
            for i in 0..n {
                for j in i + 1..=n {
                    set.insert(&data[i..j]);
                }
            }
            assert_eq!(sa.distinct_substrings(), set.len());
        }
    }

    #[test]
    fn known_answers() {
        let sa = SuffixArray::new(b"banana");
        assert_eq!(sa.sa(), &[5, 3, 1, 0, 4, 2]);
        assert_eq!(sa.lcp_array(), &[0, 1, 3, 0, 0, 2]);
        assert_eq!(sa.search(b"ana"), vec![1, 3]);
        assert_eq!(sa.longest_repeated(), Some(3)); // "ana"
        assert_eq!(sa.distinct_substrings(), 15);
        assert_eq!(sa.lcp(1, 2), 3);
    }
}

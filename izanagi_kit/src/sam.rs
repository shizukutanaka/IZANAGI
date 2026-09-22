//! Suffix automaton — the smallest deterministic automaton recognizing
//! every substring of a fixed text.
//!
//! Built online in `O(n)` (amortized) with `BTreeMap` transitions so the
//! structure, and every answer derived from it, is a pure function of
//! the input bytes. Complements [`crate::suffix`]'s suffix array: the SA
//! answers positional queries, the SAM answers existential and counting
//! queries and merges equal endpos-classes automatically.
//!
//! ```
//! use izanagi_kit::sam::Sam;
//! let sam = Sam::new(b"ababc");
//! assert!(sam.contains(b"bab"));
//! assert!(!sam.contains(b"baab"));
//! assert_eq!(sam.occurrences(b"ab"), 2);
//! assert_eq!(sam.longest_common(b"abcba"), 3); // "abc"
//! assert_eq!(sam.distinct_substrings(), 12);
//! ```

use std::collections::BTreeMap;

#[derive(Clone, Default)]
struct State {
    /// Longest string in this endpos class.
    len: u32,
    /// Suffix link (`u32::MAX` only for the root before it is set).
    link: i64,
    /// Transitions out, kept sorted so walks are deterministic.
    next: BTreeMap<u8, u32>,
    /// `|endpos|` of the class — filled in by [`Sam::finish`].
    occ: u64,
}

/// Suffix automaton over a byte string.
pub struct Sam {
    states: Vec<State>,
    last: u32,
    n: u32,
}

impl Sam {
    /// Builds the automaton for `data`. `O(n)` amortized.
    pub fn new(data: &[u8]) -> Self {
        let mut sam = Sam {
            states: vec![State {
                len: 0,
                link: -1,
                next: BTreeMap::new(),
                occ: 0,
            }],
            last: 0,
            n: data.len() as u32,
        };
        for &c in data {
            sam.extend(c);
        }
        sam.finish();
        sam
    }

    /// Number of states, `n + 1 <= states <= 2n - 1` for `n >= 2`.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Length of the text the automaton was built over.
    pub fn len(&self) -> usize {
        self.n as usize
    }

    /// Whether the text is empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Is `pat` a substring of the text?
    pub fn contains(&self, pat: &[u8]) -> bool {
        let mut v = 0u32;
        for &c in pat {
            match self.states[v as usize].next.get(&c) {
                Some(&u) => v = u,
                None => return false,
            }
        }
        true
    }

    /// Number of (possibly overlapping) occurrences of `pat` —
    /// the size of its endpos equivalence class.
    pub fn occurrences(&self, pat: &[u8]) -> u64 {
        let mut v = 0u32;
        for &c in pat {
            match self.states[v as usize].next.get(&c) {
                Some(&u) => v = u,
                None => return 0,
            }
        }
        self.states[v as usize].occ
    }

    /// Length of the longest substring of `other` that occurs in the
    /// text — the classic SAM LCS walk.
    pub fn longest_common(&self, other: &[u8]) -> usize {
        let mut v = 0u32;
        let mut l = 0u32;
        let mut best = 0u32;
        for &c in other {
            // Follow transitions, shrinking through suffix links.
            while v != 0 && !self.states[v as usize].next.contains_key(&c) {
                v = self.states[v as usize].link as u32;
                l = self.states[v as usize].len;
            }
            match self.states[v as usize].next.get(&c) {
                Some(&u) => {
                    v = u;
                    l += 1;
                }
                None => {
                    v = 0;
                    l = 0;
                }
            }
            best = best.max(l);
        }
        best as usize
    }

    /// Number of distinct nonempty substrings —
    /// `sum over states of len[v] - len[link[v]]`.
    pub fn distinct_substrings(&self) -> u64 {
        let mut total = 0u64;
        for s in self.states.iter().skip(1) {
            let parent_len = if s.link >= 0 {
                self.states[s.link as usize].len
            } else {
                0
            };
            total += (s.len - parent_len) as u64;
        }
        total
    }

    /// Length of the longest repeated substring —
    /// `max len[v]` over states with `occ >= 2`.
    pub fn longest_repeated(&self) -> usize {
        self.states
            .iter()
            .filter(|s| s.occ >= 2)
            .map(|s| s.len)
            .max()
            .unwrap_or(0) as usize
    }

    fn extend(&mut self, c: u8) {
        let cur = self.states.len() as u32;
        self.states.push(State {
            len: self.states[self.last as usize].len + 1,
            link: 0,
            next: BTreeMap::new(),
            occ: 1, // last-of-prefix states each own one new endpos
        });
        let mut p = self.last;
        // Walk suffix links until a state already has a c-transition;
        // every visited state gains the transition to cur.
        let mut hit = None;
        loop {
            if let Some(&t) = self.states[p as usize].next.get(&c) {
                hit = Some(t);
                break;
            }
            self.states[p as usize].next.insert(c, cur);
            if self.states[p as usize].link < 0 {
                break; // inserted on the whole suffix chain up to the root
            }
            p = self.states[p as usize].link as u32;
        }
        match hit {
            None => self.states[cur as usize].link = 0,
            Some(q) if self.states[p as usize].len + 1 == self.states[q as usize].len => {
                self.states[cur as usize].link = q as i64;
            }
            Some(q) => {
                // Clone q, shorten it, redirect suffix links through the clone.
                let clone = self.states.len() as u32;
                let mut cl = self.states[q as usize].clone();
                cl.len = self.states[p as usize].len + 1;
                cl.occ = 0; // clones are never last-of-prefix
                self.states.push(cl);
                loop {
                    if self.states[p as usize].next.get(&c) != Some(&q) {
                        break;
                    }
                    self.states[p as usize].next.insert(c, clone);
                    if self.states[p as usize].link < 0 {
                        break;
                    }
                    p = self.states[p as usize].link as u32;
                }
                self.states[q as usize].link = clone as i64;
                self.states[cur as usize].link = clone as i64;
            }
        }
        self.last = cur;
    }

    /// Propagate `occ` (each prefix's last state starts at 1) down the
    /// suffix-link tree in decreasing `len` order.
    fn finish(&mut self) {
        let mut order: Vec<u32> = (0..self.states.len() as u32).collect();
        order.sort_by_key(|&v| self.states[v as usize].len);
        for &v in order.iter().rev() {
            let l = self.states[v as usize].link;
            if l >= 0 {
                let add = self.states[v as usize].occ;
                self.states[l as usize].occ += add;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn oracle_contains(s: &[u8], pat: &[u8]) -> bool {
        !pat.is_empty() && s.windows(pat.len()).any(|w| w == pat)
    }

    fn oracle_occurrences(s: &[u8], pat: &[u8]) -> u64 {
        if pat.is_empty() || pat.len() > s.len() {
            return 0;
        }
        s.windows(pat.len()).filter(|&w| w == pat).count() as u64
    }

    fn oracle_lcs(a: &[u8], b: &[u8]) -> usize {
        for len in (1..=a.len().min(b.len())).rev() {
            let ok = a.windows(len).any(|w| b.windows(len).any(|v| v == w));
            if ok {
                return len;
            }
        }
        0
    }

    fn oracle_distinct(s: &[u8]) -> u64 {
        let mut set = BTreeSet::new();
        for i in 0..s.len() {
            for j in i + 1..=s.len() {
                set.insert(&s[i..j]);
            }
        }
        set.len() as u64
    }

    #[test]
    fn all_queries_match_brute_force() {
        let mut rng = SplitMix64::new(0x5A11);
        for _ in 0..200 {
            let n = (rng.below(30) + 1) as usize;
            let alpha = rng.below(3) + 1; // small alphabets → heavy overlap
            let s: Vec<u8> = (0..n).map(|_| b'a' + rng.below(alpha) as u8).collect();
            let sam = Sam::new(&s);
            // Patterns: random windows, random mutations, absent strings.
            for _ in 0..60 {
                let plen = (rng.below(8) + 1) as usize;
                let mut pat: Vec<u8> = (0..plen)
                    .map(|_| b'a' + rng.below(alpha + 1) as u8)
                    .collect();
                if n >= plen && rng.below(2) == 0 {
                    let i = rng.below((n - plen + 1) as u32) as usize;
                    pat.copy_from_slice(&s[i..i + plen]);
                }
                assert_eq!(sam.contains(&pat), oracle_contains(&s, &pat));
                assert_eq!(
                    sam.occurrences(&pat),
                    oracle_occurrences(&s, &pat),
                    "s={s:?} pat={pat:?}"
                );
            }
            assert_eq!(sam.distinct_substrings(), oracle_distinct(&s));
            let other: Vec<u8> = (0..20).map(|_| b'a' + rng.below(alpha + 1) as u8).collect();
            assert_eq!(sam.longest_common(&other), oracle_lcs(&s, &other));
        }
    }

    #[test]
    fn known_values() {
        let sam = Sam::new(b"ababa");
        assert_eq!(sam.distinct_substrings(), 9);
        assert_eq!(sam.occurrences(b"ab"), 2);
        assert_eq!(sam.occurrences(b"aba"), 2);
        assert_eq!(sam.occurrences(b"ba"), 2);
        assert_eq!(sam.longest_repeated(), 3);
        assert_eq!(sam.longest_common(b"xxabax"), 3);
        assert!(!sam.contains(b"abba"));
        // Empty / singleton edges.
        assert_eq!(Sam::new(b"").distinct_substrings(), 0);
        assert_eq!(Sam::new(b"z").distinct_substrings(), 1);
        assert_eq!(Sam::new(b"zz").distinct_substrings(), 2);
    }

    #[test]
    fn state_count_bound() {
        let mut rng = SplitMix64::new(0xCAFE);
        for _ in 0..50 {
            let n = (rng.below(60) + 2) as usize;
            let s: Vec<u8> = (0..n).map(|_| rng.below(4) as u8).collect();
            let sam = Sam::new(&s);
            assert!(sam.state_count() < 2 * n);
            assert!(sam.state_count() <= n + 1 + n); // loose sanity
        }
    }
}

//! Palindromic tree (eertree) — every distinct palindromic substring
//! of a text as a node of a `O(n)`-state structure.
//!
//! Two roots anchor the structure: the *imaginary* root (`len = -1`,
//! node 0) whose suffix link points to itself, and the *empty* root
//! (`len = 0`, node 1) whose suffix link points to the imaginary root.
//! Every other node is a real palindrome; the suffix link of a node
//! points to its longest proper palindromic suffix.
//!
//! `occ[v]` counts how many times palindrome `v` ends at some position
//! of the text — computed during construction by propagating each
//! node's end-position count down its suffix link in `len` order.
//!
//! ```
//! use izanagi_kit::eertree::Eertree;
//! let t = Eertree::new(b"ababa");
//! assert_eq!(t.distinct_palindromes(), 5); // a, b, aba, bab, ababa
//! assert_eq!(t.occurrences(b"a"), 3);
//! assert_eq!(t.occurrences(b"aba"), 2);
//! assert_eq!(t.longest_palindrome(), Some(b"ababa".as_slice()));
//! assert_eq!(t.palindromic_substring_count(), 9); // with multiplicity
//! ```
//!
//! Rubinchik & Shur (2015) — the palindromic-tree dual of `manacher`'s
//! longest-only scan and `sam`'s general substring compression.

use std::collections::BTreeMap;

#[derive(Clone)]
struct Node {
    len: i64,
    link: usize,
    next: BTreeMap<u8, usize>,
    occ: u64,
}

/// Eertree over a byte string.
pub struct Eertree {
    text: Vec<u8>,
    nodes: Vec<Node>,
    /// Node id of the longest palindrome overall (for `longest_palindrome`).
    max_node: usize,
}

const IMAG: usize = 0; // len = -1
const EMPTY: usize = 1; // len = 0

impl Eertree {
    /// Builds the palindromic tree over `text`. `O(n·(suffix-link walks))`
    /// — amortized linear for constant alphabets.
    pub fn new(text: &[u8]) -> Self {
        let mut t = Eertree {
            text: text.to_vec(),
            nodes: vec![
                Node {
                    len: -1,
                    link: IMAG,
                    next: BTreeMap::new(),
                    occ: 0,
                },
                Node {
                    len: 0,
                    link: IMAG,
                    next: BTreeMap::new(),
                    occ: 0,
                },
            ],
            max_node: EMPTY,
        };
        let mut cur = EMPTY; // longest palindromic suffix of processed prefix
        for i in 0..text.len() {
            let c = text[i];
            // Walk suffix links until the palindrome can be extended
            // by text[i-1-len] == c.
            let mut v = cur;
            loop {
                let l = t.nodes[v].len;
                let ok = l < 0 || (i as i64 - l > 0 && text[i - l as usize - 1] == c);
                if ok {
                    break;
                }
                v = t.nodes[v].link;
            }
            if let Some(&u) = t.nodes[v].next.get(&c) {
                cur = u;
            } else {
                // Create the node: len[v]+2 characters centered here.
                let id = t.nodes.len();
                t.nodes[v].next.insert(c, id);
                let len = t.nodes[v].len + 2;
                // Suffix link: for len == 1 the empty root; otherwise
                // walk v's suffix link for the same extension test.
                let link = if len == 1 {
                    EMPTY
                } else {
                    let mut w = t.nodes[v].link;
                    loop {
                        let l = t.nodes[w].len;
                        // The imaginary root always "extends" (text[i]==c),
                        // which terminates this walk — without it the loop
                        // would cycle on link[IMAG] == IMAG forever.
                        let ok = l < 0 || (i as i64 - l > 0 && text[i - l as usize - 1] == c);
                        if ok {
                            break;
                        }
                        w = t.nodes[w].link;
                    }
                    t.nodes[w].next[&c]
                };
                t.nodes.push(Node {
                    len,
                    link,
                    next: BTreeMap::new(),
                    occ: 0,
                });
                cur = id;
            }
            t.nodes[cur].occ += 1;
        }
        t.finish();
        t
    }

    /// Propagates `occ` from longer palindromes to their longest proper
    /// palindromic suffixes — after this, `occ[v]` is the total number
    /// of occurrences of palindrome `v` anywhere in the text.
    fn finish(&mut self) {
        let mut order: Vec<usize> = (2..self.nodes.len()).collect();
        order.sort_by_key(|&v| self.nodes[v].len);
        for &v in order.iter().rev() {
            let (occ, link) = (self.nodes[v].occ, self.nodes[v].link);
            if link >= 2 {
                self.nodes[link].occ += occ;
            }
        }
        // Longest palindrome = deepest node by len.
        let mut best = EMPTY;
        for (id, n) in self.nodes.iter().enumerate() {
            if id >= 2 && n.len > self.nodes[best].len {
                best = id;
            }
        }
        self.max_node = best;
    }

    /// Number of distinct palindromic substrings.
    pub fn distinct_palindromes(&self) -> u64 {
        self.nodes.len().saturating_sub(2) as u64
    }

    /// Total number of palindromic substrings counted with
    /// multiplicity — `Σ occ[v]` over real nodes.
    pub fn palindromic_substring_count(&self) -> u64 {
        self.nodes.iter().skip(2).map(|n| n.occ).sum()
    }

    /// Number of times `pat` occurs in the text. `0` when `pat` is not
    /// a palindrome of the text at all.
    pub fn occurrences(&self, pat: &[u8]) -> u64 {
        match self.node_of(pat) {
            Some(v) => self.nodes[v].occ,
            None => 0,
        }
    }

    /// The longest palindromic substring (first found wins ties —
    /// eertree visits extend in text order).
    pub fn longest_palindrome(&self) -> Option<&[u8]> {
        if self.nodes.len() <= 2 {
            return None;
        }
        let len = self.nodes[self.max_node].len as usize;
        // The node was created when its palindrome first appeared —
        // recover its ending position by walking occurrences? Cheaper:
        // re-derive from the node's own len: it occurs ending at the
        // position where this node is the current suffix; find the
        // first index where the palindrome appears by scanning.
        let n = self.text.len();
        // Scan all substrings of the right length for a palindrome —
        // O(n) positions, palindrome check is O(len). For auditing this
        // stays tiny; use `occurrences` for hot paths.
        for i in 0..=n.saturating_sub(len) {
            let s = &self.text[i..i + len];
            if s.iter().zip(s.iter().rev()).all(|(a, b)| a == b)
                && self.node_of(s) == Some(self.max_node)
            {
                return Some(&self.text[i..i + len]);
            }
        }
        None
    }

    /// Node id for the palindrome `pat`, if it occurs.
    fn node_of(&self, pat: &[u8]) -> Option<usize> {
        // Walk: center outwards — standard eertree matching follows
        // transitions half the pattern, but the simplest correct walk
        // is extension-based: descend from the appropriate root.
        let mut v = if pat.len() % 2 == 1 { IMAG } else { EMPTY };
        // For odd length start from IMAG consuming the middle char,
        // even length from EMPTY consuming pairs around the center.
        // Both cases: consume pat from its center outwards.
        // Each transition consumes ONE char (it adds that char to both
        // sides of the palindrome at once — palindromic symmetry means
        // the mirrored char needs no separate step).
        let m = pat.len();
        let half = m / 2;
        let order: Vec<u8> = if m % 2 == 1 {
            let mut o = Vec::with_capacity(half + 1);
            o.push(pat[half]); // center char extends the imaginary root
            for d in 1..=half {
                o.push(pat[half - d]);
            }
            o
        } else {
            (0..half).map(|d| pat[half - 1 - d]).collect()
        };
        for &c in &order {
            match self.nodes[v].next.get(&c) {
                Some(&u) => v = u,
                None => return None,
            }
        }
        // Validate: node len must equal pat len, and pat must be a
        // palindrome for the walk to represent it.
        if self.nodes[v].len as usize != m {
            return None;
        }
        if !pat.iter().zip(pat.iter().rev()).all(|(a, b)| a == b) {
            return None;
        }
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap as BM;
    use std::collections::BTreeSet;

    /// Oracle: multiset of all palindromic substrings.
    fn oracle_pals(text: &[u8]) -> BM<Vec<u8>, u64> {
        let mut m = BM::new();
        let n = text.len();
        for i in 0..n {
            for j in i + 1..=n {
                let s = &text[i..j];
                if s.iter().zip(s.iter().rev()).all(|(a, b)| a == b) {
                    *m.entry(s.to_vec()).or_insert(0) += 1;
                }
            }
        }
        m
    }

    #[test]
    fn all_queries_match_oracle() {
        let mut rng = SplitMix64::new(0xEE22);
        for _ in 0..200 {
            let n = (rng.below(30) + 1) as usize;
            let alph = rng.below(3) + 2;
            let text: Vec<u8> = (0..n).map(|_| rng.below(alph) as u8).collect();
            let t = Eertree::new(&text);
            let pals = oracle_pals(&text);
            assert_eq!(t.distinct_palindromes(), pals.len() as u64, "text={text:?}");
            assert_eq!(
                t.palindromic_substring_count(),
                pals.values().sum::<u64>(),
                "text={text:?}"
            );
            for (p, &want) in &pals {
                assert_eq!(t.occurrences(p), want, "pat={p:?} text={text:?}");
            }
            // Non-palindrome substrings report 0.
            for _ in 0..20 {
                let i = rng.below(n as u32) as usize;
                let j = (i as u32 + rng.below(n as u32 - i as u32)).min(n as u32) as usize;
                let s = &text[i..=j.min(n - 1).max(i)];
                if !s.iter().zip(s.iter().rev()).all(|(a, b)| a == b) {
                    assert_eq!(t.occurrences(s), 0);
                }
            }
            // Longest is a real palindrome of maximal length.
            let want_len = pals.keys().map(|p| p.len()).max().unwrap_or(0);
            let got = t.longest_palindrome().unwrap();
            assert_eq!(got.len(), want_len);
            assert!(got.iter().zip(got.iter().rev()).all(|(a, b)| a == b));
        }
    }

    #[test]
    fn known_cases() {
        let t = Eertree::new(b"");
        assert_eq!(t.distinct_palindromes(), 0);
        assert_eq!(t.palindromic_substring_count(), 0);
        assert_eq!(t.longest_palindrome(), None);

        let t = Eertree::new(b"aaaa");
        assert_eq!(t.distinct_palindromes(), 4); // a, aa, aaa, aaaa
        assert_eq!(t.occurrences(b"aa"), 3);
        assert_eq!(t.palindromic_substring_count(), 10);
        assert_eq!(t.longest_palindrome(), Some(b"aaaa".as_slice()));

        let t = Eertree::new(b"ab");
        assert_eq!(t.distinct_palindromes(), 2);
        assert_eq!(t.occurrences(b"ab"), 0);

        // Distinctness: same text, different derivations agree.
        let a = Eertree::new(b"abcba");
        let b = Eertree::new(b"abcba");
        assert_eq!(a.distinct_palindromes(), b.distinct_palindromes());
        let mut seen = BTreeSet::new();
        for p in [b"a".as_slice(), b"b", b"c", b"bcb", b"abcba"] {
            seen.insert(a.occurrences(p));
        }
        assert!(seen.contains(&1));
    }
}

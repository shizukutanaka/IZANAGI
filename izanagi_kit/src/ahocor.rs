//! Aho–Corasick multi-pattern string matching on `&[u8]`.
//!
//! Builds the classic fail-link automaton (Aho & Corasick, CACM 1975)
//! in `O(Σ|pattern|)`; a `scan` then reports every pattern occurrence —
//! including overlapping and suffix-contained ones — in `O(text + hits)`,
//! ordered by end position then pattern index (deterministic).
//!
//! Children are kept in a `BTreeMap` per node rather than a dense
//! `[usize; 256]` table: memory stays proportional to the trie's edge
//! count and the BFS order is content-defined either way. No hashing of
//! any kind reaches the output ordering — matches emit in scan order.
//!
//! ```
//! use izanagi_kit::ahocor::AhoCorasick;
//! let ac = AhoCorasick::new(&[b"he".as_slice(), b"she", b"his", b"hers"]);
//! assert_eq!(
//!     ac.scan(b"ushers"),
//!     vec![(4, 0), (4, 1), (6, 3)], // "he"@2-4, "she"@1-4, "hers"@2-6
//! );
//! ```

use std::collections::{BTreeMap, VecDeque};

#[derive(Default)]
struct Node {
    /// Transitions keyed by byte; iteration order is sorted → BFS parent
    /// discovery is deterministic.
    next: BTreeMap<u8, usize>,
    /// Longest proper suffix that is also a trie prefix.
    fail: usize,
    /// Pattern indices ending at this node (insertion order).
    out: Vec<usize>,
}

/// Aho–Corasick automaton over byte patterns.
pub struct AhoCorasick {
    nodes: Vec<Node>,
    /// For each node, the nearest ancestor (including itself) that is an
    /// output node — lets `scan` emit dict matches in one hop each.
    dict: Vec<Option<usize>>,
}

/// Root index in `nodes`.
const ROOT: usize = 0;

impl AhoCorasick {
    /// Build an automaton recognising every `patterns` entry. Duplicate
    /// patterns are kept (each index is reported separately); empty
    /// patterns are ignored — a zero-length match at every position is
    /// noise rather than signal.
    pub fn new(patterns: &[&[u8]]) -> Self {
        let mut nodes = vec![Node::default()];
        for (pi, pat) in patterns.iter().enumerate() {
            if pat.is_empty() {
                continue;
            }
            let mut v = ROOT;
            for &c in pat.iter() {
                v = match nodes[v].next.get(&c) {
                    Some(&u) => u,
                    None => {
                        let u = nodes.len();
                        nodes.push(Node::default());
                        nodes[v].next.insert(c, u);
                        u
                    }
                };
            }
            nodes[v].out.push(pi);
        }
        // BFS fail links. Depth increases along BFS order so a node's
        // fail is finalised before its children are processed.
        let mut dict = vec![None; nodes.len()];
        let mut q = VecDeque::new();
        let root_kids: Vec<usize> = nodes[ROOT].next.values().copied().collect();
        for u in root_kids {
            nodes[u].fail = ROOT;
            q.push_back(u);
        }
        dict[ROOT] = None;
        while let Some(v) = q.pop_front() {
            let kids: Vec<(u8, usize)> = nodes[v].next.iter().map(|(&c, &u)| (c, u)).collect();
            for (c, u) in kids {
                // Walk fail chain until a node has a `c` transition.
                let mut f = nodes[v].fail;
                loop {
                    if let Some(&w) = nodes[f].next.get(&c) {
                        nodes[u].fail = w;
                        break;
                    }
                    if f == ROOT {
                        nodes[u].fail = ROOT;
                        break;
                    }
                    f = nodes[f].fail;
                }
                q.push_back(u);
            }
            // dict suffix link: this node if it outputs, else the nearest
            // output node along its fail chain (already computed — BFS
            // processes nodes in increasing depth).
            dict[v] = if nodes[v].out.is_empty() {
                dict[nodes[v].fail]
            } else {
                Some(v)
            };
        }
        Self { nodes, dict }
    }

    /// All pattern occurrences in `text`, as `(end_pos, pattern_idx)`
    /// pairs where `end_pos` is the byte index just past the match.
    /// Sorted by `(end_pos, pattern_idx)` — the emission order of the
    /// dict-suffix walk is already content-defined.
    pub fn scan(&self, text: &[u8]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut v = ROOT;
        for (i, &c) in text.iter().enumerate() {
            loop {
                if let Some(&u) = self.nodes[v].next.get(&c) {
                    v = u;
                    break;
                }
                if v == ROOT {
                    break;
                }
                v = self.nodes[v].fail;
            }
            // Emit every dictionary word ending at this position by
            // walking dict links; `dict[fail[d]]` is the next output
            // ancestor above `d`. Sorted per position for canonical order.
            let base = out.len();
            let mut d = self.dict[v];
            while let Some(node) = d {
                for &pi in &self.nodes[node].out {
                    out.push((i + 1, pi));
                }
                d = self.dict[self.nodes[node].fail];
            }
            out[base..].sort();
        }
        out
    }

    /// Number of trie nodes (root included) — handy for diagnostics.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: brute-force scan — for each end position, each pattern.
    fn brute(text: &[u8], patterns: &[&[u8]]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for end in 1..=text.len() {
            for (pi, pat) in patterns.iter().enumerate() {
                let pl = pat.len();
                if pl <= end && text[end - pl..end] == **pat {
                    out.push((end, pi));
                }
            }
        }
        out
    }

    #[test]
    fn matches_brute_force_on_random_input() {
        let mut rng = SplitMix64::new(0xA0C0);
        for _ in 0..200 {
            let np = 1 + rng.below(6) as usize;
            let alphabet = b"abc";
            let pats: Vec<Vec<u8>> = (0..np)
                .map(|_| {
                    (0..1 + rng.below(6))
                        .map(|_| alphabet[rng.below(3) as usize])
                        .collect()
                })
                .collect();
            let pat_refs: Vec<&[u8]> = pats.iter().map(|p| p.as_slice()).collect();
            let text: Vec<u8> = (0..1 + rng.below(30))
                .map(|_| alphabet[rng.below(3) as usize])
                .collect();
            let ac = AhoCorasick::new(&pat_refs);
            assert_eq!(ac.scan(&text), brute(&text, &pat_refs), "text {text:?}");
        }
    }

    #[test]
    fn overlapping_and_suffix_patterns() {
        let ac = AhoCorasick::new(&[b"aa".as_slice()]);
        assert_eq!(ac.scan(b"aaaa"), vec![(2, 0), (3, 0), (4, 0)]);
        let ac2 = AhoCorasick::new(&[b"abc".as_slice(), b"bc", b"c"]);
        // "abc" ends at 3; "bc" ends at 3; "c" ends at 3 → sorted by pi.
        assert_eq!(ac2.scan(b"abc"), vec![(3, 0), (3, 1), (3, 2)]);
    }

    #[test]
    fn empty_and_duplicate_patterns() {
        let ac = AhoCorasick::new(&[b"".as_slice(), b"x".as_slice(), b"x".as_slice()]);
        assert_eq!(ac.scan(b"ax"), vec![(2, 1), (2, 2)]);
        assert_eq!(ac.scan(b""), vec![]);
    }
}

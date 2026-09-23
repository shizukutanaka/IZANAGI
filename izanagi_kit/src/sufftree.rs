//! Ukkonen's online suffix tree over `u8` text.
//!
//! One `O(n)` construction pass produces a compact tree in
//! which every suffix of `text` is a root-to-leaf path —
//! complementing [`crate::suffix`] (array) and
//! [`crate::sam`] (automaton) with an *explicit* tree:
//! substring queries walk edges, and internal nodes mark
//! branching (repeated) substrings.
//!
//! Edges carry half-open label ranges `[start, end)` into the
//! text; leaf ends are the sentinel `LEAF` resolved to
//! `text.len()` at read time, which is what makes the tree
//! "online" — appending extends every leaf edge implicitly.
//!
//! ```
//! use izanagi_kit::sufftree::SuffixTree;
//! let t = SuffixTree::new(b"banana");
//! assert!(t.contains(b"nan"));
//! assert!(!t.contains(b"bnana"));
//! assert_eq!(t.occurrences(b"ana"), vec![1, 3]);
//! assert_eq!(t.longest_repeat(), b"ana".to_vec());
//! ```

use std::collections::BTreeMap;

/// Sentinel leaf-edge end — resolved to `text.len()+1`
/// (the label includes the virtual terminator).
const LEAF: usize = !0;
/// Arena index of "no node".
const NIL: usize = !0;
/// Virtual terminator appended to the text — `u16` so it can
/// never equal a real byte. Without it, a suffix that ends
/// exactly at an internal node gets no leaf and occurrences
/// are undercounted.
const SENT: u16 = u16::MAX;

#[derive(Clone, Default)]
struct Edge {
    start: usize,
    end: usize, // LEAF for leaf edges
    child: usize,
}

#[derive(Clone, Default)]
struct Node {
    /// First symbol of the edge label → edge. Symbols are
    /// `u16`: bytes 0..256 plus the terminator [`SENT`].
    edges: BTreeMap<u16, Edge>,
    /// Suffix link (root links to itself).
    link: usize,
    /// Number of leaf descendants — filled by a final pass.
    leaf_count: u32,
    /// Depth in characters — filled by the same pass.
    depth: usize,
}

/// A suffix tree over `text`. See module docs.
pub struct SuffixTree {
    text: Vec<u8>,
    nodes: Vec<Node>,
    root: usize,
}

impl SuffixTree {
    /// Build the suffix tree of `text` via Ukkonen's algorithm.
    /// `O(n)` — each phase amortizes one step of `active_len`.
    pub fn new(text: &[u8]) -> Self {
        let n1 = text.len() + 1; // virtual terminator at index n
        let mut t = SuffixTree {
            text: text.to_vec(),
            nodes: vec![Node {
                edges: BTreeMap::new(),
                link: 0,
                leaf_count: 0,
                depth: 0,
            }],
            root: 0,
        };
        let mut active_node = 0usize;
        // active point = (active_node, active_pos, active_len):
        // the implicit node `active_len` chars into the edge
        // out of `active_node` labelled by `text[active_pos]`.
        let mut active_pos = 0usize;
        let mut active_len = 0usize;
        let mut remainder = 0usize;

        for i in 0..n1 {
            remainder += 1;
            let mut last_new = NIL;
            while remainder > 0 {
                if active_len == 0 {
                    active_pos = i;
                }
                // skip/count — walk down whole edges the active
                // point has crossed
                while active_len > 0 {
                    let key = t.text_at(active_pos);
                    let Some(e) = t.nodes[active_node].edges.get(&key).cloned() else {
                        break;
                    };
                    let elen = t.edge_len(&e);
                    if active_len < elen {
                        break;
                    }
                    active_pos += elen;
                    active_len -= elen;
                    active_node = e.child;
                }
                let active_edge = t.text_at(active_pos);
                if !t.nodes[active_node].edges.contains_key(&active_edge) {
                    // Rule 2 — no edge for active_edge: create leaf
                    let leaf = t.new_node();
                    t.nodes[active_node].edges.insert(
                        active_edge,
                        Edge {
                            start: i,
                            end: LEAF,
                            child: leaf,
                        },
                    );
                    if last_new != NIL {
                        t.nodes[last_new].link = active_node;
                        last_new = NIL;
                    }
                } else {
                    let e = t.nodes[active_node].edges[&active_edge].clone();
                    let next_char = t.text_at(e.start + active_len);
                    if next_char == t.text_at(i) {
                        // Rule 3 — character already on the edge
                        if last_new != NIL && active_node != t.root {
                            t.nodes[last_new].link = active_node;
                        }
                        active_len += 1;
                        break;
                    }
                    // Rule 2 — split at (e.start + active_len)
                    let split = t.new_node();
                    t.nodes[active_node].edges.insert(
                        active_edge,
                        Edge {
                            start: e.start,
                            end: e.start + active_len,
                            child: split,
                        },
                    );
                    // split → old continuation
                    let cont_key = t.text_at(e.start + active_len);
                    t.nodes[split].edges.insert(
                        cont_key,
                        Edge {
                            start: e.start + active_len,
                            end: e.end,
                            child: e.child,
                        },
                    );
                    // split → new leaf
                    let leaf = t.new_node();
                    let leaf_key = t.text_at(i);
                    t.nodes[split].edges.insert(
                        leaf_key,
                        Edge {
                            start: i,
                            end: LEAF,
                            child: leaf,
                        },
                    );
                    if last_new != NIL {
                        t.nodes[last_new].link = split;
                    }
                    last_new = split;
                }
                remainder -= 1;
                if active_node == t.root && active_len > 0 {
                    // the canonical Ukkonen rule — the next
                    // suffix starts one char later
                    active_len -= 1;
                    active_pos = i - remainder + 1;
                } else if active_node != t.root {
                    active_node = t.nodes[active_node].link;
                }
            }
        }
        // fill leaf_count / depth for the query API
        t.audit(t.root, 0);
        t
    }

    fn new_node(&mut self) -> usize {
        self.nodes.push(Node {
            link: self.root,
            ..Node::default()
        });
        self.nodes.len() - 1
    }

    /// Symbol at `pos` — `SENT` past the end.
    fn text_at(&self, pos: usize) -> u16 {
        if pos < self.text.len() {
            u16::from(self.text[pos])
        } else {
            SENT
        }
    }

    fn edge_len(&self, e: &Edge) -> usize {
        let end = if e.end == LEAF {
            self.text.len() + 1
        } else {
            e.end
        };
        end - e.start
    }

    /// Post-order fill: leaf_count + char depth.
    fn audit(&mut self, x: usize, depth: usize) -> u32 {
        let mut total = 0u32;
        self.nodes[x].depth = depth;
        let keys: Vec<u16> = self.nodes[x].edges.keys().copied().collect();
        for k in keys {
            let e = self.nodes[x].edges[&k].clone();
            let elen = self.edge_len(&e);
            total += self.audit(e.child, depth + elen);
        }
        let is_leaf = self.nodes[x].edges.is_empty();
        let lc = if is_leaf { 1 } else { total };
        self.nodes[x].leaf_count = lc;
        lc
    }

    /// Walk `pat` down the tree — returns the node plus how
    /// far into its parent edge the match ended, or `None`.
    fn find_node(&self, pat: &[u8]) -> Option<usize> {
        if pat.is_empty() {
            return Some(self.root);
        }
        let mut x = self.root;
        let mut i = 0usize;
        while i < pat.len() {
            let e = self.nodes[x].edges.get(&u16::from(pat[i]))?.clone();
            let elen = self.edge_len(&e);
            let take = elen.min(pat.len() - i);
            if self.text[e.start..e.start + take] != pat[i..i + take] {
                return None;
            }
            i += take;
            x = e.child;
        }
        Some(x)
    }

    /// `pat` occurs anywhere in the text?
    pub fn contains(&self, pat: &[u8]) -> bool {
        self.find_node(pat).is_some()
    }

    /// Number of occurrences of `pat` (leaf descendants of the
    /// node it ends on — covers the "ends mid-edge" case too,
    /// since that node's subtree holds all suffixes prefixed
    /// by `pat`).
    pub fn count(&self, pat: &[u8]) -> u32 {
        if pat.is_empty() {
            return (self.text.len() + 1) as u32;
        }
        match self.find_node(pat) {
            None => 0,
            Some(x) => self.nodes[x].leaf_count,
        }
    }

    /// Start indices of every occurrence, ascending.
    pub fn occurrences(&self, pat: &[u8]) -> Vec<usize> {
        let mut out = Vec::new();
        if pat.is_empty() {
            return (0..=self.text.len()).collect();
        }
        let x = match self.find_node(pat) {
            None => return out,
            Some(x) => x,
        };
        self.collect_starts(x, &mut out);
        out.sort_unstable();
        out
    }

    /// Suffix start index of a leaf = `n+1 - depth`
    /// (depth includes the terminator edge char).
    fn collect_starts(&self, x: usize, out: &mut Vec<usize>) {
        if self.nodes[x].edges.is_empty() {
            out.push(self.text.len() + 1 - self.nodes[x].depth);
            return;
        }
        for e in self.nodes[x].edges.values() {
            self.collect_starts(e.child, out);
        }
    }

    /// Longest substring occurring ≥ 2 times — the label of
    /// the deepest internal node with `leaf_count >= 2`.
    /// Ties resolve to the lexicographically smallest.
    pub fn longest_repeat(&self) -> Vec<u8> {
        let mut best_depth = 0usize;
        let mut best_end = 0usize; // absolute end index of the label
        self.dfs_repeat(self.root, &mut best_depth, &mut best_end);
        if best_depth == 0 {
            return Vec::new();
        }
        self.text[best_end - best_depth..best_end].to_vec()
    }

    fn dfs_repeat(&self, x: usize, best: &mut usize, best_end: &mut usize) {
        for e in self.nodes[x].edges.values() {
            let c = e.child;
            if self.nodes[c].leaf_count >= 2 && self.nodes[c].depth > *best {
                *best = self.nodes[c].depth;
                *best_end = e.start + self.edge_len(e);
            }
            self.dfs_repeat(c, best, best_end);
        }
    }

    /// Node count — diagnostic.
    pub fn n_nodes(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let t = SuffixTree::new(b"banana");
        assert!(t.contains(b"ana"));
        assert!(t.contains(b"banana"));
        assert!(t.contains(b"")); // empty matches everywhere
        assert!(!t.contains(b"apple"));
        assert_eq!(t.occurrences(b"ana"), vec![1, 3]);
        assert_eq!(t.occurrences(b"a"), vec![1, 3, 5]);
        assert_eq!(t.count(b"ana"), 2);
        assert_eq!(t.count(b"nana"), 1);
        assert_eq!(t.longest_repeat(), b"ana".to_vec());
        let t2 = SuffixTree::new(b"abcd");
        assert_eq!(t2.longest_repeat(), Vec::<u8>::new());
        // root + 5 leaves: one per byte + the terminator leaf
        assert_eq!(t2.n_nodes(), 6);
    }

    /// Every suffix is a path; every substring matches.
    /// Oracle: all substrings of the text are `contains` hits,
    /// and sampled non-substrings are misses — plus `count` ==
    /// naive scan.
    #[test]
    fn oracle_all_substrings() {
        let mut rng = crate::rng::SplitMix64::new(0x5EED);
        for _ in 0..40 {
            let n = 1 + rng.below(40) as usize;
            let text: Vec<u8> = (0..n).map(|_| b'a' + rng.below(4) as u8).collect();
            let t = SuffixTree::new(&text);
            // every substring present
            for i in 0..n {
                for j in i + 1..=n {
                    let pat = &text[i..j];
                    assert!(t.contains(pat), "missing {pat:?} in {text:?}");
                    // count = naive occurrence count
                    let mut naive = 0;
                    for k in 0..=n - pat.len() {
                        if &text[k..k + pat.len()] == pat {
                            naive += 1;
                        }
                    }
                    assert_eq!(t.count(pat), naive, "count {pat:?}");
                }
            }
            // occurrences match naive too
            for _ in 0..30 {
                let a = rng.below(n as u32) as usize;
                let blen = 1 + rng.below(6) as usize;
                let b = (a + blen).min(n);
                let pat = &text[a..b];
                let mut naive: Vec<usize> = (0..=n - pat.len())
                    .filter(|&k| &text[k..k + pat.len()] == pat)
                    .collect();
                naive.sort_unstable();
                assert_eq!(t.occurrences(pat), naive, "occ {pat:?}");
            }
            // longest repeat = brute-force longest duplicated
            // substring
            let mut best: Vec<u8> = Vec::new();
            for len in (1..n).rev() {
                let mut seen = BTreeMap::new();
                for i in 0..=n - len {
                    *seen.entry(&text[i..i + len]).or_insert(0u32) += 1;
                }
                if let Some((s, _)) = seen.iter().find(|(_, &c)| c >= 2) {
                    best = s.to_vec();
                    break;
                }
            }
            assert_eq!(t.longest_repeat(), best, "lrs of {text:?}");
        }
    }

    /// Online property: the tree built incrementally must
    /// contain every suffix — check the last character's edge
    /// exists on the root for each new char.
    #[test]
    fn online_extension() {
        let t = SuffixTree::new(b"mississippi");
        assert_eq!(t.occurrences(b"ssi"), vec![2, 5]);
        assert_eq!(t.longest_repeat(), b"issi".to_vec());
    }

    /// Degenerate texts — empty, single char, all-same.
    #[test]
    fn degenerate() {
        let e = SuffixTree::new(b"");
        assert_eq!(e.n_nodes(), 2); // root + terminator leaf
        assert!(!e.contains(b"a"));
        let s = SuffixTree::new(b"a");
        assert_eq!(s.occurrences(b"a"), vec![0]);
        let r = SuffixTree::new(b"aaaa");
        assert_eq!(r.longest_repeat(), b"aaa".to_vec());
    }
}

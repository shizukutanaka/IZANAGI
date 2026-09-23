//! BK-tree — Burkhard & Keller's metric-space index (1973). Each
//! node holds one string key; the child edge labelled `d` reaches
//! the subtree of keys whose Levenshtein distance from the parent's
//! key is exactly `d`. Queries prune whole subtrees with the
//! triangle inequality: a key within radius `r` of `q` can only live
//! under edges in `[d(q, node) − r, d(q, node) + r]` — turning fuzzy
//! "did we see something like this before" lookups (typo-tolerant
//! names, replay-input dedup) into a handful of distance
//! evaluations instead of a full scan.
//!
//! The index is a pure function of the insertion sequence: same
//! keys in the same order, same tree, same answers — and answers are
//! returned sorted by `(distance, key)` so callers get a canonical
//! ordering regardless of probe order.
//!
//! ```
//! use izanagi_kit::bktree::BkTree;
//!
//! let mut t = BkTree::new();
//! for w in ["cat", "cut", "bat", "carta", "dog"] {
//!     t.insert(w.as_bytes());
//! }
//! assert!(t.contains(b"cat"));
//! assert_eq!(t.nearest(b"car"), Some((1, 0))); // "cat"
//! assert_eq!(t.within(b"cat", 1).len(), 3); // cat, cut, bat
//! ```
//!
//! References: Burkhard & Keller, "Some approaches to best-match
//! file searching" (1973); Surjuse & Jichkar's BK-tree survey.

use crate::diff::levenshtein;
use std::collections::BTreeMap;

/// Metric index over byte strings keyed by Levenshtein distance.
#[derive(Default)]
pub struct BkTree {
    keys: Vec<Vec<u8>>,
    /// `children[i][d]` = node index whose distance to `i`'s key
    /// is exactly `d`.
    children: Vec<BTreeMap<u32, usize>>,
    root: Option<usize>,
}

impl BkTree {
    /// Empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored keys.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// `true` when empty.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Key stored at index `i` (insertion order).
    pub fn key(&self, i: usize) -> Option<&[u8]> {
        self.keys.get(i).map(|v| v.as_slice())
    }

    /// Insert `key`; `false` if an identical key is already stored.
    pub fn insert(&mut self, key: &[u8]) -> bool {
        match self.root {
            None => {
                self.root = Some(self.alloc(key));
                true
            }
            Some(mut cur) => loop {
                let d = levenshtein(&self.keys[cur], key);
                if d == 0 {
                    return false;
                }
                match self.children[cur].get(&d) {
                    Some(&next) => cur = next,
                    None => {
                        let i = self.alloc(key);
                        self.children[cur].insert(d, i);
                        return true;
                    }
                }
            },
        }
    }

    /// Exact membership.
    pub fn contains(&self, key: &[u8]) -> bool {
        let Some(mut cur) = self.root else {
            return false;
        };
        loop {
            let d = levenshtein(&self.keys[cur], key);
            if d == 0 {
                return true;
            }
            match self.children[cur].get(&d) {
                Some(&next) => cur = next,
                None => return false,
            }
        }
    }

    /// All `(distance, index)` pairs with `distance <= r`, sorted by
    /// `(distance, key)`.
    pub fn within(&self, query: &[u8], r: u32) -> Vec<(u32, usize)> {
        let mut out = Vec::new();
        let Some(root) = self.root else { return out };
        let mut stack = vec![root];
        while let Some(cur) = stack.pop() {
            let d = levenshtein(&self.keys[cur], query);
            if d <= r {
                out.push((d, cur));
            }
            let lo = d.saturating_sub(r);
            for (_, &ch) in self.children[cur].range(lo..=d + r) {
                stack.push(ch);
            }
        }
        out.sort_by(|a, b| (a.0, &self.keys[a.1]).cmp(&(b.0, &self.keys[b.1])));
        out
    }

    /// `(distance, index)` of the closest key; ties resolve to the
    /// smallest key. `None` when empty.
    pub fn nearest(&self, query: &[u8]) -> Option<(u32, usize)> {
        let root = self.root?;
        // Bounded walk: keep the best distance seen and only enter
        // subtrees whose edges can still beat it.
        let mut best: Option<(u32, usize)> = None;
        let mut stack = vec![root];
        while let Some(cur) = stack.pop() {
            let d = levenshtein(&self.keys[cur], query);
            let better = match best {
                None => true,
                Some((bd, bi)) => (d, &self.keys[cur]) < (bd, &self.keys[bi]),
            };
            if better {
                best = Some((d, cur));
            }
            let bound = best.map_or(u32::MAX, |(bd, _)| bd);
            let lo = d.saturating_sub(bound);
            let hi = d.saturating_add(bound);
            for (_, &ch) in self.children[cur].range(lo..=hi) {
                stack.push(ch);
            }
        }
        best
    }

    fn alloc(&mut self, key: &[u8]) -> usize {
        self.keys.push(key.to_vec());
        self.children.push(BTreeMap::new());
        self.keys.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Vec<&'static [u8]> {
        vec![
            b"cat", b"cut", b"bat", b"carta", b"dog", b"cart", b"car", b"at", b"cot",
        ]
    }

    #[test]
    fn basic_membership() {
        let mut t = BkTree::new();
        for w in ["cat", "cut", "bat"] {
            assert!(t.insert(w.as_bytes()));
        }
        assert!(!t.insert(b"cat")); // duplicate
        assert!(t.contains(b"cut"));
        assert!(!t.contains(b"cap"));
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn nearest_and_within_small() {
        let mut t = BkTree::new();
        for w in words() {
            t.insert(w);
        }
        assert_eq!(t.nearest(b"caar"), Some((1, 6))); // "car" at idx 6
        let got: Vec<u32> = t.within(b"cat", 1).iter().map(|e| e.0).collect();
        assert_eq!(got, vec![0, 1, 1, 1, 1, 1, 1]); // cat,cut,bat,cart,car,cot,at
    }

    #[test]
    fn matches_bruteforce_oracle() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0xBDB1);
        let mut t = BkTree::new();
        let mut model: Vec<Vec<u8>> = Vec::new();
        // Build a dictionary over a tiny alphabet so distances
        // collide and pruning actually engages.
        for _ in 0..80 {
            let len = 1 + rng.next_u64() % 7;
            let w: Vec<u8> = (0..len)
                .map(|_| b'a' + (rng.next_u64() % 4) as u8)
                .collect();
            let fresh = !model.contains(&w);
            assert_eq!(t.insert(&w), fresh);
            if fresh {
                model.push(w);
            }
        }
        for _ in 0..300 {
            let len = rng.next_u64() % 8;
            let q: Vec<u8> = (0..len)
                .map(|_| b'a' + (rng.next_u64() % 4) as u8)
                .collect();
            // contains
            assert_eq!(t.contains(&q), model.contains(&q));
            // within
            let r = rng.next_u64() as u32 % 4;
            let mut want: Vec<(u32, usize)> = model
                .iter()
                .enumerate()
                .map(|(i, w)| (levenshtein(w, &q), i))
                .filter(|e| e.0 <= r)
                .collect();
            want.sort_by(|a, b| (a.0, &model[a.1]).cmp(&(b.0, &model[b.1])));
            assert_eq!(t.within(&q, r), want);
            // nearest
            let want_n = model
                .iter()
                .enumerate()
                .map(|(i, w)| (levenshtein(w, &q), i))
                .min_by(|a, b| (a.0, &model[a.1]).cmp(&(b.0, &model[b.1])));
            assert_eq!(t.nearest(&q), want_n);
        }
    }

    #[test]
    fn empty_index() {
        let t = BkTree::new();
        assert!(!t.contains(b"x"));
        assert_eq!(t.nearest(b"x"), None);
        assert!(t.within(b"x", 3).is_empty());
    }
}

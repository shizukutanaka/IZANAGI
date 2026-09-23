//! Pairing heap (Fredman–Sedgewick–Sleator–Tarjan, 1986) — a
//! meldable priority queue whose `push`/`meld`/`pop` all touch
//! only the root lane. Entries are `(prio, key)` `u64` pairs and
//! the minimum is compared by the whole pair, so the popped
//! sequence is a pure function of the op log (canonical order:
//! equal priorities come out in key order). Nodes live in a
//! `Vec` arena — no pointers, no addresses in the output.
//!
//! `meld` absorbs another heap in O(1), which makes this the
//! priority queue for structures that merge repeatedly
//! (`mcflow`-style multi-source relaxations, tournament merging
//! of per-shard queues).
//!
//! ```
//! use izanagi_kit::pairingheap::PairingHeap;
//! let mut h = PairingHeap::new();
//! h.push(3, 30);
//! h.push(1, 10);
//! h.push(2, 20);
//! assert_eq!(h.pop(), Some((1, 10)));
//! assert_eq!(h.pop(), Some((2, 20)));
//! ```

#[derive(Clone)]
struct Node {
    prio: u64,
    key: u64,
    child: Option<usize>,
    sib: Option<usize>,
}

fn lt(a: (u64, u64), b: (u64, u64)) -> bool {
    a < b
}

/// Arena-backed pairing heap of `(prio, key)` pairs.
pub struct PairingHeap {
    nodes: Vec<Node>,
    root: Option<usize>,
    len: usize,
}

impl Default for PairingHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl PairingHeap {
    /// Empty heap.
    pub fn new() -> Self {
        PairingHeap {
            nodes: Vec::new(),
            root: None,
            len: 0,
        }
    }

    fn meld_nodes(&mut self, a: Option<usize>, b: Option<usize>) -> Option<usize> {
        let (a, b) = match (a, b) {
            (None, x) | (x, None) => return x,
            (Some(a), Some(b)) => (a, b),
        };
        let (winner, loser) = if lt(
            (self.nodes[a].prio, self.nodes[a].key),
            (self.nodes[b].prio, self.nodes[b].key),
        ) {
            (a, b)
        } else {
            (b, a)
        };
        self.nodes[loser].sib = self.nodes[winner].child;
        self.nodes[winner].child = Some(loser);
        Some(winner)
    }

    /// Insert `(prio, key)`; equal `(prio,key)` pairs are stored
    /// (multiset semantics) and pop in arrival order among
    /// themselves — still a pure function of the op log.
    pub fn push(&mut self, prio: u64, key: u64) {
        self.nodes.push(Node {
            prio,
            key,
            child: None,
            sib: None,
        });
        let n = self.nodes.len() - 1;
        self.root = self.meld_nodes(self.root, Some(n));
        self.len += 1;
    }

    /// Absorb `other` (drained empty) in O(1).
    pub fn meld(&mut self, mut other: PairingHeap) {
        self.len += other.len;
        other.len = 0;
        let base = self.nodes.len();
        for n in &mut other.nodes {
            n.child = n.child.map(|c| c + base);
            n.sib = n.sib.map(|c| c + base);
        }
        let oroot = other.root.map(|r| r + base);
        self.nodes.append(&mut other.nodes);
        self.root = self.meld_nodes(self.root, oroot);
    }

    /// Minimum `(prio, key)` without removing it.
    pub fn peek(&self) -> Option<(u64, u64)> {
        self.root.map(|r| (self.nodes[r].prio, self.nodes[r].key))
    }

    /// Remove and return the minimum `(prio, key)`.
    pub fn pop(&mut self) -> Option<(u64, u64)> {
        let r = self.root?;
        let ans = (self.nodes[r].prio, self.nodes[r].key);
        self.len -= 1;
        // Two-pass pairing: fold left-to-right into pairs, then
        // fold right-to-left across the pair winners — the
        // classical pass that keeps the tree shallow.
        let mut pairs: Vec<usize> = Vec::new();
        let mut c = self.nodes[r].child;
        while let Some(x) = c {
            let next = self.nodes[x].sib;
            self.nodes[x].sib = None;
            pairs.push(x);
            c = next;
        }
        let mut i = 0;
        let mut winners: Vec<Option<usize>> = Vec::with_capacity(pairs.len() / 2 + 1);
        while i + 1 < pairs.len() {
            winners.push(self.meld_nodes(Some(pairs[i]), Some(pairs[i + 1])));
            i += 2;
        }
        if i < pairs.len() {
            winners.push(Some(pairs[i]));
        }
        let mut new_root: Option<usize> = None;
        for w in winners.into_iter().rev() {
            new_root = self.meld_nodes(w, new_root);
        }
        self.root = new_root;
        Some(ans)
    }

    /// Number of stored pairs.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    fn drain(h: &mut PairingHeap) -> Vec<(u64, u64)> {
        let mut out = Vec::new();
        while let Some(x) = h.pop() {
            out.push(x);
        }
        out
    }

    #[test]
    fn basic_order() {
        let mut h = PairingHeap::new();
        for (p, k) in [(5, 50), (1, 10), (3, 30), (3, 3), (1, 1)] {
            h.push(p, k);
        }
        assert_eq!(h.peek(), Some((1, 1)));
        assert_eq!(
            drain(&mut h),
            vec![(1, 1), (1, 10), (3, 3), (3, 30), (5, 50)]
        );
        assert!(h.is_empty());
    }

    #[test]
    fn pop_sequence_matches_btreemap() {
        // Interleaved push/pop/meld oracle against a BTreeMap
        // multiset keyed by (prio,key).
        let mut rng = SplitMix64::new(4242);
        for _ in 0..60 {
            let mut h = PairingHeap::new();
            let mut spare = PairingHeap::new();
            let mut oracle: BTreeMap<(u64, u64), usize> = BTreeMap::new();
            let mut spare_oracle: BTreeMap<(u64, u64), usize> = BTreeMap::new();
            for _ in 0..300 {
                match rng.below(10) {
                    0..=4 => {
                        let e = (rng.below(50) as u64, rng.below(5) as u64);
                        if rng.below(4) == 0 {
                            spare.push(e.0, e.1);
                            *spare_oracle.entry(e).or_insert(0) += 1;
                        } else {
                            h.push(e.0, e.1);
                            *oracle.entry(e).or_insert(0) += 1;
                        }
                    }
                    5 => {
                        // Meld drains 'spare' into 'h'.
                        h.meld(std::mem::take(&mut spare));
                        for (k, c) in std::mem::take(&mut spare_oracle) {
                            *oracle.entry(k).or_insert(0) += c;
                        }
                        spare = PairingHeap::new();
                    }
                    _ => {
                        let expect = oracle.iter().next().map(|(&k, _)| k);
                        assert_eq!(h.pop(), expect);
                        if let Some(k) = expect {
                            let c = oracle
                                .get_mut(&k)
                                .unwrap_or_else(|| panic!("oracle missing {k:?}"));
                            *c -= 1;
                            if *c == 0 {
                                oracle.remove(&k);
                            }
                        }
                    }
                }
                assert_eq!(h.len(), oracle.values().sum::<usize>());
            }
        }
    }

    #[test]
    fn meld_preserves_all() {
        let mut a = PairingHeap::new();
        let mut b = PairingHeap::new();
        a.push(1, 1);
        a.push(9, 9);
        b.push(2, 2);
        b.push(8, 8);
        a.meld(b);
        assert_eq!(a.len(), 4);
        assert_eq!(drain(&mut a), vec![(1, 1), (2, 2), (8, 8), (9, 9)]);
    }

    #[test]
    fn large_descent_sorted() {
        // A monotone decreasing input must still pop sorted —
        // exercises the pairing pass on a deep root lane.
        let mut h = PairingHeap::new();
        for i in (0..500u64).rev() {
            h.push(i, i);
        }
        let out = drain(&mut h);
        let mut want: Vec<(u64, u64)> = (0..500).map(|i| (i, i)).collect();
        want.sort();
        assert_eq!(out, want);
    }
}

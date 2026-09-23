//! Deterministic skip list over `u64` keys (Pugh 1990). Instead of
//! a coin flip, each key's level is `trailing_zeros` of a seeded
//! hash — a geometric-distribution level that is a pure function of
//! `(key, seed)`, so two replicas inserting the same key set always
//! build the same tower structure. Lookup is `O(log n)` expected;
//! iteration is the ascending bottom lane — a sorted ordered map
//! with cheaper expected splice work than a B-tree page split.
//!
//! ```
//! use izanagi_kit::skiplist::SkipList;
//! let mut s = SkipList::new(7);
//! s.insert(40);
//! s.insert(10);
//! s.insert(70);
//! assert!(s.contains(40));
//! assert_eq!(s.iter().collect::<Vec<u64>>(), vec![10, 40, 70]);
//! s.remove(40);
//! assert_eq!(s.iter().collect::<Vec<u64>>(), vec![10, 70]);
//! ```

/// Highest tower level (geometric tail is truncated here).
const MAX_LEVEL: usize = 32;

fn hash_key(key: u64, seed: u64) -> u64 {
    // SplitMix64 finalizer over key^seed — pure function of both.
    let mut z = key.wrapping_add(seed).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn level_of(key: u64, seed: u64) -> usize {
    let lvl = hash_key(key, seed).trailing_zeros() as usize + 1;
    lvl.min(MAX_LEVEL)
}

struct Node {
    key: u64,
    fwd: Vec<usize>, // next node index per level; NONE sentinel
}

const NONE: usize = 0; // index 0 is the never-allocated head sentinel

/// Ordered `u64` set as a deterministic skip list.
pub struct SkipList {
    seed: u64,
    /// `head[l]` = first node reachable at level `l` (node index or
    /// `NONE`).
    head: [usize; MAX_LEVEL],
    nodes: Vec<Node>,
    free: Vec<usize>,
    len: usize,
}

impl SkipList {
    /// Empty set with level distribution seeded by `seed`.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            head: [NONE; MAX_LEVEL],
            // Slot 0 is the sentinel — never a real node.
            nodes: vec![Node {
                key: 0,
                fwd: Vec::new(),
            }],
            free: Vec::new(),
            len: 0,
        }
    }

    /// Predecessor node index at each level (0 = before first).
    fn preds(&self, key: u64) -> [usize; MAX_LEVEL] {
        let mut p = [NONE; MAX_LEVEL];
        let mut cur = NONE;
        for l in (0..MAX_LEVEL).rev() {
            loop {
                let next = if cur == NONE {
                    self.head[l]
                } else {
                    *self.nodes[cur].fwd.get(l).unwrap_or(&NONE)
                };
                if next != NONE && self.nodes[next].key < key {
                    cur = next;
                } else {
                    break;
                }
            }
            p[l] = cur;
        }
        p
    }

    fn next_at(&self, level: usize, pred: usize) -> usize {
        if pred == NONE {
            self.head[level]
        } else {
            *self.nodes[pred].fwd.get(level).unwrap_or(&NONE)
        }
    }

    /// Insert `key`; returns whether it was new.
    pub fn insert(&mut self, key: u64) -> bool {
        let p = self.preds(key);
        let nxt = self.next_at(0, p[0]);
        if nxt != NONE && self.nodes[nxt].key == key {
            return false;
        }
        let lvl = level_of(key, self.seed);
        let idx = match self.free.pop() {
            Some(i) => {
                self.nodes[i].key = key;
                self.nodes[i].fwd.clear();
                self.nodes[i].fwd.resize(lvl, NONE);
                i
            }
            None => {
                self.nodes.push(Node {
                    key,
                    fwd: vec![NONE; lvl],
                });
                self.nodes.len() - 1
            }
        };
        for (l, &pred) in p.iter().enumerate().take(lvl) {
            let old_next = self.next_at(l, pred);
            self.nodes[idx].fwd[l] = old_next;
            if pred == NONE {
                self.head[l] = idx;
            } else {
                if self.nodes[pred].fwd.len() <= l {
                    self.nodes[pred].fwd.resize(l + 1, NONE);
                }
                self.nodes[pred].fwd[l] = idx;
            }
        }
        self.len += 1;
        true
    }

    /// Remove `key`; returns whether it was present.
    pub fn remove(&mut self, key: u64) -> bool {
        let p = self.preds(key);
        let idx = self.next_at(0, p[0]);
        if idx == NONE || self.nodes[idx].key != key {
            return false;
        }
        for (l, &nxt) in self.nodes[idx].fwd.clone().iter().enumerate() {
            let pred = p[l];
            if pred == NONE {
                self.head[l] = nxt;
            } else if self.nodes[pred].fwd.len() > l {
                self.nodes[pred].fwd[l] = nxt;
            }
        }
        self.free.push(idx);
        self.len -= 1;
        true
    }

    /// Membership test.
    pub fn contains(&self, key: u64) -> bool {
        let p = self.preds(key);
        let nxt = self.next_at(0, p[0]);
        nxt != NONE && self.nodes[nxt].key == key
    }

    /// Element count.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Ascending iteration over every member.
    pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
        let mut v = Vec::with_capacity(self.len);
        let mut cur = self.head[0];
        while cur != NONE {
            v.push(self.nodes[cur].key);
            cur = *self.nodes[cur].fwd.first().unwrap_or(&NONE);
        }
        v.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn basic_ops() {
        let mut s = SkipList::new(1);
        assert!(s.is_empty());
        assert!(s.insert(5));
        assert!(!s.insert(5));
        assert!(s.contains(5));
        assert!(s.remove(5));
        assert!(!s.remove(5));
        assert!(!s.contains(5));
    }

    #[test]
    fn oracle_all_ops() {
        let mut rng = SplitMix64::new(19);
        for seed in [0u64, 1, 42] {
            let mut s = SkipList::new(seed);
            let mut b = BTreeSet::new();
            for _ in 0..2000 {
                let k = rng.below(500) as u64;
                match rng.below(3) {
                    0 => assert_eq!(s.insert(k), b.insert(k)),
                    1 => assert_eq!(s.remove(k), b.remove(&k)),
                    _ => assert_eq!(s.contains(k), b.contains(&k)),
                }
            }
            assert_eq!(s.len(), b.len());
            let got: Vec<u64> = s.iter().collect();
            let want: Vec<u64> = b.iter().copied().collect();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn structure_is_pure_function() {
        // Two lists built from the same key multiset share shape.
        let mut rng = SplitMix64::new(3);
        let xs: Vec<u64> = (0..200).map(|_| rng.below(10_000) as u64).collect();
        let mut a = SkipList::new(9);
        let mut b = SkipList::new(9);
        for &x in &xs {
            a.insert(x);
        }
        let mut ys = xs.clone();
        ys.reverse();
        for &y in &ys {
            b.insert(y);
        }
        assert_eq!(a.iter().collect::<Vec<_>>(), b.iter().collect::<Vec<_>>());
        // Tower shape is order-independent: the key sequence on every
        // level lane matches across the two build orders. (Node indices
        // are allocation order — compare keys, not indices.)
        for l in 0..MAX_LEVEL {
            let lane = |s: &SkipList| {
                let mut v = Vec::new();
                let mut cur = s.head[l];
                while cur != NONE {
                    v.push(s.nodes[cur].key);
                    cur = *s.nodes[cur].fwd.get(l).unwrap_or(&NONE);
                }
                v
            };
            assert_eq!(lane(&a), lane(&b), "lane {l} diverged");
        }
    }

    #[test]
    fn dense_level_distribution() {
        // Geometric-ish: most keys at low level, some towers high.
        let mut s = SkipList::new(0);
        for k in 0..10_000u64 {
            s.insert(k);
        }
        let tall = s.nodes.iter().filter(|n| n.fwd.len() >= 4).count();
        assert!(tall > 0);
        assert!(tall < 3000);
    }
}

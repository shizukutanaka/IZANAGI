//! Binary bitwise trie over `u64` — answers **max/min xor
//! queries** (`max_xor`, `min_xor`, `max_xor_pair`) in `O(64)`
//! by descending toward the complementary bit. Complements
//! [`crate::xorbasis`]: the linear basis solves *subset*-xor
//! extremization (any combination of elements); this solves
//! *single-element* partner extremization (nearest/farthest
//! under xor distance).
//!
//! The trie is a pure function of its key set: node ids come
//! from first-touch arena order, queries are deterministic
//! greedy descents, `insert` is idempotent (`cnt` per node
//! tracks multiplicity so `remove` works).
//!
//! ```
//! use izanagi_kit::xortrie::XorTrie;
//! let mut t = XorTrie::new();
//! for v in [3u64, 10, 5, 25] {
//!     t.insert(v);
//! }
//! assert_eq!(t.max_xor(10), Some(25)); // 10^25 = 19 max
//! assert_eq!(t.min_xor(10), Some(10)); // 10^10 = 0
//! assert_eq!(t.max_xor_pair(), Some((5, 25))); // 5^25 = 28
//! ```

const NIL: u32 = u32::MAX;
const BITS: u32 = 64;

#[derive(Clone)]
struct Node {
    child: [u32; 2],
    /// keys passing through this node (multiplicity)
    cnt: u32,
}

/// Bitwise trie of `u64` keys — `O(64)` per operation.
pub struct XorTrie {
    nodes: Vec<Node>,
    len: usize,
}

impl XorTrie {
    /// Empty trie.
    pub fn new() -> XorTrie {
        XorTrie {
            nodes: vec![Node {
                child: [NIL, NIL],
                cnt: 0,
            }],
            len: 0,
        }
    }

    /// Number of keys (distinct multiplicities counted once
    /// per `insert`; `remove` decrements).
    pub fn len(&self) -> usize {
        self.len
    }

    /// True iff no keys.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insert `x` once (duplicates bump the multiplicity —
    /// `remove` undoes one).
    pub fn insert(&mut self, x: u64) {
        let mut cur = 0u32;
        self.nodes[0].cnt += 1;
        for b in (0..BITS).rev() {
            let bit = ((x >> b) & 1) as usize;
            let nxt = self.nodes[cur as usize].child[bit];
            let nxt = if nxt == NIL {
                let id = self.nodes.len() as u32;
                self.nodes.push(Node {
                    child: [NIL, NIL],
                    cnt: 0,
                });
                self.nodes[cur as usize].child[bit] = id;
                id
            } else {
                nxt
            };
            self.nodes[nxt as usize].cnt += 1;
            cur = nxt;
        }
        self.len += 1;
    }

    /// `x` present (multiplicity ≥ 1)?
    pub fn contains(&self, x: u64) -> bool {
        let mut cur = 0u32;
        for b in (0..BITS).rev() {
            let bit = ((x >> b) & 1) as usize;
            let nxt = self.nodes[cur as usize].child[bit];
            if nxt == NIL || self.nodes[nxt as usize].cnt == 0 {
                return false;
            }
            cur = nxt;
        }
        true
    }

    /// Remove one copy of `x`; returns whether it was present.
    pub fn remove(&mut self, x: u64) -> bool {
        if !self.contains(x) {
            return false;
        }
        let mut cur = 0u32;
        self.nodes[0].cnt -= 1;
        for b in (0..BITS).rev() {
            let bit = ((x >> b) & 1) as usize;
            let nxt = self.nodes[cur as usize].child[bit];
            self.nodes[nxt as usize].cnt -= 1;
            cur = nxt;
        }
        self.len -= 1;
        true
    }

    /// Greedy descent: at each level take the branch whose
    /// bit matches `want`, else the only alternative.
    fn descend(&self, x: u64, want_opposite: bool) -> Option<u64> {
        if self.len == 0 {
            return None;
        }
        let mut cur = 0u32;
        let mut y = 0u64;
        for b in (0..BITS).rev() {
            let bit = ((x >> b) & 1) as usize;
            let want = if want_opposite { 1 - bit } else { bit };
            let (pref, alt) = (
                self.nodes[cur as usize].child[want],
                self.nodes[cur as usize].child[1 - want],
            );
            let (nxt, taken) = if pref != NIL && self.nodes[pref as usize].cnt > 0 {
                (pref, want)
            } else if alt != NIL && self.nodes[alt as usize].cnt > 0 {
                (alt, 1 - want)
            } else {
                return None;
            };
            y |= (taken as u64) << b;
            cur = nxt;
        }
        Some(y)
    }

    /// Partner `y` in the trie maximizing `x ^ y`; `None` if
    /// empty. The xor value determines `y` uniquely.
    pub fn max_xor(&self, x: u64) -> Option<u64> {
        self.descend(x, true)
    }

    /// Partner `y` minimizing `x ^ y`; `None` if empty.
    pub fn min_xor(&self, x: u64) -> Option<u64> {
        self.descend(x, false)
    }

    /// Pair `(a, b)` with `a < b` maximizing `a ^ b` over all
    /// distinct pairs — `O(n·64)`: the best partner of every
    /// element comes from one greedy descent each, and the
    /// global best pair is among them. Ties break on the
    /// smallest `a`, then smallest `b`. `None` when fewer
    /// than 2 distinct keys exist.
    pub fn max_xor_pair(&self) -> Option<(u64, u64)> {
        if self.len < 2 {
            return None;
        }
        let mut keys = Vec::new();
        self.collect(0, 0, 0, &mut keys);
        let mut best: Option<(u64, u64, u64)> = None;
        for &x in &keys {
            if let Some(y) = self.max_xor(x) {
                if x != y {
                    let v = x ^ y;
                    let (a, b) = if x < y { (x, y) } else { (y, x) };
                    let better = match best {
                        None => true,
                        Some((bv, ba, bb)) => v > bv || (v == bv && (a, b) < (ba, bb)),
                    };
                    if better {
                        best = Some((v, a, b));
                    }
                }
            }
        }
        best.map(|(_, a, b)| (a, b))
    }

    fn collect(&self, id: u32, depth: u32, acc: u64, out: &mut Vec<u64>) {
        if depth == BITS {
            if self.nodes[id as usize].cnt > 0 {
                out.push(acc);
            }
            return;
        }
        for bit in 0..2usize {
            let nxt = self.nodes[id as usize].child[bit];
            if nxt != NIL && self.nodes[nxt as usize].cnt > 0 {
                self.collect(
                    nxt,
                    depth + 1,
                    acc | ((bit as u64) << (BITS - 1 - depth)),
                    out,
                );
            }
        }
    }
}

impl Default for XorTrie {
    fn default() -> XorTrie {
        XorTrie::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn brute_max(set: &BTreeSet<u64>, x: u64) -> Option<u64> {
        set.iter().map(|&y| (x ^ y, y)).max().map(|(_, y)| y)
    }

    fn brute_min(set: &BTreeSet<u64>, x: u64) -> Option<u64> {
        set.iter().map(|&y| (x ^ y, y)).min().map(|(_, y)| y)
    }

    fn brute_pair(set: &BTreeSet<u64>) -> Option<(u64, u64)> {
        if set.len() < 2 {
            return None;
        }
        let v: Vec<u64> = set.iter().copied().collect();
        let mut best: Option<(u64, u64, u64)> = None;
        for i in 0..v.len() {
            for j in i + 1..v.len() {
                let (a, b) = (v[i], v[j]);
                let x = a ^ b;
                let better = match best {
                    None => true,
                    Some((bx, ba, bb)) => x > bx || (x == bx && (a, b) < (ba, bb)),
                };
                if better {
                    best = Some((x, a, b));
                }
            }
        }
        best.map(|(_, a, b)| (a, b))
    }

    #[test]
    fn basics() {
        let mut t = XorTrie::new();
        assert!(t.is_empty() && t.max_xor(7).is_none());
        for v in [3u64, 10, 5, 25] {
            t.insert(v);
        }
        assert_eq!(t.len(), 4);
        assert!(t.contains(10) && !t.contains(11));
        assert_eq!(t.max_xor(10), Some(25));
        assert_eq!(t.min_xor(10), Some(10));
        assert_eq!(t.max_xor_pair(), Some((5, 25)));
        assert!(t.remove(10));
        assert!(!t.contains(10) && !t.remove(10));
        // {3,5,25}: 5^25=28 still the max pair
        assert_eq!(t.max_xor_pair(), Some((5, 25)));
        // multiplicity: len counts live inserts (5 after
        // one remove + two inserts of 7)
        t.insert(7);
        t.insert(7);
        assert_eq!(t.len(), 5);
        assert!(t.remove(7));
        assert!(t.contains(7));
        assert_eq!(t.len(), 4);
        // extremes
        let mut t2 = XorTrie::new();
        t2.insert(u64::MAX);
        t2.insert(0);
        assert_eq!(t2.max_xor(0), Some(u64::MAX));
        assert_eq!(t2.max_xor(u64::MAX), Some(0));
        assert_eq!(t2.max_xor_pair(), Some((0, u64::MAX)));
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(41);
        for _round in 0..150 {
            let n = (rng.below(30) + 1) as usize;
            let width = [4u32, 8, 16, 64][rng.below(4) as usize];
            let mask = if width == 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            };
            let mut t = XorTrie::new();
            let mut set = BTreeSet::new();
            for _ in 0..n {
                let x = rng.next_u64() & mask;
                t.insert(x);
                set.insert(x);
            }
            // queries incl. non-members
            for _ in 0..50 {
                let x = rng.next_u64() & mask;
                assert_eq!(
                    t.max_xor(x).map(|y| x ^ y),
                    brute_max(&set, x).map(|y| x ^ y)
                );
                assert_eq!(
                    t.min_xor(x).map(|y| x ^ y),
                    brute_min(&set, x).map(|y| x ^ y)
                );
            }
            assert_eq!(t.max_xor_pair(), brute_pair(&set), "round {_round} pair");
        }
    }

    #[test]
    fn remove_consistency() {
        let mut rng = SplitMix64::new(7);
        let mut t = XorTrie::new();
        let mut set = BTreeSet::new();
        // distinct-key ops so trie cnt stays ≤1 and len
        // tracks the set exactly
        for _ in 0..300 {
            let x = rng.below(64) as u64;
            if rng.below(2) == 0 {
                if set.insert(x) {
                    t.insert(x);
                }
            } else if set.remove(&x) {
                assert!(t.remove(x));
            }
            assert_eq!(t.len(), set.len());
        }
        let mut tr = Vec::new();
        t.collect(0, 0, 0, &mut tr);
        assert_eq!(BTreeSet::from_iter(tr), set);
    }
}

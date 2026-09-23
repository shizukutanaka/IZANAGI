//! Implicit-key treap — a sequence as a balanced BST keyed by
//! position rather than value. Every position is just an index
//! into the in-order walk, so `split(k)` / `merge` give
//! `insert`/`remove`/`reverse`/`get` in `O(log n)` expected
//! time — a `Vec<T>` that supports splicing without `O(n)`
//! memmove.
//!
//! Determinism: node priorities are drawn from a seeded
//! `SplitMix64` stream, so the whole tree shape is a pure
//! function of `(seed, operation sequence)` — no addresses, no
//! randomness outside the crate. Lazy `rev` flags propagate on
//! descent exactly once per visited node, so `to_vec` is always
//! the canonical read.
//!
//! ```
//! use izanagi_kit::imptreap::ImplicitTreap;
//! let mut t = ImplicitTreap::new(7);
//! t.push_back(1);
//! t.push_back(2);
//! t.push_back(3);
//! t.insert(1, 9);
//! t.reverse(0, 3);
//! assert_eq!(t.to_vec(), vec![2, 9, 1, 3]);
//! ```
//!
//! References: Seidel & Aragon (1996) treaps; e-maxx implicit
//! treap for the split/merge bookkeeping.

use crate::rng::SplitMix64;

const NIL: u32 = !0;

#[derive(Clone)]
struct Node<T> {
    pr: u64,
    ch: [u32; 2],
    sz: u32,
    val: T,
    rev: bool,
}

/// A deterministic implicit-key treap holding a sequence of `T`.
/// Capacity is unbounded; node slots recycle through a free
/// list so `remove`/`insert` stay `O(log n)` amortized.
pub struct ImplicitTreap<T> {
    ns: Vec<Node<T>>,
    free: Vec<u32>,
    root: u32,
    rng: SplitMix64,
}

impl<T: Clone> ImplicitTreap<T> {
    /// Empty sequence; `seed` drives the node-priority stream.
    pub fn new(seed: u64) -> Self {
        ImplicitTreap {
            ns: Vec::new(),
            free: Vec::new(),
            root: NIL,
            rng: SplitMix64::new(seed),
        }
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.size(self.root) as usize
    }

    /// `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn size(&self, n: u32) -> u32 {
        if n == NIL {
            0
        } else {
            self.ns[n as usize].sz
        }
    }

    fn pull(&mut self, n: u32) {
        if n != NIL {
            let (l, r) = (self.ns[n as usize].ch[0], self.ns[n as usize].ch[1]);
            self.ns[n as usize].sz = 1 + self.size(l) + self.size(r);
        }
    }

    fn push(&mut self, n: u32) {
        if n != NIL && self.ns[n as usize].rev {
            let (l, r) = (self.ns[n as usize].ch[0], self.ns[n as usize].ch[1]);
            self.ns[n as usize].ch = [r, l];
            if l != NIL {
                self.ns[l as usize].rev ^= true;
            }
            if r != NIL {
                self.ns[r as usize].rev ^= true;
            }
            self.ns[n as usize].rev = false;
        }
    }

    fn alloc(&mut self, val: T) -> u32 {
        let pr = self.rng.next_u64();
        let node = Node {
            pr,
            ch: [NIL, NIL],
            sz: 1,
            val,
            rev: false,
        };
        match self.free.pop() {
            Some(i) => {
                self.ns[i as usize] = node;
                i
            }
            None => {
                self.ns.push(node);
                (self.ns.len() - 1) as u32
            }
        }
    }

    /// `(left with the first k elements, the rest)`.
    fn split(&mut self, n: u32, k: u32) -> (u32, u32) {
        if n == NIL {
            return (NIL, NIL);
        }
        self.push(n);
        let lsz = self.size(self.ns[n as usize].ch[0]);
        if k <= lsz {
            let (l, r) = self.split(self.ns[n as usize].ch[0], k);
            self.ns[n as usize].ch[0] = r;
            self.pull(n);
            (l, n)
        } else {
            let (l, r) = self.split(self.ns[n as usize].ch[1], k - lsz - 1);
            self.ns[n as usize].ch[1] = l;
            self.pull(n);
            (n, r)
        }
    }

    fn merge(&mut self, a: u32, b: u32) -> u32 {
        if a == NIL {
            return b;
        }
        if b == NIL {
            return a;
        }
        if self.ns[a as usize].pr < self.ns[b as usize].pr {
            self.push(a);
            let r = self.merge(self.ns[a as usize].ch[1], b);
            self.ns[a as usize].ch[1] = r;
            self.pull(a);
            a
        } else {
            self.push(b);
            let l = self.merge(a, self.ns[b as usize].ch[0]);
            self.ns[b as usize].ch[0] = l;
            self.pull(b);
            b
        }
    }

    /// Insert `val` at `pos` (0..=len).
    pub fn insert(&mut self, pos: usize, val: T) {
        let pos = pos.min(self.len());
        let n = self.alloc(val);
        let (l, r) = self.split(self.root, pos as u32);
        let m = self.merge(l, n);
        self.root = self.merge(m, r);
    }

    /// Append `val` at the end.
    pub fn push_back(&mut self, val: T) {
        let n = self.alloc(val);
        self.root = self.merge(self.root, n);
    }

    /// Remove and return the element at `pos` (`None` out of range).
    pub fn remove(&mut self, pos: usize) -> Option<T> {
        if pos >= self.len() {
            return None;
        }
        let (l, r) = self.split(self.root, pos as u32);
        let (m, rr) = self.split(r, 1);
        self.root = self.merge(l, rr);
        let v = self.ns[m as usize].val.clone();
        self.ns[m as usize].sz = 0;
        self.free.push(m);
        Some(v)
    }

    /// Element at `pos` (`None` out of range).
    ///
    /// Reads through pending `rev` flags without pushing them:
    /// a node's flag reverses the *whole subtree*, so the parity
    /// of flags on the descent decides each child's effective
    /// side (parent flip XOR child's own flag).
    pub fn get(&self, pos: usize) -> Option<&T> {
        let mut n = self.root;
        let mut k = pos;
        let mut flip = false;
        loop {
            if n == NIL {
                return None;
            }
            let node = &self.ns[n as usize];
            let f = flip ^ node.rev;
            let (l, r) = if f {
                (node.ch[1], node.ch[0])
            } else {
                (node.ch[0], node.ch[1])
            };
            let lsz = if l == NIL { 0 } else { self.ns[l as usize].sz };
            if k < lsz as usize {
                n = l;
            } else if k == lsz as usize {
                return Some(&self.ns[n as usize].val);
            } else {
                k -= lsz as usize + 1;
                n = r;
            }
            flip = f;
        }
    }

    /// Reverse the range `[l, r)` (`l < r` required for effect).
    pub fn reverse(&mut self, l: usize, r: usize) {
        let (l, r) = (l.min(self.len()), r.min(self.len()));
        if l >= r {
            return;
        }
        let (a, bc) = self.split(self.root, l as u32);
        let (b, c) = self.split(bc, (r - l) as u32);
        if b != NIL {
            self.ns[b as usize].rev ^= true;
        }
        let m = self.merge(a, b);
        self.root = self.merge(m, c);
    }

    /// In-order contents as a `Vec`, lazily unwinding `rev` flags.
    pub fn to_vec(&mut self) -> Vec<T> {
        fn walk<T: Clone>(t: &mut ImplicitTreap<T>, n: u32, out: &mut Vec<T>) {
            if n == NIL {
                return;
            }
            t.push(n);
            let (l, r) = (t.ns[n as usize].ch[0], t.ns[n as usize].ch[1]);
            walk(t, l, out);
            out.push(t.ns[n as usize].val.clone());
            walk(t, r, out);
        }
        let mut out = Vec::with_capacity(self.len());
        let root = self.root;
        walk(self, root, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64 as R;

    #[test]
    fn basic_ops() {
        let mut t = ImplicitTreap::new(1);
        for i in 0..10 {
            t.push_back(i);
        }
        assert_eq!(t.len(), 10);
        t.insert(5, 99);
        assert_eq!(t.get(5), Some(&99));
        assert_eq!(t.remove(0), Some(0));
        t.reverse(2, 7);
        let v = t.to_vec();
        let mut want: Vec<i32> = (0..10).collect();
        want.insert(5, 99);
        want.remove(0);
        want[2..7].reverse();
        assert_eq!(v, want);
    }

    #[test]
    fn oracle_against_vec() {
        let mut rng = R::new(0x1E77);
        let mut t = ImplicitTreap::new(99);
        let mut oracle: Vec<u32> = Vec::new();
        for _ in 0..600 {
            match rng.below(4) {
                0 => {
                    let pos = (rng.below((oracle.len() + 1) as u32)) as usize;
                    let v = rng.below(1000);
                    t.insert(pos, v);
                    oracle.insert(pos, v);
                }
                1 => {
                    if !oracle.is_empty() {
                        let pos = rng.below(oracle.len() as u32) as usize;
                        assert_eq!(t.remove(pos), Some(oracle.remove(pos)));
                    }
                }
                2 => {
                    if !oracle.is_empty() {
                        let l = rng.below(oracle.len() as u32) as usize;
                        let r = l + rng.below((oracle.len() - l) as u32) as usize;
                        t.reverse(l, r);
                        oracle[l..r].reverse();
                    }
                }
                _ => {
                    if !oracle.is_empty() {
                        let pos = rng.below(oracle.len() as u32) as usize;
                        assert_eq!(t.get(pos), oracle.get(pos));
                    }
                }
            }
            assert_eq!(t.len(), oracle.len());
        }
        assert_eq!(t.to_vec(), oracle);
    }

    #[test]
    fn determinism_same_seed() {
        let build = |seed| {
            let mut t = ImplicitTreap::new(seed);
            for i in 0..50 {
                t.insert(i % 7, i);
            }
            t.reverse(3, 40);
            t.to_vec()
        };
        assert_eq!(build(5), build(5));
    }

    #[test]
    fn empty_and_oob() {
        let mut t: ImplicitTreap<i32> = ImplicitTreap::new(0);
        assert!(t.is_empty());
        assert_eq!(t.get(0), None);
        assert_eq!(t.remove(0), None);
        t.reverse(0, 5); // clamps, no panic
    }
}

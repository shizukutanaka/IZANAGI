//! Fibonacci heap — the lazy-binomial-forest priority queue, on an
//! arena (`Vec`-indexed nodes, circular sibling rings) instead of
//! pointers. `push`/`decrease_key`/`meld` are O(1) lazy writes; the
//! work happens in [`FibHeap::pop`]'s degree-consolidation pass, so
//! amortized bounds hold per the Fredman–Tarjan analysis. Canonical
//! `(key, seq)` order keeps pop traces a pure function of the op
//! sequence — `seq` is the insertion counter, ties pop in FIFO order.
//!
//! ```
//! use izanagi_kit::fibheap::FibHeap;
//! let mut h = FibHeap::new();
//! h.push(5, 10);
//! let six = h.push(6, 20);
//! h.push(1, 30);
//! h.decrease_key(six, 0);
//! assert_eq!(h.peek(), Some((0, 20)));
//! assert_eq!(h.pop(), Some((0, 20)));
//! ```
//!
//! Reference: Fredman & Tarjan (1987), "Fibonacci heaps and their
//! uses in improved network optimization algorithms" (JACM).

/// Sentinel handle.
const NONE: u32 = !0;

#[derive(Clone, Copy)]
struct Node {
    key: u64,
    seq: u64,
    val: u32,
    parent: u32,
    child: u32,
    left: u32,
    right: u32,
    degree: u32,
    marked: bool,
    /// False once popped (until the slot is recycled) — stale-handle
    /// guard for `decrease_key`.
    live: bool,
}

/// A Fibonacci heap over `(u64 key, u32 payload)`.
pub struct FibHeap {
    nodes: Vec<Node>,
    root_list: u32,
    min: u32,
    len: usize,
    seq: u64,
    free: Vec<u32>,
}

impl Default for FibHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl FibHeap {
    /// Empty heap.
    pub const fn new() -> FibHeap {
        FibHeap {
            nodes: Vec::new(),
            root_list: NONE,
            min: NONE,
            len: 0,
            seq: 0,
            free: Vec::new(),
        }
    }

    /// Number of live items.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Insert `(key, val)`; returns the item's handle for
    /// [`decrease_key`](FibHeap::decrease_key).
    pub fn push(&mut self, key: u64, val: u32) -> u32 {
        let id = match self.free.pop() {
            Some(id) => {
                self.nodes[id as usize] = Node {
                    key,
                    seq: self.seq,
                    val,
                    parent: NONE,
                    child: NONE,
                    left: id,
                    right: id,
                    degree: 0,
                    marked: false,
                    live: true,
                };
                id
            }
            None => {
                let id = self.nodes.len() as u32;
                self.nodes.push(Node {
                    key,
                    seq: self.seq,
                    val,
                    parent: NONE,
                    child: NONE,
                    left: id,
                    right: id,
                    degree: 0,
                    marked: false,
                    live: true,
                });
                id
            }
        };
        self.seq += 1;
        self.len += 1;
        self.insert_root(id);
        if self.min == NONE
            || (key, self.nodes[id as usize].seq)
                < (
                    self.nodes[self.min as usize].key,
                    self.nodes[self.min as usize].seq,
                )
        {
            self.min = id;
        }
        id
    }

    /// The minimum `(key, val)` without popping.
    pub fn peek(&self) -> Option<(u64, u32)> {
        if self.min == NONE {
            None
        } else {
            let n = &self.nodes[self.min as usize];
            Some((n.key, n.val))
        }
    }

    /// Pop the minimum `(key, val)` — ties break on insertion order.
    /// Runs the degree-consolidation pass that gives Fibonacci heaps
    /// their amortized bound.
    pub fn pop(&mut self) -> Option<(u64, u32)> {
        if self.min == NONE {
            return None;
        }
        let z = self.min;
        let (key, _seq, val) = {
            let n = &self.nodes[z as usize];
            (n.key, n.seq, n.val)
        };
        // Move z's children up into the root list.
        let mut c = self.nodes[z as usize].child;
        let mut kids = Vec::new();
        if c != NONE {
            let start = c;
            loop {
                kids.push(c);
                self.nodes[c as usize].parent = NONE;
                let next = self.nodes[c as usize].right;
                c = next;
                if c == start {
                    break;
                }
            }
            self.nodes[z as usize].child = NONE;
            self.nodes[z as usize].degree = 0;
        }
        self.remove_from_ring(z);
        for &k in &kids {
            self.insert_root(k);
        }
        self.len -= 1;
        self.nodes[z as usize].live = false;
        if self.len == 0 {
            self.min = NONE;
            self.root_list = NONE;
        } else {
            self.consolidate();
        }
        self.free.push(z);
        Some((key, val))
    }

    /// Lower `handle`'s key to `new_key`; `false` if the handle is
    /// stale (popped or invalid) or `new_key` is not smaller.
    /// Cut-and-cascade.
    pub fn decrease_key(&mut self, handle: u32, new_key: u64) -> bool {
        let idx = handle as usize;
        if idx >= self.nodes.len() {
            return false;
        }
        if !self.nodes[idx].live || new_key >= self.nodes[idx].key {
            return false;
        }
        self.nodes[handle as usize].key = new_key;
        let p = self.nodes[handle as usize].parent;
        if p != NONE && self.nodes[handle as usize].key < self.nodes[p as usize].key {
            self.cut(handle, p);
            self.cascade(p);
        }
        if (
            self.nodes[handle as usize].key,
            self.nodes[handle as usize].seq,
        ) < (
            self.nodes[self.min as usize].key,
            self.nodes[self.min as usize].seq,
        ) {
            self.min = handle;
        }
        true
    }

    // ---------- internals ----------

    /// Splice `id` (currently in no ring) into the root ring.
    fn insert_root(&mut self, id: u32) {
        if self.root_list == NONE {
            self.nodes[id as usize].left = id;
            self.nodes[id as usize].right = id;
            self.root_list = id;
        } else {
            let r = self.root_list;
            let l = self.nodes[r as usize].left;
            self.nodes[id as usize].right = r;
            self.nodes[id as usize].left = l;
            self.nodes[l as usize].right = id;
            self.nodes[r as usize].left = id;
        }
        self.nodes[id as usize].parent = NONE;
        self.nodes[id as usize].marked = false;
    }

    /// Remove `id` from its sibling ring (parent's child head or
    /// root ring fixed up by the caller).
    fn remove_from_ring(&mut self, id: u32) {
        let (l, r) = (self.nodes[id as usize].left, self.nodes[id as usize].right);
        self.nodes[l as usize].right = r;
        self.nodes[r as usize].left = l;
        if self.root_list == id {
            self.root_list = if r == id { NONE } else { r };
        }
        // If id was some parent's child head, the parent must point
        // elsewhere (or NONE if the ring was singleton).
        let p = self.nodes[id as usize].parent;
        if p != NONE && self.nodes[p as usize].child == id {
            self.nodes[p as usize].child = if r == id { NONE } else { r };
        }
        self.nodes[id as usize].left = id;
        self.nodes[id as usize].right = id;
    }

    /// Link tree `y` under `x` (`y` leaves the root ring).
    fn link(&mut self, y: u32, x: u32) {
        self.remove_from_ring(y);
        self.nodes[y as usize].parent = x;
        // splice y under x's child ring
        let c = self.nodes[x as usize].child;
        if c == NONE {
            self.nodes[x as usize].child = y;
            self.nodes[y as usize].left = y;
            self.nodes[y as usize].right = y;
        } else {
            let l = self.nodes[c as usize].left;
            self.nodes[y as usize].right = c;
            self.nodes[y as usize].left = l;
            self.nodes[l as usize].right = y;
            self.nodes[c as usize].left = y;
        }
        self.nodes[x as usize].degree += 1;
        self.nodes[y as usize].marked = false;
    }

    /// Merge equal-degree roots until every degree is unique.
    fn consolidate(&mut self) {
        // First pass: count roots, rebuild min afterwards.
        let mut roots = Vec::new();
        if self.root_list == NONE {
            self.min = NONE;
            return;
        }
        let start = self.root_list;
        let mut cur = start;
        loop {
            roots.push(cur);
            cur = self.nodes[cur as usize].right;
            if cur == start {
                break;
            }
        }
        // degree -> root id; iterate roots snapshot (ring mutates).
        let mut by_degree: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
        self.root_list = NONE;
        for &w in &roots {
            // detach w first (its ring already broken by iteration order)
            self.nodes[w as usize].left = w;
            self.nodes[w as usize].right = w;
            let mut x = w;
            let mut d = self.nodes[x as usize].degree;
            while let Some(&y) = by_degree.get(&d) {
                // link larger-key under smaller-key (canonical (key,seq))
                let (a, b) = (self.nodes[x as usize].key, self.nodes[x as usize].seq);
                let (c, e) = (self.nodes[y as usize].key, self.nodes[y as usize].seq);
                let (small, big) = if (a, b) <= (c, e) { (x, y) } else { (y, x) };
                self.nodes[big as usize].left = big;
                self.nodes[big as usize].right = big;
                self.link(big, small);
                by_degree.remove(&d);
                x = small;
                d = self.nodes[x as usize].degree;
            }
            by_degree.insert(d, x);
        }
        // Rebuild the root ring over by_degree values.
        let mut first = NONE;
        self.min = NONE;
        for (&_d, &x) in by_degree.iter() {
            self.nodes[x as usize].left = x;
            self.nodes[x as usize].right = x;
            self.nodes[x as usize].parent = NONE;
            self.nodes[x as usize].marked = false;
            if first == NONE {
                first = x;
                self.root_list = x;
            } else {
                let l = self.nodes[first as usize].left;
                self.nodes[x as usize].right = first;
                self.nodes[x as usize].left = l;
                self.nodes[l as usize].right = x;
                self.nodes[first as usize].left = x;
            }
            if self.min == NONE
                || (self.nodes[x as usize].key, self.nodes[x as usize].seq)
                    < (
                        self.nodes[self.min as usize].key,
                        self.nodes[self.min as usize].seq,
                    )
            {
                self.min = x;
            }
        }
    }

    /// Cut `x` from parent `y` and promote to the root list.
    fn cut(&mut self, x: u32, y: u32) {
        self.remove_from_ring(x);
        self.nodes[y as usize].degree = self.nodes[y as usize].degree.saturating_sub(1);
        self.nodes[x as usize].parent = NONE;
        self.nodes[x as usize].marked = false;
        self.insert_root(x);
    }

    /// Cascading cut: if `y` is marked, cut it from its own parent.
    fn cascade(&mut self, y: u32) {
        let p = self.nodes[y as usize].parent;
        if p != NONE {
            if !self.nodes[y as usize].marked {
                self.nodes[y as usize].marked = true;
            } else {
                self.cut(y, p);
                self.cascade(p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn basics() {
        let mut h = FibHeap::new();
        assert!(h.is_empty());
        h.push(9, 1);
        h.push(1, 2);
        h.push(5, 3);
        assert_eq!(h.peek(), Some((1, 2)));
        assert_eq!(h.pop(), Some((1, 2)));
        assert_eq!(h.pop(), Some((5, 3)));
        assert_eq!(h.pop(), Some((9, 1)));
        assert_eq!(h.pop(), None);
    }

    #[test]
    fn decrease_key_moves_min() {
        let mut h = FibHeap::new();
        h.push(10, 1);
        let b = h.push(20, 2);
        h.push(30, 3);
        assert!(h.decrease_key(b, 5));
        assert_eq!(h.peek(), Some((5, 2)));
        assert_eq!(h.pop(), Some((5, 2)));
        assert!(!h.decrease_key(b, 0)); // stale handle after pop
    }

    #[test]
    fn interleaved_oracle() {
        // Shadow: BTreeMap<(key, seq)> == canonical FIFO tie order.
        let mut rng = SplitMix64::new(0xf1b0_9acc_5eed_beef);
        for _ in 0..30 {
            let mut h = FibHeap::new();
            let mut shadow: BTreeMap<(u64, u64), u32> = BTreeMap::new();
            let mut seq = 0u64;
            for _ in 0..800 {
                if shadow.is_empty() || rng.below(2) == 0 {
                    let key = rng.below(200) as u64;
                    let val = rng.below(1_000_000);
                    h.push(key, val);
                    shadow.insert((key, seq), val);
                    seq += 1;
                } else {
                    let got = h.pop();
                    let want = shadow.iter().next().map(|(k, v)| (k.0, *v));
                    assert_eq!(got, want);
                    if let Some((k, _)) = shadow.iter().next() {
                        let kk = *k;
                        shadow.remove(&kk);
                    }
                }
            }
            while let Some(kv) = h.pop() {
                let want = shadow.iter().next().map(|(k, v)| (k.0, *v));
                assert_eq!(Some(kv), want);
                if let Some((k, _)) = shadow.iter().next() {
                    let kk = *k;
                    shadow.remove(&kk);
                }
            }
        }
    }

    #[test]
    fn shadow_with_decrease() {
        // Track per-handle (key,seq) explicitly — exact oracle.
        let mut rng = SplitMix64::new(0x5ea5_ea5e_a5ea_5ea5);
        let mut h = FibHeap::new();
        let mut shadow: BTreeMap<(u64, u64), (u32, u32)> = BTreeMap::new(); // (key,seq)->(val,handle)
        let mut by_handle: BTreeMap<u32, (u64, u64)> = BTreeMap::new(); // handle->(key,seq)
        let mut seq = 0u64;
        for _ in 0..3000 {
            match rng.below(10) {
                0..=3 => {
                    let key = rng.below(500) as u64;
                    let val = rng.below(1_000_000);
                    let hd = h.push(key, val);
                    shadow.insert((key, seq), (val, hd));
                    by_handle.insert(hd, (key, seq));
                    seq += 1;
                }
                4 | 5 => {
                    if let Some((&hd, &(k, s))) = by_handle
                        .iter()
                        .nth((rng.next_u64() as usize) % by_handle.len().max(1))
                    {
                        if k > 0 {
                            let nk = rng.below(k as u32) as u64;
                            if h.decrease_key(hd, nk) {
                                let (val, _) = shadow.remove(&(k, s)).unwrap();
                                shadow.insert((nk, s), (val, hd));
                                by_handle.insert(hd, (nk, s));
                            } else {
                                // pop'd handle — drop bookkeeping
                                by_handle.remove(&hd);
                            }
                        }
                    }
                }
                _ => {
                    let got = h.pop();
                    let want = shadow.iter().next().map(|(k, v)| (k.0, v.0));
                    assert_eq!(got, want);
                    if let Some((k, v)) = shadow.iter().next() {
                        let (kk, hd) = (*k, v.1);
                        shadow.remove(&kk);
                        by_handle.remove(&hd);
                    }
                }
            }
        }
        while let Some(kv) = h.pop() {
            let want = shadow.iter().next().map(|(k, v)| (k.0, v.0));
            assert_eq!(Some(kv), want);
            if let Some((k, _)) = shadow.iter().next() {
                let kk = *k;
                shadow.remove(&kk);
            }
        }
    }

    #[test]
    fn consolidation_structure() {
        // After pops, degree-unique invariant holds — probe via a full
        // pop sequence being monotone nondecreasing.
        let mut rng = SplitMix64::new(1);
        let mut h = FibHeap::new();
        for _ in 0..500 {
            h.push(rng.next_u64(), rng.below(100));
        }
        let mut last = 0;
        while let Some((k, _)) = h.pop() {
            assert!(k >= last);
            last = k;
        }
    }
}

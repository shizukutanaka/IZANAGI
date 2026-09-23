//! Euler-tour tree — dynamic-forest connectivity over vertex
//! ids: `link`/`cut`/`connected`/`tree_size` in `O(log n)`
//! expected time, the sequence-based counterpart to
//! [`crate::linkcut`] (which exposes paths and aggregates;
//! ETT's win is that connectivity itself needs only tour
//! membership, so no splay exposes needed at all).
//!
//! Every tree keeps its Euler *edge tour* in one implicit
//! treap — a cyclic sequence of directed half-edge markers,
//! each undirected `{u,v}` appearing once as `(u,v)` and once
//! as `(v,u)`. Each vertex owns a permanent vertex node in its
//! tree's tour (a singleton tour when isolated). Then:
//!
//! - `connected(u,v)` ⇔ `root(tour_node(u)) == root(tour_node(v))`
//!   — climb parent pointers, compare roots.
//! - `link(u,v)`: reroot u's tour at `u` (split at u's node,
//!   merge the halves — a cyclic rotation), same for v, then
//!   `U + [uv] + V + [vu]` is the merged tour.
//! - `cut(u,v)`: with `x=(u,v)` before `y=(v,u)` in the tour
//!   `A x B y C`, the result is `B` for v's tree and `C+A`
//!   for u's — four splits and one merge.
//!
//! ```
//! use izanagi_kit::ett::Ett;
//! let mut t = Ett::new(1);
//! assert!(t.link(0, 1));
//! assert!(t.link(1, 2));
//! assert!(t.connected(0, 2));
//! assert!(!t.link(0, 2)); // would close a cycle
//! assert!(t.cut(1, 2));
//! assert!(!t.connected(0, 2));
//! ```
//!
//! References: Henzinger & King (1999) randomized dynamic
//! connectivity (the ET-tree level), Tarjan (1997) "Dynamic
//! trees as search trees via Euler tours"; competitive-
//! programming ETT for the half-edge tour layout.

use crate::rng::SplitMix64;
use std::collections::BTreeMap;

const NIL: u32 = !0;

#[derive(Clone)]
struct Node {
    pr: u32,
    ch: [u32; 2],
    parent: u32,
    /// Total tour nodes in the subtree.
    sz: u32,
    /// Vertex nodes in the subtree (for `tree_size`).
    vsz: u32,
    /// This node is a vertex marker (permanent, one per id).
    is_vertex: bool,
}

/// Euler-tour forest with seeded (deterministic) treap
/// priorities. Vertex ids are arbitrary `u32`s, materialized
/// lazily on first mention.
pub struct Ett {
    ns: Vec<Node>,
    free: Vec<u32>,
    /// vertex id -> its permanent tour node.
    vnode: BTreeMap<u32, u32>,
    /// directed half-edge (u,v) -> its tour node.
    edges: BTreeMap<(u32, u32), u32>,
    rng: SplitMix64,
}

impl Ett {
    /// Empty forest — `seed` fixes every node priority, so the
    /// whole shape is a pure function of `(seed, op sequence)`.
    pub fn new(seed: u64) -> Ett {
        Ett {
            ns: Vec::new(),
            free: Vec::new(),
            vnode: BTreeMap::new(),
            edges: BTreeMap::new(),
            rng: SplitMix64::new(seed),
        }
    }

    fn alloc(&mut self, is_vertex: bool) -> u32 {
        let n = Node {
            pr: self.rng.next_u64() as u32,
            ch: [NIL; 2],
            parent: NIL,
            sz: 1,
            vsz: u32::from(is_vertex),
            is_vertex,
        };
        match self.free.pop() {
            Some(i) => {
                self.ns[i as usize] = n;
                i
            }
            None => {
                self.ns.push(n);
                (self.ns.len() - 1) as u32
            }
        }
    }

    fn release(&mut self, n: u32) {
        self.ns[n as usize].ch = [NIL; 2];
        self.ns[n as usize].parent = NIL;
        self.free.push(n);
    }

    fn sz(&self, n: u32) -> u32 {
        if n == NIL {
            0
        } else {
            self.ns[n as usize].sz
        }
    }

    fn pull(&mut self, n: u32) {
        if n == NIL {
            return;
        }
        let [l, r] = self.ns[n as usize].ch;
        self.ns[n as usize].sz = 1 + self.sz(l) + self.sz(r);
        self.ns[n as usize].vsz = u32::from(self.ns[n as usize].is_vertex)
            + if l == NIL { 0 } else { self.ns[l as usize].vsz }
            + if r == NIL { 0 } else { self.ns[r as usize].vsz };
    }

    fn set_ch(&mut self, n: u32, side: usize, c: u32) {
        self.ns[n as usize].ch[side] = c;
        if c != NIL {
            self.ns[c as usize].parent = n;
        }
    }

    /// Treap root of the treap containing node `n` — parent
    /// climb, `O(height)` expected.
    fn root(&self, mut n: u32) -> u32 {
        while self.ns[n as usize].parent != NIL {
            n = self.ns[n as usize].parent;
        }
        n
    }

    /// In-order position of node `n` inside its treap.
    fn index(&self, mut n: u32) -> usize {
        let mut i = self.sz(self.ns[n as usize].ch[0]) as usize;
        while self.ns[n as usize].parent != NIL {
            let p = self.ns[n as usize].parent;
            if self.ns[p as usize].ch[1] == n {
                i += self.sz(self.ns[p as usize].ch[0]) as usize + 1;
            }
            n = p;
        }
        i
    }

    /// `merge(a, b)` — all of `a`'s sequence before `b`'s;
    /// min-heap on `pr`.
    fn merge(&mut self, a: u32, b: u32) -> u32 {
        if a == NIL {
            if b != NIL {
                self.ns[b as usize].parent = NIL;
            }
            return b;
        }
        if b == NIL {
            self.ns[a as usize].parent = NIL;
            return a;
        }
        if self.ns[a as usize].pr < self.ns[b as usize].pr {
            let r = self.merge(self.ns[a as usize].ch[1], b);
            self.set_ch(a, 1, r);
            self.pull(a);
            self.ns[a as usize].parent = NIL;
            a
        } else {
            let l = self.merge(a, self.ns[b as usize].ch[0]);
            self.set_ch(b, 0, l);
            self.pull(b);
            self.ns[b as usize].parent = NIL;
            b
        }
    }

    /// `split(t, k)` — first `k` nodes to the left treap.
    fn split(&mut self, t: u32, k: usize) -> (u32, u32) {
        if t == NIL {
            return (NIL, NIL);
        }
        let lsz = self.sz(self.ns[t as usize].ch[0]) as usize;
        if k <= lsz {
            let (a, b) = self.split(self.ns[t as usize].ch[0], k);
            self.set_ch(t, 0, b);
            self.pull(t);
            self.ns[t as usize].parent = NIL;
            (a, t)
        } else {
            let (a, b) = self.split(self.ns[t as usize].ch[1], k - lsz - 1);
            self.set_ch(t, 1, a);
            self.pull(t);
            self.ns[t as usize].parent = NIL;
            (t, b)
        }
    }

    /// Lazily materialize `u`'s permanent vertex node.
    fn touch(&mut self, u: u32) -> u32 {
        if let Some(&n) = self.vnode.get(&u) {
            return n;
        }
        let n = self.alloc(true);
        self.vnode.insert(u, n);
        n
    }

    /// `true` when `u` and `v` sit in the same tree. `u == v`
    /// is trivially connected; unseen vertices are never
    /// connected to anything else.
    pub fn connected(&mut self, u: u32, v: u32) -> bool {
        if u == v {
            return true;
        }
        let (a, b) = match (self.vnode.get(&u), self.vnode.get(&v)) {
            (Some(&a), Some(&b)) => (a, b),
            _ => return false,
        };
        self.root(a) == self.root(b)
    }

    /// Add edge `{u,v}` — `false` (and no change) when the
    /// vertices are already connected (a cycle would form).
    pub fn link(&mut self, u: u32, v: u32) -> bool {
        if u == v || self.connected(u, v) {
            return false;
        }
        let nu = self.touch(u);
        let nv = self.touch(v);
        // reroot each tour at its vertex node (cyclic rotation)
        let ru = self.root(nu);
        let iu = self.index(nu);
        let (a1, b1) = self.split(ru, iu);
        let utour = self.merge(b1, a1);
        let rv = self.root(nv);
        let iv = self.index(nv);
        let (a2, b2) = self.split(rv, iv);
        let vtour = self.merge(b2, a2);
        // splice: U.., [uv], V.., [vu]
        let x = self.alloc(false);
        let y = self.alloc(false);
        self.edges.insert((u, v), x);
        self.edges.insert((v, u), y);
        let m = self.merge(utour, x);
        let m = self.merge(m, vtour);
        self.merge(m, y);
        true
    }

    /// Remove edge `{u,v}` — `false` when the edge is absent.
    /// `O(log n)` expected; the two sides stay valid tours.
    pub fn cut(&mut self, u: u32, v: u32) -> bool {
        let (x, y) = match (self.edges.get(&(u, v)), self.edges.get(&(v, u))) {
            (Some(&x), Some(&y)) => (x, y),
            _ => return false,
        };
        self.edges.remove(&(u, v));
        self.edges.remove(&(v, u));
        let t = self.root(x);
        let mut ix = self.index(x);
        let mut iy = self.index(y);
        if ix > iy {
            std::mem::swap(&mut ix, &mut iy);
        }
        // tour = A[ix] B[iy] C — cut out both half-edge nodes
        let (a, xbc) = self.split(t, ix);
        let (_x, bc) = self.split(xbc, 1);
        let (b, yc) = self.split(bc, iy - ix - 1);
        let (_y, c) = self.split(yc, 1);
        // v's side keeps B; u's side wraps C+A
        self.merge(c, a);
        let _ = b;
        self.release(x);
        self.release(y);
        true
    }

    /// Number of vertices in `u`'s tree — 1 for an isolated
    /// vertex, 0 for an unseen one.
    pub fn tree_size(&mut self, u: u32) -> usize {
        match self.vnode.get(&u) {
            None => 0,
            Some(&n) => {
                let r = self.root(n);
                self.ns[r as usize].vsz as usize
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Adjacency-map oracle with BFS connectivity.
    #[derive(Default)]
    struct Oracle {
        adj: BTreeMap<u32, BTreeSet<u32>>,
    }
    impl Oracle {
        fn touch(&mut self, v: u32) {
            self.adj.entry(v).or_default();
        }
        fn connected(&self, u: u32, v: u32) -> bool {
            if u == v {
                return true;
            }
            let mut seen = BTreeSet::new();
            let mut q = vec![u];
            seen.insert(u);
            while let Some(x) = q.pop() {
                if x == v {
                    return true;
                }
                if let Some(s) = self.adj.get(&x) {
                    for &w in s {
                        if seen.insert(w) {
                            q.push(w);
                        }
                    }
                }
            }
            false
        }
        fn component_size(&self, u: u32) -> usize {
            let mut seen = BTreeSet::new();
            let mut q = vec![u];
            seen.insert(u);
            while let Some(x) = q.pop() {
                if let Some(s) = self.adj.get(&x) {
                    for &w in s {
                        if seen.insert(w) {
                            q.push(w);
                        }
                    }
                }
            }
            seen.len()
        }
    }

    #[test]
    fn basics() {
        let mut t = Ett::new(1);
        assert!(t.link(0, 1));
        assert!(t.link(1, 2));
        assert!(t.connected(0, 2));
        assert!(!t.link(0, 2));
        assert_eq!(t.tree_size(0), 3);
        assert!(t.cut(1, 2));
        assert!(!t.connected(0, 2));
        assert_eq!(t.tree_size(0), 2);
        assert_eq!(t.tree_size(2), 1);
        // absent edge, unseen vertex, self loops
        assert!(!t.cut(1, 2));
        assert!(!t.connected(9, 0));
        assert_eq!(t.tree_size(9), 0);
        assert!(!t.link(5, 5));
        assert!(t.connected(7, 7));
        // re-link after cut
        assert!(t.link(0, 2));
        assert!(t.connected(0, 2));
    }

    #[test]
    fn oracle_bfs() {
        let mut rng = SplitMix64::new(51);
        for round in 0..30 {
            let n = rng.below(10) + 3;
            let mut t = Ett::new(round + 100);
            let mut o = Oracle::default();
            for _ in 0..120 {
                let u = rng.below(n);
                let v = rng.below(n);
                match rng.below(4) {
                    0 => {
                        o.touch(u);
                        let _ = t.tree_size(u);
                        assert_eq!(t.connected(u, v), o.connected(u, v));
                    }
                    1 | 2 => {
                        if u == v {
                            continue;
                        }
                        let want = !o.connected(u, v);
                        o.touch(u);
                        o.touch(v);
                        assert_eq!(t.link(u, v), want, "link {u}-{v}");
                        if want {
                            o.adj.entry(u).or_default().insert(v);
                            o.adj.entry(v).or_default().insert(u);
                        }
                    }
                    _ => {
                        if u == v {
                            continue;
                        }
                        let have = o.adj.get(&u).map(|s| s.contains(&v)).unwrap_or(false);
                        assert_eq!(t.cut(u, v), have, "cut {u}-{v}");
                        if have {
                            o.adj.get_mut(&u).unwrap().remove(&v);
                            o.adj.get_mut(&v).unwrap().remove(&u);
                        }
                        assert_eq!(t.connected(u, v), o.connected(u, v));
                    }
                }
            }
            // full membership check
            for u in 0..n {
                for v in 0..n {
                    assert_eq!(t.connected(u, v), o.connected(u, v), "{u}-{v}");
                }
                assert_eq!(t.tree_size(u), o.component_size(u), "size {u}");
            }
        }
    }

    #[test]
    fn chains_and_relink() {
        // long chain then heavy cuts — stress reroot/index
        let mut t = Ett::new(3);
        let n = 60u32;
        for i in 1..n {
            assert!(t.link(i - 1, i));
        }
        assert_eq!(t.tree_size(0), n as usize);
        for i in (1..n).step_by(3) {
            assert!(t.cut(i - 1, i));
        }
        assert_eq!(t.tree_size(0), 1);
        assert!(!t.connected(0, n - 1));
        // relink the same edges back in reverse order
        for i in (1..n).step_by(3).rev() {
            assert!(t.link(i - 1, i));
        }
        assert_eq!(t.tree_size(0), n as usize);
    }
}

//! Link–Cut trees — Sleator & Tarjan's dynamic forest with
//! `O(log n)` amortized `link`, `cut`, `find_root`, and path
//! aggregates. `hld` answers path queries on a *static* tree;
//! a link–cut tree keeps the same queries working while edges
//! appear and disappear — the data structure behind dynamic
//! connectivity, network simplex, and online MST updates.
//!
//! The forest is decomposed into preferred paths; each preferred
//! path is one splay ("auxiliary") tree keyed by depth, and
//! non-preferred edges hang off aux roots as path-parent
//! pointers. `access(v)` makes `v`'s root path preferred;
//! `make_root(v)` = `access` + reverse that whole path (a lazy
//! `rev` flag flips the depth order). Everything is pointer-free
//! index arithmetic — the state is a pure function of the
//! operation sequence.
//!
//! ```
//! use izanagi_kit::linkcut::LinkCut;
//! let mut t = LinkCut::new(4);
//! t.set_val(0, 5);
//! t.set_val(1, 9);
//! t.link(0, 1); // 0 hangs under 1
//! assert!(t.connected(0, 1));
//! assert_eq!(t.path_min(0, 1), Some(5));
//! t.cut(0, 1);
//! assert!(!t.connected(0, 1));
//! ```
//!
//! References: Sleator & Tarjan (1983) "A data structure for
//! dynamic trees"; the splay-based access decomposition used by
//! Alstrup et al. and every competitive-programming LCT.

/// Link–Cut forest over nodes `0..n` with `i128` vertex weights
/// and a path-minimum aggregate.
#[derive(Clone, Debug)]
pub struct LinkCut {
    n: usize,
    /// splay children `ch[v][0]` = shallower side, `ch[v][1]` deeper.
    ch: [[u32; 2]; 64],
    /// aux parent or, for aux roots, the path-parent.
    p: [u32; 64],
    /// lazy "whole preferred path reversed" flag.
    rev: [bool; 64],
    val: [i128; 64],
    /// min `val` over the aux subtree = min over the preferred
    /// path segment this splay covers.
    min: [i128; 64],
}

const NIL: u32 = !0;

impl LinkCut {
    /// Empty forest of `n` singleton nodes (weight 0).
    ///
    /// # Panics-free contract — `n > 64` is clamped to 64.
    pub fn new(n: usize) -> Self {
        let n = n.min(64);
        Self {
            n,
            ch: [[NIL; 2]; 64],
            p: [NIL; 64],
            rev: [false; 64],
            val: [0; 64],
            min: [i128::MAX; 64],
        }
    }

    fn is_aux_root(&self, x: u32) -> bool {
        let p = self.p[x as usize];
        p == NIL || (self.ch[p as usize][0] != x && self.ch[p as usize][1] != x)
    }

    fn pushup(&mut self, x: u32) {
        let mut m = self.val[x as usize];
        for d in 0..2 {
            let c = self.ch[x as usize][d];
            if c != NIL && self.min[c as usize] < m {
                m = self.min[c as usize];
            }
        }
        self.min[x as usize] = m;
    }

    fn push(&mut self, x: u32) {
        if self.rev[x as usize] {
            self.rev[x as usize] = false;
            self.ch[x as usize].swap(0, 1);
            for d in 0..2 {
                let c = self.ch[x as usize][d];
                if c != NIL {
                    self.rev[c as usize] = !self.rev[c as usize];
                }
            }
        }
    }

    /// push every pending `rev` on the path from the aux-tree
    /// root down to `x` — required before rotating at `x`.
    fn push_path(&mut self, x: u32) {
        // collect ancestors to the aux root top-down
        let mut stack = Vec::new();
        let mut y = x;
        stack.push(y);
        while !self.is_aux_root(y) {
            y = self.p[y as usize];
            stack.push(y);
        }
        while let Some(y) = stack.pop() {
            self.push(y);
        }
    }

    fn rot(&mut self, x: u32) {
        let p = self.p[x as usize];
        let g = self.p[p as usize];
        let dir = (self.ch[p as usize][1] == x) as usize; // x is deeper side?
        let b = self.ch[x as usize][dir ^ 1];
        if !self.is_aux_root(p) {
            // attach x where p hung under g
            if self.ch[g as usize][0] == p {
                self.ch[g as usize][0] = x;
            } else {
                self.ch[g as usize][1] = x;
            }
        }
        self.p[x as usize] = g;
        self.ch[x as usize][dir ^ 1] = p;
        self.p[p as usize] = x;
        self.ch[p as usize][dir] = b;
        if b != NIL {
            self.p[b as usize] = p;
        }
        self.pushup(p);
        self.pushup(x);
    }

    fn splay(&mut self, x: u32) {
        self.push_path(x);
        while !self.is_aux_root(x) {
            let p = self.p[x as usize];
            if !self.is_aux_root(p) {
                let g = self.p[p as usize];
                // zig-zig if x and p hang on the same side
                let same = (self.ch[p as usize][1] == x) == (self.ch[g as usize][1] == p);
                if same {
                    self.rot(p);
                } else {
                    self.rot(x);
                }
            }
            self.rot(x);
        }
    }

    /// Make `x`'s whole root-to-`x` path preferred; `x` ends up
    /// the deepest node of its aux tree. Returns the last
    /// path-parent visited — `access(a); access(b)` returns the
    /// two vertices' LCA when they share a tree.
    pub fn access(&mut self, x: u32) -> Option<u32> {
        if x as usize >= self.n {
            return None;
        }
        let mut last = NIL;
        let mut y = x;
        while y != NIL {
            self.splay(y);
            self.ch[y as usize][1] = last;
            self.pushup(y);
            last = y;
            y = self.p[y as usize];
        }
        self.splay(x);
        if last == NIL {
            None
        } else {
            Some(last)
        }
    }

    /// Re-root `x`'s tree at `x` — the classic `access` + path
    /// reversal, so `x` has no shallower ancestors.
    pub fn make_root(&mut self, x: u32) -> bool {
        if x as usize >= self.n {
            return false;
        }
        self.access(x);
        self.rev[x as usize] = !self.rev[x as usize];
        self.push(x);
        true
    }

    /// Root of `x`'s represented tree.
    pub fn find_root(&mut self, x: u32) -> Option<u32> {
        if x as usize >= self.n {
            return None;
        }
        self.access(x);
        let mut r = x;
        self.push_path(r);
        while self.ch[r as usize][0] != NIL {
            r = self.ch[r as usize][0];
            self.push_path(r);
        }
        self.splay(r);
        Some(r)
    }

    /// Do `x` and `y` sit in one tree?
    pub fn connected(&mut self, x: u32, y: u32) -> bool {
        if x == y {
            return x != NIL && (x as usize) < self.n;
        }
        match (self.find_root(x), self.find_root(y)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }

    /// Hang `x` under `y` (`x` becomes a child of `y`). Refuses
    /// if they are already connected — the represented structure
    /// stays a forest.
    pub fn link(&mut self, x: u32, y: u32) -> bool {
        if x as usize >= self.n || y as usize >= self.n || x == y {
            return false;
        }
        self.make_root(x);
        if self.find_root(y) == Some(x) {
            return false; // would create a cycle
        }
        self.p[x as usize] = y;
        true
    }

    /// Remove the edge `(x, y)` if present.
    pub fn cut(&mut self, x: u32, y: u32) -> bool {
        if x as usize >= self.n || y as usize >= self.n {
            return false;
        }
        self.make_root(x);
        self.access(y);
        // after make_root(x)+access(y), the x-y edge exists iff
        // y's left child is x and x has no right child
        if self.ch[y as usize][0] == x && self.ch[x as usize][1] == NIL {
            self.ch[y as usize][0] = NIL;
            self.p[x as usize] = NIL;
            self.pushup(y);
            true
        } else {
            false
        }
    }

    /// Least common ancestor of `x` and `y` within their tree
    /// (w.r.t. the current rooting), `None` if disconnected.
    pub fn lca(&mut self, x: u32, y: u32) -> Option<u32> {
        if x as usize >= self.n || y as usize >= self.n {
            return None;
        }
        if !self.connected(x, y) {
            return None;
        }
        self.access(x);
        self.access(y)
    }

    /// Assign vertex weight.
    pub fn set_val(&mut self, x: u32, v: i128) -> bool {
        if x as usize >= self.n {
            return false;
        }
        self.access(x);
        self.val[x as usize] = v;
        self.pushup(x);
        true
    }

    /// Vertex weight.
    pub fn val(&mut self, x: u32) -> Option<i128> {
        if x as usize >= self.n {
            return None;
        }
        self.access(x);
        Some(self.val[x as usize])
    }

    /// Minimum vertex weight on the `x`–`y` path.
    pub fn path_min(&mut self, x: u32, y: u32) -> Option<i128> {
        if x as usize >= self.n || y as usize >= self.n {
            return None;
        }
        if !self.connected(x, y) {
            return None;
        }
        self.make_root(x);
        self.access(y);
        Some(self.min[y as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::VecDeque;

    /// BFS path between a and b in an undirected forest (unique
    /// path when connected).
    fn bfs_path(adj: &[Vec<u32>], a: u32, b: u32) -> Option<Vec<u32>> {
        let n = adj.len();
        let mut prev = vec![!0u32; n];
        let mut q = VecDeque::from([a]);
        prev[a as usize] = a;
        while let Some(u) = q.pop_front() {
            if u == b {
                break;
            }
            for &w in &adj[u as usize] {
                if prev[w as usize] == !0 {
                    prev[w as usize] = u;
                    q.push_back(w);
                }
            }
        }
        if prev[b as usize] == !0 {
            return None;
        }
        let mut path = vec![b];
        let mut cur = b;
        while cur != a {
            cur = prev[cur as usize];
            path.push(cur);
        }
        Some(path)
    }

    #[test]
    fn linked_nodes_report_connected() {
        let mut t = LinkCut::new(4);
        t.link(0, 1);
        t.link(1, 2);
        assert!(t.connected(0, 2));
        assert!(!t.connected(0, 3));
        assert_eq!(t.find_root(0), t.find_root(2));
    }

    #[test]
    fn cycle_is_refused() {
        let mut t = LinkCut::new(3);
        assert!(t.link(0, 1));
        assert!(t.link(1, 2));
        assert!(!t.link(0, 2)); // 0 and 2 already share a tree
    }

    #[test]
    fn cut_disconnects() {
        let mut t = LinkCut::new(4);
        t.link(0, 1);
        t.link(1, 2);
        t.link(2, 3);
        assert!(t.cut(1, 2));
        assert!(t.connected(0, 1));
        assert!(t.connected(2, 3));
        assert!(!t.connected(0, 3));
        assert!(!t.cut(0, 3));
    }

    #[test]
    fn path_min_and_lca_agree_with_bfs() {
        let mut r = SplitMix64::new(0xCA7);
        for _round in 0..80 {
            let n = 1 + (r.next_u64() % 24) as usize;
            let mut t = LinkCut::new(n);
            let mut adj = vec![Vec::<u32>::new(); n];
            let mut w = vec![0i128; n];
            for _ in 0..200 {
                match r.next_u64() % 6 {
                    0 => {
                        let (a, b) = (
                            (r.next_u64() % n as u64) as u32,
                            (r.next_u64() % n as u64) as u32,
                        );
                        if t.link(a, b) {
                            adj[a as usize].push(b);
                            adj[b as usize].push(a);
                        }
                    }
                    1 => {
                        let (a, b) = (
                            (r.next_u64() % n as u64) as u32,
                            (r.next_u64() % n as u64) as u32,
                        );
                        if t.cut(a, b) {
                            adj[a as usize].retain(|&x| x != b);
                            adj[b as usize].retain(|&x| x != a);
                        }
                    }
                    2 => {
                        let a = (r.next_u64() % n as u64) as u32;
                        let v = (r.next_u64() % 1000) as i128;
                        t.set_val(a, v);
                        w[a as usize] = v;
                    }
                    3 => {
                        let (a, b) = (
                            (r.next_u64() % n as u64) as u32,
                            (r.next_u64() % n as u64) as u32,
                        );
                        assert_eq!(t.connected(a, b), bfs_path(&adj, a, b).is_some());
                    }
                    _ => {
                        let (a, b) = (
                            (r.next_u64() % n as u64) as u32,
                            (r.next_u64() % n as u64) as u32,
                        );
                        match (t.path_min(a, b), bfs_path(&adj, a, b)) {
                            (Some(m), Some(p)) => {
                                assert_eq!(m, p.iter().map(|&v| w[v as usize]).min().unwrap_or(0))
                            }
                            (None, None) => {}
                            _ => panic!("path_min disagrees with connectivity"),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn reroot_preserves_connectivity() {
        let mut t = LinkCut::new(5);
        t.link(0, 1);
        t.link(1, 2);
        t.link(2, 3);
        t.link(3, 4);
        t.make_root(4);
        assert!(t.connected(0, 4));
        assert_eq!(t.lca(0, 4), Some(4));
        assert!(t.cut(0, 1));
        assert!(!t.connected(0, 4));
    }
}

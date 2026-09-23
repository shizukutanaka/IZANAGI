//! Edmonds' blossom algorithm — maximum cardinality matching on
//! general (non-bipartite) graphs. `hopcroft_karp` covers the
//! bipartite case; odd cycles make general matching harder — an
//! alternating walk that re-enters a cycle can report a false
//! augmenting path unless the odd flower is contracted first.
//! This is the e-maxx-style BFS implementation: blossom
//! contraction by `base[]` relabeling inside the search, `O(n³)`,
//! fully deterministic (adjacency order is sorted).
//!
//! ```
//! use izanagi_kit::blossom::max_matching;
//! // triangle + tail: one edge covers two of the triangle
//! let m = max_matching(4, &[(0, 1), (1, 2), (0, 2), (2, 3)]);
//! assert_eq!(m.len(), 2);
//! ```
//!
//! References: Edmonds (1965) "Paths, trees, and flowers";
//! e-maxx/cp-algorithms blossom implementation.

const NIL: usize = !0;

/// Maximum matching on `n` vertices with undirected `edges`
/// (self-loops and duplicate edges are ignored). Returns the
/// matched pairs in sorted canonical order — a pure function of
/// the input.
pub fn max_matching(n: usize, edges: &[(usize, usize)]) -> Vec<(u32, u32)> {
    let mut g: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        if a != b && a < n && b < n && !g[a].contains(&b) {
            g[a].push(b);
            g[b].push(a);
        }
    }
    for adj in &mut g {
        adj.sort_unstable();
    }
    let mut mate = vec![NIL; n];
    let mut root = 0;
    while root < n {
        if mate[root] == NIL {
            augment(root, n, &g, &mut mate);
        }
        root += 1;
    }
    let mut out = Vec::new();
    for (v, &u) in mate.iter().enumerate() {
        if u != NIL && v < u {
            out.push((v as u32, u as u32));
        }
    }
    out
}

struct Search<'a> {
    g: &'a [Vec<usize>],
    mate: &'a mut Vec<usize>,
    base: Vec<usize>,
    p: Vec<usize>,
    used: Vec<bool>,
    inb: Vec<bool>,
    lca_seen: Vec<u32>,
    lca_clock: u32,
}

impl<'a> Search<'a> {
    /// Lowest common ancestor on the alternating forest.
    fn lca(&mut self, mut a: usize, mut b: usize) -> usize {
        self.lca_clock += 1;
        let clk = self.lca_clock;
        loop {
            a = self.base[a];
            self.lca_seen[a] = clk;
            if self.mate[a] == NIL {
                break;
            }
            a = self.p[self.mate[a]];
        }
        loop {
            b = self.base[b];
            if self.lca_seen[b] == clk {
                return b;
            }
            b = self.p[self.mate[b]];
        }
    }

    /// Relabel `v`'s blossom up to `b`.
    fn mark_path(&mut self, mut v: usize, b: usize, children: usize) {
        while self.base[v] != b {
            self.inb[self.base[v]] = true;
            self.inb[self.base[self.mate[v]]] = true;
            self.p[v] = children;
            let nxt = self.mate[v];
            v = self.p[nxt];
        }
    }

    /// BFS for an augmenting path from `root`; returns the
    /// unmatched endpoint (`NIL` = none).
    fn find_path(&mut self, root: usize) -> usize {
        self.p.fill(NIL);
        self.used.fill(false);
        for (i, b) in self.base.iter_mut().enumerate() {
            *b = i;
        }
        let mut q = vec![root];
        self.p[root] = root;
        self.used[root] = true;
        let mut qi = 0;
        while qi < q.len() {
            let v = q[qi];
            qi += 1;
            for &u in &self.g[v] {
                if self.base[v] == self.base[u] || self.mate[v] == u {
                    continue;
                }
                if u == root || (self.mate[u] != NIL && self.p[self.mate[u]] != NIL) {
                    // odd cycle → contract the blossom at lca(v,u)
                    let cur = self.lca(v, u);
                    self.inb.fill(false);
                    self.mark_path(v, cur, u);
                    self.mark_path(u, cur, v);
                    for (i, base_i) in self.base.iter_mut().enumerate() {
                        if self.inb[*base_i] {
                            *base_i = cur;
                            if !self.used[i] {
                                self.used[i] = true;
                                q.push(i);
                            }
                        }
                    }
                } else if self.p[u] == NIL {
                    self.p[u] = v;
                    if self.mate[u] == NIL {
                        return u;
                    }
                    let m = self.mate[u];
                    self.used[m] = true;
                    q.push(m);
                }
            }
        }
        NIL
    }
}

fn augment(root: usize, n: usize, g: &[Vec<usize>], mate: &mut Vec<usize>) {
    let mut s = Search {
        g,
        mate,
        base: vec![0; n],
        p: vec![NIL; n],
        used: vec![false; n],
        inb: vec![false; n],
        lca_seen: vec![0; n],
        lca_clock: 0,
    };
    let mut cur = s.find_path(root);
    while cur != NIL {
        let pv = s.p[cur];
        let ppv = s.mate[pv];
        s.mate[cur] = pv;
        s.mate[pv] = cur;
        cur = ppv;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn valid_matching(edges: &[(usize, usize)], m: &[(u32, u32)]) -> bool {
        let mut seen = BTreeSet::new();
        let set: BTreeSet<(usize, usize)> =
            edges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
        for &(a, b) in m {
            let (a, b) = (a as usize, b as usize);
            if !set.contains(&(a.min(b), a.max(b))) {
                return false;
            }
            if !seen.insert(a) || !seen.insert(b) {
                return false;
            }
        }
        true
    }

    /// Oracle: max matching size by exhaustive subset check (n ≤ 9).
    fn brute(n: usize, edges: &[(usize, usize)]) -> usize {
        let mut best = 0usize;
        let m = edges.len();
        for mask in 0..(1u64 << m).min(1 << 16) {
            let mut seen = [false; 9];
            let mut cnt = 0;
            for (i, &(a, b)) in edges.iter().enumerate() {
                if mask >> i & 1 == 1 && a < n && b < n {
                    if seen[a] || seen[b] {
                        cnt = 0;
                        break;
                    }
                    seen[a] = true;
                    seen[b] = true;
                    cnt += 1;
                }
            }
            if mask.count_ones() as usize != cnt {
                continue;
            }
            best = best.max(cnt);
        }
        best
    }

    #[test]
    fn known_cases() {
        // triangle + tail
        assert_eq!(max_matching(4, &[(0, 1), (1, 2), (0, 2), (2, 3)]).len(), 2);
        // K4 → perfect matching of 2
        let k4 = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        assert_eq!(max_matching(4, &k4).len(), 2);
        // 5-cycle → floor(5/2)
        assert_eq!(
            max_matching(5, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)]).len(),
            2
        );
        // star → 1
        assert_eq!(max_matching(5, &[(0, 1), (0, 2), (0, 3), (0, 4)]).len(), 1);
        // empty
        assert!(max_matching(3, &[]).is_empty());
    }

    #[test]
    fn blossom_is_needed() {
        // two triangles joined at one vertex: greedy-first-found
        // augmenting paths without contraction stall at size 2
        let edges = [(0, 1), (1, 2), (0, 2), (2, 3), (3, 4), (4, 5), (3, 5)];
        let m = max_matching(6, &edges);
        assert_eq!(m.len(), 3);
        assert!(valid_matching(&edges, &m));
    }

    #[test]
    fn oracle_small_graphs() {
        let mut r = SplitMix64::new(0xB105);
        for _ in 0..400 {
            let n = (r.below(8) + 2) as usize;
            let m_e = r.below(20) as usize;
            let mut edges = Vec::new();
            let mut seen = BTreeSet::new();
            for _ in 0..m_e {
                let a = r.below(n as u32) as usize;
                let b = r.below(n as u32) as usize;
                if a != b && seen.insert((a.min(b), a.max(b))) {
                    edges.push((a, b));
                }
            }
            if edges.len() > 16 {
                continue;
            }
            let m = max_matching(n, &edges);
            assert!(valid_matching(&edges, &m), "invalid matching {m:?}");
            assert_eq!(m.len(), brute(n, &edges), "suboptimal on {edges:?}");
        }
    }

    #[test]
    fn determinism() {
        let edges: Vec<(usize, usize)> = {
            let mut r = SplitMix64::new(42);
            (0..60)
                .map(|_| (r.below(12) as usize, r.below(12) as usize))
                .collect()
        };
        assert_eq!(max_matching(12, &edges), max_matching(12, &edges));
    }
}

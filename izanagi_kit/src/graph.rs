//! Deterministic graph algorithms over `u32`-indexed adjacency lists.
//!
//! - [`strongly_connected`] — Tarjan's SCC decomposition (Tarjan 1972,
//!   *SIAM J. Comput.* 1(2)): returns components in reverse topological
//!   order of the condensation DAG, each component sorted ascending.
//! - [`articulation_points`] / [`bridges`] — the classic low-link cut
//!   structure of the *undirected* projection: the chokepoints where a
//!   generated map or tech tree would split.
//! - [`topo_sort`] — Kahn's algorithm with ascending ready-queue; returns
//!   `None` on a cycle. Deterministic build-order for any DAG.
//! - [`UnionFind`] — path-halving + union-by-rank disjoint set (the
//!   structure `voronoi::mst_edges` uses internally), exposed for callers
//!   writing their own connectivity passes.
//!
//! Everything is integer-indexed and order-deterministic: DFS visits
//! neighbours in adjacency-list order, so output is a pure function of the
//! input arrays — no hashing anywhere.
//!
//! ```
//! use izanagi_kit::graph::{strongly_connected, topo_sort};
//! let dag = vec![vec![1], vec![2], vec![]]; // 0→1→2
//! assert_eq!(topo_sort(&dag), Some(vec![0, 1, 2]));
//! // A directed cycle and a tail: {1,2} is an SCC, 0 is alone.
//! let g = vec![vec![1], vec![2], vec![1]];
//! let scc = strongly_connected(&g);
//! assert!(scc.iter().any(|c| c == &vec![1, 2]));
//! ```

/// Disjoint-set (union-find) with union-by-rank and path halving —
/// near-constant `find`. Deterministic: `representative` is a pure function
/// of the union sequence.
///
/// ```
/// use izanagi_kit::graph::UnionFind;
/// let mut uf = UnionFind::new(4);
/// uf.union(0, 1);
/// uf.union(2, 3);
/// assert!(uf.same(0, 1) && uf.same(2, 3) && !uf.same(0, 3));
/// assert_eq!(uf.sets(), 2);
/// ```
#[derive(Clone, Debug)]
pub struct UnionFind {
    parent: Vec<u32>,
    rank: Vec<u8>,
    sets: usize,
}

impl UnionFind {
    /// `n` singleton sets `{0}..{n-1}`.
    pub fn new(n: u32) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            rank: vec![0; n as usize],
            sets: n as usize,
        }
    }

    /// The representative of `x`'s set.
    pub fn find(&mut self, x: u32) -> u32 {
        let mut r = x;
        // First pass: find the root.
        while self.parent[r as usize] != r {
            r = self.parent[r as usize];
        }
        // Path halving: point every other node at its grandparent.
        let mut c = x;
        while self.parent[c as usize] != r {
            let g = self.parent[self.parent[c as usize] as usize];
            self.parent[c as usize] = g;
            c = self.parent[c as usize];
        }
        r
    }

    /// Merge the sets containing `a` and `b`; returns true when they were
    /// separate (the merge actually did something).
    pub fn union(&mut self, a: u32, b: u32) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        // Union by rank; ties keep the smaller index as root for
        // determinism of the representative.
        let (mut hi, mut lo) = (ra, rb);
        if self.rank[lo as usize] > self.rank[hi as usize]
            || (self.rank[lo as usize] == self.rank[hi as usize] && lo < hi)
        {
            std::mem::swap(&mut hi, &mut lo);
        }
        self.parent[lo as usize] = hi;
        if self.rank[hi as usize] == self.rank[lo as usize] {
            self.rank[hi as usize] += 1;
        }
        self.sets -= 1;
        true
    }

    /// Whether `a` and `b` are in the same set.
    pub fn same(&mut self, a: u32, b: u32) -> bool {
        self.find(a) == self.find(b)
    }

    /// Number of disjoint sets remaining.
    pub fn sets(&self) -> usize {
        self.sets
    }
}

/// Tarjan's strongly-connected-component decomposition of a directed graph.
/// Components come back in reverse topological order of the condensation
/// (sinks first), each component's vertices ascending. Runs in `O(V + E)`
/// with an explicit iterative stack — no recursion, so depth is unbounded.
pub fn strongly_connected(adj: &[Vec<u32>]) -> Vec<Vec<u32>> {
    let n = adj.len();
    let mut index = vec![u32::MAX; n];
    let mut low = vec![0u32; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<u32> = Vec::new();
    let mut out: Vec<Vec<u32>> = Vec::new();
    let mut next = 0u32;

    for start in 0..n as u32 {
        if index[start as usize] != u32::MAX {
            continue;
        }
        // Iterative Tarjan: (node, next-neighbour-slot) frames.
        let mut dfs: Vec<(u32, usize)> = vec![(start, 0)];
        index[start as usize] = next;
        low[start as usize] = next;
        next += 1;
        stack.push(start);
        on_stack[start as usize] = true;
        loop {
            // Decide this step's action inside the borrow, apply it after.
            enum Step {
                Descend(u32),
                Next,
                Pop(u32),
            }
            let step = {
                let Some(&mut (v, ref mut i)) = dfs.last_mut() else {
                    break;
                };
                if *i < adj[v as usize].len() {
                    let w = adj[v as usize][*i];
                    *i += 1;
                    if index[w as usize] == u32::MAX {
                        Step::Descend(w)
                    } else {
                        if on_stack[w as usize] {
                            low[v as usize] = low[v as usize].min(index[w as usize]);
                        }
                        Step::Next
                    }
                } else {
                    Step::Pop(v)
                }
            };
            match step {
                Step::Descend(w) => {
                    index[w as usize] = next;
                    low[w as usize] = next;
                    next += 1;
                    stack.push(w);
                    on_stack[w as usize] = true;
                    dfs.push((w, 0));
                }
                Step::Next => {}
                Step::Pop(v) => {
                    // Propagate low-link to the parent before popping.
                    if dfs.len() >= 2 {
                        let p = dfs[dfs.len() - 2].0;
                        low[p as usize] = low[p as usize].min(low[v as usize]);
                    }
                    if low[v as usize] == index[v as usize] {
                        // v is an SCC root: pop the stack to v.
                        let mut comp = Vec::new();
                        while let Some(w) = stack.pop() {
                            on_stack[w as usize] = false;
                            comp.push(w);
                            if w == v {
                                break;
                            }
                        }
                        comp.sort_unstable();
                        out.push(comp);
                    }
                    dfs.pop();
                }
            }
        }
    }
    out
}

/// Articulation points of the undirected projection: vertices whose removal
/// disconnects the graph. Returned sorted. Self-loops and parallel edges
/// are handled correctly (neither is ever a bridge).
pub fn articulation_points(adj_undirected: &[Vec<u32>]) -> Vec<u32> {
    let n = adj_undirected.len();
    let mut disc = vec![u32::MAX; n];
    let mut low = vec![0u32; n];
    let mut out = Vec::new();
    let mut time = 0u32;

    for start in 0..n as u32 {
        if disc[start as usize] != u32::MAX {
            continue;
        }
        // Frames: (v, parent, next-neighbour-slot, children-seen,
        // parent-edge-skipped). The last flag skips exactly ONE adjacency
        // entry pointing back to the parent — the tree edge — so a second
        // (parallel) edge counts as a real back edge lowering `low`.
        let mut dfs: Vec<(u32, u32, usize, u32, bool)> = vec![(start, u32::MAX, 0, 0, false)];
        disc[start as usize] = time;
        low[start as usize] = time;
        time += 1;
        loop {
            enum Step {
                Descend(u32),
                Next,
                Pop(u32),
            }
            let step = {
                let Some(&mut (v, p, ref mut i, ref mut children, ref mut par_skipped)) =
                    dfs.last_mut()
                else {
                    break;
                };
                if *i < adj_undirected[v as usize].len() {
                    let w = adj_undirected[v as usize][*i];
                    *i += 1;
                    if disc[w as usize] == u32::MAX {
                        *children += 1;
                        Step::Descend(w)
                    } else if w == p && !*par_skipped {
                        *par_skipped = true;
                        Step::Next
                    } else {
                        low[v as usize] = low[v as usize].min(disc[w as usize]);
                        Step::Next
                    }
                } else {
                    Step::Pop(v)
                }
            };
            match step {
                Step::Descend(w) => {
                    let v = dfs.last().map(|f| f.0).unwrap_or(start);
                    disc[w as usize] = time;
                    low[w as usize] = time;
                    time += 1;
                    dfs.push((w, v, 0, 0, false));
                }
                Step::Next => {}
                Step::Pop(v) => {
                    if dfs.len() >= 2 {
                        let p = dfs[dfs.len() - 2].0;
                        low[p as usize] = low[p as usize].min(low[v as usize]);
                        if p != start && low[v as usize] >= disc[p as usize] {
                            out.push(p);
                        }
                    }
                    if dfs.len() == 1 && dfs[0].3 > 1 {
                        out.push(start);
                    }
                    dfs.pop();
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Bridge edges of the undirected projection — `(u, v)` with `u < v`,
/// sorted. Removing one disconnects the graph. Parallel edges are never
/// bridges (the second edge to the same neighbour acts as a back edge).
pub fn bridges(adj_undirected: &[Vec<u32>]) -> Vec<(u32, u32)> {
    let n = adj_undirected.len();
    let mut disc = vec![u32::MAX; n];
    let mut low = vec![0u32; n];
    let mut out = Vec::new();
    let mut time = 0u32;

    for start in 0..n as u32 {
        if disc[start as usize] != u32::MAX {
            continue;
        }
        // Frame flag par_skipped: skip exactly one adjacency entry back to
        // the parent, so parallel edges still lower `low` (and therefore
        // are never reported as bridges).
        let mut dfs: Vec<(u32, u32, usize, bool)> = vec![(start, u32::MAX, 0, false)];
        disc[start as usize] = time;
        low[start as usize] = time;
        time += 1;
        loop {
            enum Step {
                Descend(u32),
                Next,
                Pop(u32),
            }
            let step = {
                let Some(&mut (v, p, ref mut i, ref mut par_skipped)) = dfs.last_mut() else {
                    break;
                };
                if *i < adj_undirected[v as usize].len() {
                    let w = adj_undirected[v as usize][*i];
                    *i += 1;
                    if disc[w as usize] == u32::MAX {
                        Step::Descend(w)
                    } else if w == p && !*par_skipped {
                        *par_skipped = true;
                        Step::Next
                    } else {
                        low[v as usize] = low[v as usize].min(disc[w as usize]);
                        Step::Next
                    }
                } else {
                    Step::Pop(v)
                }
            };
            match step {
                Step::Descend(w) => {
                    let v = dfs.last().map(|f| f.0).unwrap_or(start);
                    disc[w as usize] = time;
                    low[w as usize] = time;
                    time += 1;
                    dfs.push((w, v, 0, false));
                }
                Step::Next => {}
                Step::Pop(v) => {
                    if dfs.len() >= 2 {
                        let p = dfs[dfs.len() - 2].0;
                        low[p as usize] = low[p as usize].min(low[v as usize]);
                        if low[v as usize] > disc[p as usize] {
                            out.push((p.min(v), p.max(v)));
                        }
                    }
                    dfs.pop();
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Kahn topological sort — `Some(order)` for a DAG (ascending ready-queue:
/// among all zero-indegree vertices the smallest is emitted first, so the
/// order is the canonical lexicographically-smallest topological order) or
/// `None` when a cycle remains.
pub fn topo_sort(adj: &[Vec<u32>]) -> Option<Vec<u32>> {
    let n = adj.len();
    let mut indeg = vec![0u32; n];
    for outs in adj {
        for &w in outs {
            if (w as usize) < n {
                indeg[w as usize] += 1;
            }
        }
    }
    // Ascending ready queue: a sorted VecDeque-free scan is O(V²); use a
    // BinaryHeap of Reverse for determinism at O(E log V).
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let mut heap: BinaryHeap<Reverse<u32>> = BinaryHeap::new();
    for v in 0..n as u32 {
        if indeg[v as usize] == 0 {
            heap.push(Reverse(v));
        }
    }
    let mut order = Vec::with_capacity(n);
    while let Some(Reverse(v)) = heap.pop() {
        order.push(v);
        for &w in &adj[v as usize] {
            if (w as usize) < n {
                indeg[w as usize] -= 1;
                if indeg[w as usize] == 0 {
                    heap.push(Reverse(w));
                }
            }
        }
    }
    if order.len() == n {
        Some(order)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Oracle SCC: group by mutual reachability via plain BFS.
    fn scc_oracle(adj: &[Vec<u32>]) -> Vec<BTreeSet<u32>> {
        let n = adj.len();
        let reach = |from: u32| -> Vec<bool> {
            let mut seen = vec![false; n];
            let mut q = vec![from];
            seen[from as usize] = true;
            while let Some(v) = q.pop() {
                for &w in &adj[v as usize] {
                    if !seen[w as usize] {
                        seen[w as usize] = true;
                        q.push(w);
                    }
                }
            }
            seen
        };
        let mut assigned = vec![false; n];
        let mut out = Vec::new();
        for v in 0..n as u32 {
            if assigned[v as usize] {
                continue;
            }
            let rv = reach(v);
            let comp: BTreeSet<u32> = (0..n as u32)
                .filter(|&w| rv[w as usize] && reach(w)[v as usize])
                .collect();
            for &w in &comp {
                assigned[w as usize] = true;
            }
            out.push(comp);
        }
        out
    }

    fn undirected(n: usize, edges: &[(u32, u32)]) -> Vec<Vec<u32>> {
        let mut adj = vec![Vec::new(); n];
        for &(a, b) in edges {
            adj[a as usize].push(b);
            adj[b as usize].push(a);
        }
        adj
    }

    fn components(adj: &[Vec<u32>]) -> usize {
        let mut seen = vec![false; adj.len()];
        let mut count = 0;
        for s in 0..adj.len() {
            if seen[s] {
                continue;
            }
            count += 1;
            let mut q = vec![s as u32];
            seen[s] = true;
            while let Some(v) = q.pop() {
                for &w in &adj[v as usize] {
                    if !seen[w as usize] {
                        seen[w as usize] = true;
                        q.push(w);
                    }
                }
            }
        }
        count
    }

    #[test]
    fn scc_on_known_shapes() {
        // Two cycles joined by a bridge: {0,1}, {2,3,4}.
        let g = vec![vec![1], vec![0, 2], vec![3], vec![4], vec![2]];
        let scc = strongly_connected(&g);
        let set: BTreeSet<BTreeSet<u32>> =
            scc.iter().map(|c| c.iter().copied().collect()).collect();
        let oracle: BTreeSet<BTreeSet<u32>> = scc_oracle(&g).into_iter().collect();
        assert_eq!(set, oracle);
    }

    #[test]
    fn scc_matches_reachability_oracle_on_random_graphs() {
        let mut rng = SplitMix64::new(0x7A9A);
        for _ in 0..80 {
            let n = rng.below(9) as usize + 1;
            let m = rng.below(20) as usize;
            let adj: Vec<Vec<u32>> = {
                let mut a = vec![Vec::new(); n];
                for _ in 0..m {
                    let (u, v) = (rng.below(n as u32), rng.below(n as u32));
                    a[u as usize].push(v);
                }
                a
            };
            let set: BTreeSet<BTreeSet<u32>> = strongly_connected(&adj)
                .iter()
                .map(|c| c.iter().copied().collect())
                .collect();
            let oracle: BTreeSet<BTreeSet<u32>> = scc_oracle(&adj).into_iter().collect();
            assert_eq!(set, oracle);
        }
    }

    #[test]
    fn articulation_and_bridges_on_known_shapes() {
        // Path 0-1-2 plus a triangle 2-3-4-2 hanging off 2.
        let g = undirected(5, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 2)]);
        assert_eq!(articulation_points(&g), vec![1, 2]);
        assert_eq!(bridges(&g), vec![(0, 1), (1, 2)]);
        // Parallel edge kills the bridge.
        let mut g2 = g.clone();
        g2[0].push(1);
        g2[1].push(0);
        assert_eq!(bridges(&g2), vec![(1, 2)]);
    }

    #[test]
    fn cuts_match_removal_oracle_on_random_graphs() {
        let mut rng = SplitMix64::new(0xB21D);
        for _ in 0..60 {
            let n = rng.below(8) as usize + 2;
            let m = rng.below(12) as usize;
            let edges: Vec<(u32, u32)> = (0..m)
                .map(|_| {
                    let u = rng.below(n as u32);
                    let mut v = rng.below(n as u32);
                    if v == u {
                        v = (v + 1) % n as u32;
                    }
                    (u.min(v), v.max(u))
                })
                .collect();
            let g = undirected(n, &edges);
            let base = components(&g);
            // Oracle articulation: removing v increases component count.
            let mut oracle_ap = BTreeSet::new();
            for v in 0..n as u32 {
                let adj2: Vec<Vec<u32>> = g
                    .iter()
                    .enumerate()
                    .map(|(i, outs)| {
                        if i == v as usize {
                            Vec::new()
                        } else {
                            outs.iter().copied().filter(|&w| w != v).collect()
                        }
                    })
                    .collect();
                // Isolated removed vertex doesn't count toward components.
                let mut seen = vec![false; n];
                let mut count = 0;
                for s in 0..n {
                    if s == v as usize || seen[s] {
                        continue;
                    }
                    count += 1;
                    let mut q = vec![s as u32];
                    seen[s] = true;
                    while let Some(u) = q.pop() {
                        for &w in &adj2[u as usize] {
                            if !seen[w as usize] {
                                seen[w as usize] = true;
                                q.push(w);
                            }
                        }
                    }
                }
                if count > base {
                    oracle_ap.insert(v);
                }
            }
            let got: BTreeSet<u32> = articulation_points(&g).into_iter().collect();
            assert_eq!(got, oracle_ap, "articulation on {edges:?}");

            // Oracle bridge: removing that edge increases components.
            let mut oracle_br = BTreeSet::new();
            for i in 0..edges.len() {
                let mut e2 = edges.clone();
                e2.remove(i);
                if components(&undirected(n, &e2)) > base {
                    oracle_br.insert(edges[i]);
                }
            }
            let got: BTreeSet<(u32, u32)> = bridges(&g).into_iter().collect();
            assert_eq!(got, oracle_br, "bridges on {edges:?}");
        }
    }

    #[test]
    fn topo_is_a_valid_linear_extension() {
        let mut rng = SplitMix64::new(0x70C0);
        for _ in 0..100 {
            let n = rng.below(10) as usize + 1;
            // Random DAG: edges only from earlier to later in a shuffled order.
            let mut perm: Vec<u32> = (0..n as u32).collect();
            for i in (1..n).rev() {
                perm.swap(i, rng.below(i as u32 + 1) as usize);
            }
            let mut adj = vec![Vec::new(); n];
            for i in 0..n {
                for j in i + 1..n {
                    if rng.below(4) == 0 {
                        adj[perm[i] as usize].push(perm[j]);
                    }
                }
            }
            let order = topo_sort(&adj).unwrap_or_else(|| panic!("DAG rejected"));
            let mut pos = vec![0u32; n];
            for (i, &v) in order.iter().enumerate() {
                pos[v as usize] = i as u32;
            }
            for (v, outs) in adj.iter().enumerate() {
                for &w in outs {
                    assert!(pos[v] < pos[w as usize], "edge {v}->{w} violates order");
                }
            }
            // Cycles are rejected.
            assert_eq!(topo_sort(&[vec![1], vec![0]]), None);
        }
    }

    #[test]
    fn union_find_behaves_like_a_reference_set_model() {
        let mut rng = SplitMix64::new(0xF1D5);
        for _ in 0..80 {
            let n = rng.below(12) + 1;
            let mut uf = UnionFind::new(n);
            let mut model: Vec<BTreeSet<u32>> = (0..n).map(|i| BTreeSet::from([i])).collect();
            for _ in 0..40 {
                let (a, b) = (rng.below(n), rng.below(n));
                let merged = uf.union(a, b);
                let (sa, sb) = (
                    model.iter().position(|s| s.contains(&a)).unwrap_or(0),
                    model.iter().position(|s| s.contains(&b)).unwrap_or(0),
                );
                assert_eq!(merged, sa != sb);
                if sa != sb {
                    let take = model.remove(sb.max(sa));
                    model[sa.min(sb)].extend(take);
                }
                let (x, y) = (rng.below(n), rng.below(n));
                let same_model = model.iter().any(|s| s.contains(&x) && s.contains(&y));
                assert_eq!(uf.same(x, y), same_model);
                assert_eq!(uf.sets(), model.len());
            }
        }
    }
}

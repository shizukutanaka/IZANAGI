//! Steiner tree — minimum tree spanning a terminal subset of a
//! weighted undirected graph. NP-hard in general, so this is the
//! classic 2-approximation (Kou–Markowsky–Berman 1981):
//!
//! 1. **Metric closure**: all-pairs shortest-path distances between
//!    terminals (one Dijkstra per terminal).
//! 2. **MST** on the complete terminal graph (Kruskal — edge set is
//!    `t choose 2`, small).
//! 3. **Unfold**: replace each MST edge by its shortest path in the
//!    original graph; union the edges.
//! 4. **Prune**: MST of the union removes any cycles the unfolded
//!    paths created — can only lower the weight.
//!
//! The bound `weight ≤ 2·opt` holds because doubling an optimal
//! Steiner tree gives an Euler tour whose shortcut tour through the
//! terminals costs ≤ 2·opt, and the closure MST beats any such tour.
//! Deterministic throughout: all ties break on vertex indices.
//!
//! ```
//! use izanagi_kit::steiner::steiner_tree;
//! // Path 0-1-2-3 with unit edges; terminals 0 and 3.
//! let edges = vec![(0, 1, 1u64), (1, 2, 1), (2, 3, 1), (0, 3, 10)];
//! let t = steiner_tree(4, &edges, &[0, 3]).unwrap();
//! assert_eq!(t.weight, 3);
//! ```

use crate::graph::UnionFind;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// An approximately-minimal tree spanning the requested terminals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteinerTree {
    /// Original-graph edges `(u, v, w)` making up the tree — sorted
    /// and deduplicated, always `u < v`.
    pub edges: Vec<(u32, u32, u64)>,
    /// Total edge weight.
    pub weight: u64,
}

/// Dijkstra from `src` on the undirected weighted `adj` —
/// `(dist, prev)`; `None` distances are unreachable.
fn dijkstra(adj: &[Vec<(usize, u64)>], src: usize) -> (Vec<Option<u64>>, Vec<Option<usize>>) {
    let n = adj.len();
    let mut dist: Vec<Option<u64>> = vec![None; n];
    let mut prev: Vec<Option<usize>> = vec![None; n];
    dist[src] = Some(0);
    let mut heap = BinaryHeap::with_capacity(n);
    heap.push((Reverse(0u64), src));
    while let Some((Reverse(d), u)) = heap.pop() {
        if dist[u] != Some(d) {
            continue; // stale entry
        }
        for &(v, w) in &adj[u] {
            let nd = d.saturating_add(w);
            if dist[v].is_none() || nd < dist[v].unwrap_or(u64::MAX) {
                dist[v] = Some(nd);
                prev[v] = Some(u);
                heap.push((Reverse(nd), v));
            }
        }
    }
    (dist, prev)
}

/// Approximate Steiner tree over `edges` (undirected `u64` weights,
/// parallel edges allowed — the cheapest wins) covering every vertex
/// in `terminals`. `None` when the terminals are not all in one
/// connected component, or when the input is malformed (endpoint
/// `≥ n`). A single terminal yields the empty tree; no terminals
/// yields the empty tree too.
pub fn steiner_tree(n: usize, edges: &[(u32, u32, u64)], terminals: &[u32]) -> Option<SteinerTree> {
    let mut adj: Vec<Vec<(usize, u64)>> = vec![Vec::new(); n];
    for &(u, v, w) in edges {
        if u as usize >= n || v as usize >= n {
            return None;
        }
        if u == v {
            continue;
        }
        adj[u as usize].push((v as usize, w));
        adj[v as usize].push((u as usize, w));
    }
    if terminals.is_empty() {
        return Some(SteinerTree {
            edges: Vec::new(),
            weight: 0,
        });
    }
    if terminals.iter().any(|&t| t as usize >= n) {
        return None;
    }
    let t = terminals.len();
    // Keep only the cheapest parallel edge between each terminal pair;
    // distances below already handle that implicitly.
    let mut dists: Vec<Vec<Option<u64>>> = Vec::with_capacity(t);
    let mut prevs: Vec<Vec<Option<usize>>> = Vec::with_capacity(t);
    for &term in terminals {
        let (d, p) = dijkstra(&adj, term as usize);
        dists.push(d);
        prevs.push(p);
    }
    for d in &dists {
        for &term in terminals {
            let _ = d[term as usize]?; // disconnected terminals
        }
    }
    // Metric-closure MST (Kruskal on the complete terminal graph).
    let mut tedges: Vec<(u64, usize, usize)> = Vec::with_capacity(t * (t - 1) / 2);
    for (i, di) in dists.iter().enumerate() {
        for j in (i + 1)..t {
            let w = di[terminals[j] as usize].unwrap_or(u64::MAX);
            tedges.push((w, i, j));
        }
    }
    tedges.sort();
    let mut uf = UnionFind::new(t as u32);
    let mut union_edges: Vec<(u32, u32, u64)> = Vec::new();
    for &(_, i, j) in &tedges {
        if uf.union(i as u32, j as u32) {
            // Unfold the shortest path terminals[i] → terminals[j].
            let mut v = terminals[j] as usize;
            while let Some(u) = prevs[i][v] {
                // Find the (cheapest) original edge u—v.
                let w = adj[u]
                    .iter()
                    .filter(|&&(x, _)| x == v)
                    .map(|&(_, w)| w)
                    .min()
                    .unwrap_or(0);
                let (a, b) = if u < v { (u, v) } else { (v, u) };
                union_edges.push((a as u32, b as u32, w));
                v = u;
            }
        }
    }
    // Prune cycles: MST over the unioned path edges.
    union_edges.sort();
    union_edges.dedup();
    union_edges.sort_by_key(|&(a, b, w)| (w, a, b));
    let mut uf2 = UnionFind::new(n as u32);
    let mut tree: Vec<(u32, u32, u64)> = Vec::new();
    let mut weight = 0u64;
    for &(a, b, w) in &union_edges {
        if uf2.union(a, b) {
            tree.push((a, b, w));
            weight = weight.saturating_add(w);
        }
    }
    tree.sort();
    Some(SteinerTree {
        edges: tree,
        weight,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Exact optimum via Dreyfus–Wagner DP — `dp[mask][v]` =
    /// cheapest tree covering terminal-subset `mask` rooted at `v`.
    /// `O(3^k·n + 2^k·n²)` — fine for `k ≤ 5`, `n ≤ 10`.
    fn optimal(n: usize, edges: &[(u32, u32, u64)], terminals: &[u32]) -> Option<u64> {
        let mut adj: Vec<Vec<(usize, u64)>> = vec![Vec::new(); n];
        for &(u, v, w) in edges {
            adj[u as usize].push((v as usize, w));
            adj[v as usize].push((u as usize, w));
        }
        let t = terminals.len();
        let full = 1usize << t;
        let inf = u64::MAX / 4;
        let mut dp = vec![vec![inf; n]; full];
        for (i, &term) in terminals.iter().enumerate() {
            dp[1 << i][term as usize] = 0;
        }
        // Combine over masks in increasing size order; then relax
        // each row with multi-source Dijkstra over the metric.
        let mut masks: Vec<usize> = (1..full).collect();
        masks.sort_by_key(|m| m.count_ones());
        for &mask in &masks {
            // Combine: try proper nonempty submasks.
            let mut sub = (mask - 1) & mask;
            while sub > 0 {
                let other = mask ^ sub;
                if other > 0 {
                    let cand: Vec<u64> = dp[sub]
                        .iter()
                        .zip(dp[other].iter())
                        .map(|(&a, &b)| a.saturating_add(b))
                        .collect();
                    for (dm, &c) in dp[mask].iter_mut().zip(cand.iter()) {
                        if c < *dm {
                            *dm = c;
                        }
                    }
                }
                sub = (sub - 1) & mask;
            }
            // Relax: shortest-path closure.
            let mut heap = BinaryHeap::with_capacity(n);
            for (v, &dv) in dp[mask].iter().enumerate() {
                if dv < inf {
                    heap.push((Reverse(dv), v));
                }
            }
            while let Some((Reverse(d), u)) = heap.pop() {
                if dp[mask][u] < d {
                    continue;
                }
                for &(w, wt) in &adj[u] {
                    let nd = d.saturating_add(wt);
                    if nd < dp[mask][w] {
                        dp[mask][w] = nd;
                        heap.push((Reverse(nd), w));
                    }
                }
            }
        }
        let best = dp[full - 1].iter().copied().min()?;
        (best < inf).then_some(best)
    }

    fn random_graph(rng: &mut SplitMix64, n: usize, extra: usize) -> Vec<(u32, u32, u64)> {
        // Random spanning tree + `extra` chords, weights 1..=9.
        let mut edges = Vec::new();
        for v in 1..n {
            let u = rng.below(v as u32) as usize;
            edges.push((u as u32, v as u32, rng.below(9) as u64 + 1));
        }
        for _ in 0..extra {
            let u = rng.below(n as u32) as usize;
            let v = rng.below(n as u32) as usize;
            if u != v {
                edges.push((u as u32, v as u32, rng.below(9) as u64 + 1));
            }
        }
        edges
    }

    /// Result sanity: edges form a tree spanning all terminals.
    fn check_is_tree(n: usize, st: &SteinerTree, terminals: &[u32]) {
        let mut verts = BTreeSet::new();
        for &(u, v, _) in &st.edges {
            assert!(u < v);
            verts.insert(u);
            verts.insert(v);
        }
        if !st.edges.is_empty() {
            // A real tree: |E| = |V| - 1.
            assert_eq!(st.edges.len(), verts.len() - 1);
        }
        let mut uf = UnionFind::new(n as u32);
        for &(u, v, _) in &st.edges {
            uf.union(u, v);
        }
        for &t in terminals {
            for &t2 in terminals {
                assert!(uf.same(t, t2), "terminals {t} and {t2} disconnected");
            }
        }
        let sum: u64 = st.edges.iter().map(|&(_, _, w)| w).sum();
        assert_eq!(sum, st.weight);
        let _ = n;
    }

    #[test]
    fn within_factor_two_of_optimal() {
        let mut rng = SplitMix64::new(0x57E1);
        for _ in 0..300 {
            let n = 3 + rng.below(6) as usize;
            let edges = random_graph(&mut rng, n, 2);
            let k = 1 + rng.below(4.min(n as u32)) as usize;
            let mut terms = BTreeSet::new();
            while terms.len() < k {
                terms.insert(rng.below(n as u32));
            }
            let terminals: Vec<u32> = terms.into_iter().collect();
            let opt = optimal(n, &edges, &terminals).unwrap();
            let st = steiner_tree(n, &edges, &terminals).unwrap();
            check_is_tree(n, &st, &terminals);
            assert!(st.weight >= opt, "below optimum?");
            assert!(
                st.weight <= 2 * opt,
                "weight {} exceeds 2×opt {}",
                st.weight,
                opt
            );
        }
    }

    #[test]
    fn known_shapes() {
        // Star is optimal: terminals at leaves, Steiner at center.
        // 0 center; leaves 1,2,3 weight 1 each; direct leaf-leaf edges
        // weight 10.
        let edges = vec![
            (0, 1, 1u64),
            (0, 2, 1),
            (0, 3, 1),
            (1, 2, 10),
            (2, 3, 10),
            (1, 3, 10),
        ];
        let st = steiner_tree(4, &edges, &[1, 2, 3]).unwrap();
        assert_eq!(st.weight, 3);
        assert_eq!(st.edges.len(), 3);
        // Single terminal → empty tree.
        let st = steiner_tree(4, &edges, &[2]).unwrap();
        assert_eq!(st.weight, 0);
        assert!(st.edges.is_empty());
        // Two terminals → shortest path (the unit two-hop way).
        let st = steiner_tree(4, &edges, &[1, 3]).unwrap();
        assert_eq!(st.weight, 2);
    }

    #[test]
    fn disconnected_terminals_and_bad_input_fail() {
        // Two disjoint edges; terminals on different ones.
        let edges = vec![(0, 1, 1u64), (2, 3, 1)];
        assert!(steiner_tree(4, &edges, &[0, 2]).is_none());
        assert!(steiner_tree(4, &edges, &[0, 5]).is_none()); // bad endpoint in terminals→also None via missing dist
        let edges = vec![(0, 5, 1u64)];
        assert!(steiner_tree(4, &edges, &[0, 5]).is_none()); // bad endpoint in edges
                                                             // Empty terminal list → empty tree.
        assert_eq!(steiner_tree(2, &[(0, 1, 1u64)], &[]).unwrap().weight, 0);
    }
}

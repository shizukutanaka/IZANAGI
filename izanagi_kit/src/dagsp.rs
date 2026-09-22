//! DAG shortest / longest path by topological order — `O(V+E)` where
//! [`bellman`](crate::bellman) spends `O(V·E)`, and the *longest* path
//! (the critical path of a build/tech/quest schedule — what `pathfinding`
//! can't express at all). Edges may carry any signed weight; `None` when
//! the graph isn't acyclic.
//!
//! ```
//! use izanagi_kit::dagsp::dag_paths;
//! // 0→1(2) 0→2(5) 1→2(-3): longest to 2 is 5, shortest is -1.
//! let p = dag_paths(3, &[(0,1,2),(0,2,5),(1,2,-3)], 0).unwrap_or_default();
//! assert_eq!(p.lo[2], Some(-1));
//! assert_eq!(p.hi[2], Some(5));
//! ```

use crate::graph::topo_sort;

/// Build the topo order or `None`; out-of-range edges are skipped.
fn order_of(n: usize, edges: &[(usize, usize, i64)]) -> Option<Vec<u32>> {
    let mut adj = vec![Vec::<u32>::new(); n];
    for &(u, v, _) in edges {
        if u < n && v < n {
            adj[u].push(v as u32);
        }
    }
    topo_sort(&adj)
}

/// Shortest `lo[v]` and longest `hi[v]` distances from the source on
/// a DAG — `None` per vertex means unreachable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DagPaths {
    /// Shortest-path distance to each vertex.
    pub lo: Vec<Option<i64>>,
    /// Longest-path distance to each vertex.
    pub hi: Vec<Option<i64>>,
}

/// Single-source shortest AND longest paths on a DAG.
/// `None` overall when a cycle exists.
pub fn dag_paths(n: usize, edges: &[(usize, usize, i64)], src: usize) -> Option<DagPaths> {
    if src >= n {
        return None;
    }
    let order = order_of(n, edges)?;
    let adj_edges: Vec<(usize, usize, i64)> = edges
        .iter()
        .copied()
        .filter(|&(u, v, _)| u < n && v < n)
        .collect();
    let mut lo: Vec<Option<i64>> = vec![None; n];
    let mut hi: Vec<Option<i64>> = vec![None; n];
    lo[src] = Some(0);
    hi[src] = Some(0);
    for &u_ in &order {
        let u = u_ as usize;
        let (du_lo, du_hi) = (lo[u], hi[u]);
        for &(_, v, w) in adj_edges.iter().filter(|&&(a, _, _)| a == u) {
            if let Some(d) = du_lo {
                let c = d.saturating_add(w);
                let better = match lo[v] {
                    None => true,
                    Some(x) => c < x,
                };
                if better {
                    lo[v] = Some(c);
                }
            }
            if let Some(d) = du_hi {
                let c = d.saturating_add(w);
                let better = match hi[v] {
                    None => true,
                    Some(x) => c > x,
                };
                if better {
                    hi[v] = Some(c);
                }
            }
        }
    }
    Some(DagPaths { lo, hi })
}

/// The critical path — longest route through a DAG, as vertex list.
/// Picks the lexicographically-earliest order among ties for
/// determinism. `None` on a cyclic graph; `Some([])` on `n == 0`.
pub fn critical_path(n: usize, edges: &[(usize, usize, i64)]) -> Option<Vec<usize>> {
    let order = order_of(n, edges)?;
    if n == 0 {
        return Some(Vec::new());
    }
    let adj_edges: Vec<(usize, usize, i64)> = edges
        .iter()
        .copied()
        .filter(|&(u, v, _)| u < n && v < n)
        .collect();
    // Every vertex may start the path (empty prefix, weight 0) — the
    // global longest route need not begin at an in-degree-0 source.
    let mut dist: Vec<Option<i64>> = (0..n).map(|_| Some(0)).collect();
    let mut prev: Vec<Option<usize>> = vec![None; n];
    for &u_ in &order {
        let u = u_ as usize;
        if let Some(d) = dist[u] {
            for &(_, v, w) in adj_edges.iter().filter(|&&(a, _, _)| a == u) {
                let c = d.saturating_add(w);
                let better = match dist[v] {
                    None => true,
                    Some(x) => c > x || (c == x && u < prev[v].unwrap_or(u)),
                };
                if better {
                    dist[v] = Some(c);
                    prev[v] = Some(u);
                }
            }
        }
    }
    // Best endpoint: max dist, smallest vertex on tie.
    let mut end: Option<usize> = None;
    for (v, &d) in dist.iter().enumerate() {
        if d.is_some() {
            let better = match end {
                None => true,
                Some(e) => d > dist[e] || (d == dist[e] && v < e),
            };
            if better {
                end = Some(v);
            }
        }
    }
    let mut path = Vec::new();
    let mut cur = end?;
    for _ in 0..=n {
        path.push(cur);
        match prev[cur] {
            Some(p) => cur = p,
            None => break,
        }
    }
    path.reverse();
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bellman::shortest as bf_shortest;
    use crate::rng::SplitMix64;

    #[test]
    fn shortest_matches_bellman_ford_and_longest_is_max() {
        let mut rng = SplitMix64::new(0xDA65_7A91);
        for _ in 0..300 {
            let n = (rng.below(12) + 1) as usize;
            // Random DAG: only edges u→v with u < v (plus shuffle).
            let perm: Vec<usize> = {
                let mut p: Vec<usize> = (0..n).collect();
                for i in (1..n).rev() {
                    let j = rng.below(i as u32 + 1) as usize;
                    p.swap(i, j);
                }
                p
            };
            let mut edges: Vec<(usize, usize, i64)> = Vec::new();
            for i in 0..n {
                for j in i + 1..n {
                    if rng.below(4) == 0 {
                        edges.push((perm[i], perm[j], (rng.below(21) as i64) - 10));
                    }
                }
            }
            let src = perm[rng.below(n as u32) as usize];
            let p = match dag_paths(n, &edges, src) {
                Some(x) => x,
                None => continue,
            };
            // Shortest ⟷ Bellman–Ford agrees exactly.
            let bf = bf_shortest(n, &edges, src).unwrap_or_default();
            assert_eq!(p.lo, bf.dist, "shortest mismatch {edges:?}");
            // Longest ⟷ Bellman–Ford on negated weights, negated back.
            let neg: Vec<(usize, usize, i64)> = edges.iter().map(|&(u, v, w)| (u, v, -w)).collect();
            let bf2 = bf_shortest(n, &neg, src).unwrap_or_default();
            for v in 0..n {
                assert_eq!(p.hi[v], bf2.dist[v].map(|d| -d), "longest {edges:?}");
            }
            // Critical path validity: each hop is a real edge, sum is the max.
            let cp = critical_path(n, &edges).unwrap_or_default();
            assert!(!cp.is_empty());
            let mut total = 0i64;
            for w in cp.windows(2) {
                let best = edges
                    .iter()
                    .filter(|&&(a, b, _)| a == w[0] && b == w[1])
                    .map(|&(_, _, x)| x)
                    .min();
                total = total.saturating_add(best.unwrap_or(0));
            }
            // Critical path vs global Bellman–Ford longest: augment with
            // a virtual source n→v(0) and negate weights.
            let mut aug: Vec<(usize, usize, i64)> =
                edges.iter().map(|&(u, v, w)| (u, v, -w)).collect();
            for v in 0..n {
                aug.push((n, v, 0));
            }
            let global = bf_shortest(n + 1, &aug, n).unwrap_or_default();
            let want = global.dist.iter().take(n).flatten().min().map(|d| -d);
            assert_eq!(Some(total), want, "cp total {edges:?} {cp:?}");
        }
        // Cycle → None.
        assert!(dag_paths(2, &[(0, 1, 1), (1, 0, 1)], 0).is_none());
        assert!(critical_path(2, &[(0, 1, 1), (1, 0, 1)]).is_none());
        // Known critical path.
        let e = [(0usize, 1usize, 3i64), (0, 2, 1), (1, 3, 4), (2, 3, 10)];
        assert_eq!(critical_path(4, &e), Some(vec![0, 2, 3])); // 11 > 7
        assert_eq!(critical_path(0, &[]), Some(Vec::new()));
    }
}

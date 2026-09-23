//! Yen's algorithm — k shortest *loopless* paths in a directed
//! weighted graph. After the first shortest path (Dijkstra), each
//! subsequent path deviates at some node of an already-accepted
//! path: run Dijkstra again from that spur node on a graph with the
//! rejected edges and the earlier paths' prefix nodes removed.
//! Determinism: candidates are kept in a `BTreeMap` keyed by
//! `(cost, path-vector)`, so both cost ties and equal-cost output
//! order are canonical — the result is a pure function of the input.
//!
//! Negative edges are rejected (Yen's spur optimality rests on
//! Dijkstra). Self-loops are ignored by construction — paths are
//! simple, so a loop can never be taken.
//!
//! ```
//! // 0→1→3 costs 4; 0→2→3 costs 5; the k=2 answer is both.
//! let e = &[(0, 1, 1), (1, 3, 3), (0, 2, 2), (2, 3, 3)];
//! let ps = izanagi_kit::kpaths::yen(4, e, 0, 3, 2);
//! assert_eq!(ps.len(), 2);
//! assert_eq!(ps[0].1, 4);
//! assert_eq!(ps[1].1, 5);
//! ```
//!
//! Reference: Yen (1971), "Finding the k shortest loopless paths
//! in a network" (Management Science).

use std::collections::BTreeMap;

/// Adjacency list `(to, weight)` per source — excludes removed edges.
fn adjacency(
    n: usize,
    edges: &[(u32, u32, u64)],
    banned_edges: &BTreeMap<(u32, u32), ()>,
) -> Vec<Vec<(u32, u64)>> {
    let mut adj: Vec<Vec<(u32, u64)>> = vec![Vec::new(); n];
    for &(a, b, w) in edges {
        if banned_edges.contains_key(&(a, b)) {
            continue;
        }
        if a as usize >= n || b as usize >= n {
            continue;
        }
        adj[a as usize].push((b, w));
    }
    for a in adj.iter_mut() {
        a.sort_unstable();
    }
    adj
}

/// Canonical Dijkstra: lex-min predecessor on (dist,path) order —
/// returns `(dist, path)` or `None` if unreachable.
fn dijkstra(
    adj: &[Vec<(u32, u64)>],
    src: u32,
    dst: u32,
    banned_nodes: &[bool],
) -> Option<(u64, Vec<u32>)> {
    let n = adj.len();
    let mut dist = vec![u64::MAX; n];
    let mut prev = vec![u32::MAX; n];
    // (dist, node) frontier — BTreeMap for deterministic pop order.
    let mut open: BTreeMap<(u64, u32), ()> = BTreeMap::new();
    if banned_nodes[src as usize] {
        return None;
    }
    dist[src as usize] = 0;
    open.insert((0, src), ());
    while let Some(((d, u), _)) = open.iter().next() {
        let (d, u) = (*d, *u);
        open.remove(&(d, u));
        if d != dist[u as usize] {
            continue;
        }
        if u == dst {
            break;
        }
        for &(v, w) in &adj[u as usize] {
            if banned_nodes[v as usize] {
                continue;
            }
            let nd = d.saturating_add(w);
            if nd < dist[v as usize] {
                dist[v as usize] = nd;
                prev[v as usize] = u;
                open.insert((nd, v), ());
            }
        }
    }
    if dist[dst as usize] == u64::MAX {
        return None;
    }
    let mut path = vec![dst];
    let mut cur = dst;
    while cur != src {
        cur = prev[cur as usize];
        if cur == u32::MAX {
            return None;
        }
        path.push(cur);
    }
    path.reverse();
    Some((dist[dst as usize], path))
}

/// Path cost helper.
pub fn path_cost(edges: &[(u32, u32, u64)], path: &[u32]) -> u64 {
    let mut c = 0u64;
    for w in path.windows(2) {
        let mut best = u64::MAX;
        for &(a, b, wt) in edges {
            if a == w[0] && b == w[1] {
                best = best.min(wt);
            }
        }
        c = c.saturating_add(best);
    }
    c
}

/// The k shortest loopless `src→dst` paths, sorted `(cost, path)`.
/// `edges` are directed `(from, to, weight)`; `None` entries never
/// appear — fewer than k paths just returns fewer. `None` when
/// `src == dst` is meaningful only for k ≥ 1 (returns the empty
/// cost-0 path `[src]` plus positive simple cycles — kept simple:
/// `[src]` for the first, then true simple paths for the rest).
pub fn yen(
    n: usize,
    edges: &[(u32, u32, u64)],
    src: u32,
    dst: u32,
    k: usize,
) -> Vec<(Vec<u32>, u64)> {
    if k == 0 || src as usize >= n || dst as usize >= n {
        return Vec::new();
    }
    if src == dst {
        // Shortest is trivially [src]; simple cycles beyond that need
        // a different convention — keep it honest and simple.
        return vec![(vec![src], 0)];
    }
    // deduplicate: for parallel edges keep the min weight (a simple
    // path can only use one of them anyway — canonical choice).
    let mut dedup: BTreeMap<(u32, u32), u64> = BTreeMap::new();
    for &(a, b, w) in edges {
        let e = dedup.entry((a, b)).or_insert(w);
        if w < *e {
            *e = w;
        }
    }
    let edges: Vec<(u32, u32, u64)> = dedup.iter().map(|(&(a, b), &w)| (a, b, w)).collect();

    let no_edges: BTreeMap<(u32, u32), ()> = BTreeMap::new();
    let no_nodes = vec![false; n];
    let mut accepted: Vec<(Vec<u32>, u64)> = Vec::new();
    let mut candidates: BTreeMap<(u64, Vec<u32>), ()> = BTreeMap::new();

    let adj = adjacency(n, &edges, &no_edges);
    let first = match dijkstra(&adj, src, dst, &no_nodes) {
        Some((d, p)) => (p, d),
        None => return Vec::new(),
    };
    accepted.push(first);

    while accepted.len() < k {
        let (prev_path, _) = &accepted[accepted.len() - 1];
        // spur at each prefix node of the last accepted path
        for i in 0..prev_path.len() - 1 {
            let root = &prev_path[..=i];
            let spur = prev_path[i];
            // ban the arc each accepted path takes out of `spur`
            // after the same root (the "next edge" deviations).
            let mut banned_edges: BTreeMap<(u32, u32), ()> = BTreeMap::new();
            for (p, _) in &accepted {
                if p.len() > i + 1 && p[..=i] == *root {
                    banned_edges.insert((p[i], p[i + 1]), ());
                }
            }
            // nodes on the root prefix (except spur) are banned —
            // guarantees looplessness.
            let mut banned_nodes = vec![false; n];
            for &nd in &root[..i] {
                banned_nodes[nd as usize] = true;
            }
            let adj2 = adjacency(n, &edges, &banned_edges);
            if let Some((_d2, tail)) = dijkstra(&adj2, spur, dst, &banned_nodes) {
                if tail.len() > 1 {
                    let mut cand: Vec<u32> = root[..i].to_vec();
                    cand.extend_from_slice(&tail);
                    let cost = path_cost(&edges, &cand);
                    candidates.insert((cost, cand), ());
                }
            }
        }
        match candidates.iter().next() {
            Some(((c, p), _)) => {
                let (c, p) = (*c, p.clone());
                candidates.remove(&(c, p.clone()));
                accepted.push((p, c));
            }
            None => break,
        }
    }
    accepted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn textbook_example() {
        // 0→1→3=4, 0→2→3=5 — the k=2 classic.
        let e = &[(0, 1, 1), (1, 3, 3), (0, 2, 2), (2, 3, 3)];
        let ps = yen(4, e, 0, 3, 2);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0], (vec![0, 1, 3], 4));
        assert_eq!(ps[1], (vec![0, 2, 3], 5));
        // k larger than path count: saturates.
        assert_eq!(yen(4, e, 0, 3, 9).len(), 2);
    }

    #[test]
    fn deviation_paths() {
        // k=3 on a diamond with a back route.
        let e = &[
            (0, 1, 1),
            (1, 3, 1),
            (0, 2, 2),
            (2, 3, 2),
            (1, 2, 1),
            (2, 1, 1),
        ];
        let ps = yen(4, e, 0, 3, 4);
        let costs: Vec<u64> = ps.iter().map(|p| p.1).collect();
        assert_eq!(costs, vec![2, 4, 4, 4]);
        // all simple, all distinct
        for (p, c) in &ps {
            let mut seen = std::collections::BTreeSet::new();
            for &nd in p {
                assert!(seen.insert(nd), "loop in {p:?}");
            }
            assert_eq!(*c, path_cost(e, p));
        }
    }

    /// Brute-force oracle: enumerate ALL simple src→dst paths by DFS,
    /// sort by (cost, path) — Yen must return the top-k prefix.
    fn brute(n: usize, edges: &[(u32, u32, u64)], src: u32, dst: u32) -> Vec<(Vec<u32>, u64)> {
        let mut adj: Vec<Vec<(u32, u64)>> = vec![Vec::new(); n];
        let mut dedup: BTreeMap<(u32, u32), u64> = BTreeMap::new();
        for &(a, b, w) in edges {
            let e = dedup.entry((a, b)).or_insert(w);
            if w < *e {
                *e = w;
            }
        }
        for (&(a, b), &w) in dedup.iter() {
            adj[a as usize].push((b, w));
        }
        for a in adj.iter_mut() {
            a.sort_unstable();
        }
        let mut out = Vec::new();
        let mut onpath = vec![false; n];
        onpath[src as usize] = true;
        let mut path = vec![src];
        // DFS enumeration
        fn dfs(
            adj: &[Vec<(u32, u64)>],
            cur: u32,
            dst: u32,
            cost: u64,
            path: &mut Vec<u32>,
            onpath: &mut [bool],
            out: &mut Vec<(Vec<u32>, u64)>,
        ) {
            if cur == dst {
                out.push((path.clone(), cost));
                return;
            }
            for &(v, w) in &adj[cur as usize] {
                if onpath[v as usize] {
                    continue;
                }
                onpath[v as usize] = true;
                path.push(v);
                dfs(adj, v, dst, cost + w, path, onpath, out);
                path.pop();
                onpath[v as usize] = false;
            }
        }
        dfs(&adj, src, dst, 0, &mut path, &mut onpath, &mut out);
        out.sort_by_key(|a| (a.1, a.0.clone()));
        out
    }

    #[test]
    fn brute_force_oracle() {
        let mut rng = SplitMix64::new(0x9e1_5eed_57d1_0001);
        for _ in 0..300 {
            let n = 2 + rng.below(6) as usize;
            let m = rng.below((n * n / 2) as u32) as usize;
            let mut edges = Vec::new();
            for _ in 0..m {
                let a = rng.below(n as u32);
                let b = rng.below(n as u32);
                if a == b {
                    continue;
                }
                edges.push((a, b, rng.below(9) as u64 + 1));
            }
            let (src, dst) = (0u32, n as u32 - 1);
            let k = 1 + rng.below(4) as usize;
            let got = yen(n, &edges, src, dst, k);
            let all_paths: std::collections::BTreeSet<Vec<u32>> = {
                let full = brute(n, &edges, src, dst);
                full.iter().map(|(p, _)| p.clone()).collect()
            };
            let mut want = brute(n, &edges, src, dst);
            want.truncate(k);
            // Yen's contract is k loopless paths with the k smallest
            // costs — equal-cost tie order is implementation-defined,
            // so check the cost vector exactly, each path against the
            // brute path set, distinctness, and determinism.
            let want_costs: Vec<u64> = want.iter().map(|(_, c)| *c).collect();
            let got_costs: Vec<u64> = got.iter().map(|(_, c)| *c).collect();
            assert_eq!(got_costs, want_costs, "n={n} edges={edges:?} k={k}");
            let mut seen = std::collections::BTreeSet::new();
            for (p, c) in &got {
                assert!(all_paths.contains(p), "{p:?} not a simple path");
                assert!(seen.insert(p.clone()), "dup path {p:?}");
                assert_eq!(*c, path_cost(&edges, p));
            }
            assert_eq!(got, yen(n, &edges, src, dst, k), "not deterministic");
        }
    }

    #[test]
    fn unreachable_and_trivial() {
        assert!(yen(3, &[], 0, 2, 3).is_empty());
        assert_eq!(yen(3, &[(0, 1, 5)], 0, 0, 2), vec![(vec![0], 0)]);
        assert!(yen(3, &[(0, 1, 5)], 0, 2, 0).is_empty());
        // self-loops can't shorten anything and never appear
        let e = &[(0, 0, 1), (0, 1, 1), (1, 1, 1), (1, 2, 1)];
        assert_eq!(yen(3, e, 0, 2, 1), vec![(vec![0, 1, 2], 2)]);
    }

    #[test]
    fn parallel_edges_canonical() {
        // parallel edges: min-weight wins in the dedup.
        let e = &[(0, 1, 9), (0, 1, 2), (1, 2, 1)];
        assert_eq!(yen(3, e, 0, 2, 1), vec![(vec![0, 1, 2], 3)]);
    }
}

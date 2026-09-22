//! Biconnected components of an undirected graph — Tarjan's
//! edge-stack DFS decomposition.
//!
//! Two edges are *biconnected* when they lie on a common simple cycle
//! (equivalently: no single vertex removal separates them). The
//! partition of edges into biconnected components exposes a graph's
//! articulation structure as a set of edge bags — bridges come out as
//! singleton components, and articulation points are exactly the
//! vertices shared between components.
//!
//! Complements [`crate::graph`]'s vertex-level analysis
//! (articulation points / bridges) with the actual component
//! decomposition: which edges survive each other's failures.
//!
//! ```
//! use izanagi_kit::biconn::biconnected_components;
//! // Triangle 0-1-2 plus bridge 2-3.
//! let comps = biconnected_components(4, &[(0, 1), (1, 2), (2, 0), (2, 3)]);
//! assert_eq!(comps.len(), 2);
//! assert!(comps.iter().any(|c| c.len() == 3)); // the cycle
//! assert!(comps.iter().any(|c| c.len() == 1)); // the bridge
//! ```

/// Partition `edges` into biconnected components.
///
/// `edges` are `(u, v)` vertex pairs over `0..n` (self-loops are
/// ignored — a loop is its own trivial component but carries no
/// articulation information). Returns the components as edge-index
/// bags (`Vec` of indices into `edges`), each sorted ascending and
/// the component list sorted by first index — a pure function of the
/// input, independent of any insertion or hash ordering.
pub fn biconnected_components(n: usize, edges: &[(u32, u32)]) -> Vec<Vec<u32>> {
    // Build adjacency with edge ids, dropping self-loops.
    let mut adj: Vec<Vec<(usize, u32)>> = vec![Vec::new(); n];
    for (i, &(u, v)) in edges.iter().enumerate() {
        let (u, v) = (u as usize, v as usize);
        if u == v || u >= n || v >= n {
            continue;
        }
        adj[u].push((v, i as u32));
        adj[v].push((u, i as u32));
    }
    let mut disc = vec![0u32; n];
    let mut low = vec![0u32; n];
    let mut visited = vec![false; n];
    let mut time = 0u32;
    let mut comps: Vec<Vec<u32>> = Vec::new();

    // Iterative DFS; `stack` holds the current path's edges (as edge
    // ids). On `low[w] >= disc[v]` the subtree at v→w closes a
    // component — pop everything up to and including (v,w).
    for s in 0..n {
        if visited[s] {
            continue;
        }
        visited[s] = true;
        let mut stack_edges: Vec<u32> = Vec::new();
        // (vertex, parent edge id, next adjacency index)
        let mut st: Vec<(usize, i64, usize)> = vec![(s, -1, 0)];
        disc[s] = time;
        low[s] = time;
        time += 1;
        while let Some(top) = st.last() {
            let (v, pe, ci) = *top;
            if ci < adj[v].len() {
                let (w, eid) = adj[v][ci];
                if let Some(t) = st.last_mut() {
                    t.2 += 1;
                }
                if eid as i64 == pe {
                    continue; // the edge we arrived on
                }
                if !visited[w] {
                    visited[w] = true;
                    disc[w] = time;
                    low[w] = time;
                    time += 1;
                    stack_edges.push(eid);
                    st.push((w, eid as i64, 0));
                } else if disc[w] < disc[v] {
                    // Back edge to an ancestor (each undirected edge
                    // seen twice — only push on the descendant side).
                    low[v] = low[v].min(disc[w]);
                    stack_edges.push(eid);
                }
            } else {
                let (_, pe_v, _) = match st.pop() {
                    Some(f) => f,
                    None => break,
                };
                if let Some(&(p, _, _)) = st.last() {
                    low[p] = low[p].min(low[v]);
                    if low[v] >= disc[p] {
                        // v is the root of a biconnected component:
                        // pop the edge stack up to and including the
                        // tree edge (p, v) = v's parent edge `pe_v`.
                        let mut comp: Vec<u32> = Vec::new();
                        while let Some(&e) = stack_edges.last() {
                            stack_edges.pop();
                            comp.push(e);
                            if e as i64 == pe_v {
                                break;
                            }
                        }
                        comp.sort_unstable();
                        comps.push(comp);
                    }
                }
            }
        }
    }

    // Sort components by smallest edge index for a canonical output.
    comps.sort_by_key(|c| c.first().copied().unwrap_or(0));
    comps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Oracle: two distinct edges are biconnected iff some simple
    /// cycle contains both — enumerate all simple cycles by DFS.
    fn oracle_classes(n: usize, edges: &[(u32, u32)]) -> Vec<BTreeSet<u32>> {
        let m = edges.len();
        // co_cyclic[i][j] = edges i,j share a simple cycle.
        let mut co = vec![vec![false; m]; m];
        let adj: Vec<Vec<(usize, u32)>> = {
            let mut a = vec![Vec::new(); n];
            for (i, &(u, v)) in edges.iter().enumerate() {
                let (u, v) = (u as usize, v as usize);
                if u != v && u < n && v < n {
                    a[u].push((v, i as u32));
                    a[v].push((u, i as u32));
                }
            }
            a
        };
        // Enumerate every simple cycle: start at each vertex, walk
        // without revisiting, close back at start. Mark edge sets.
        let mut seen_cycles: BTreeSet<Vec<u32>> = BTreeSet::new();
        for s in 0..n {
            let mut path_edges: Vec<u32> = Vec::new();
            let mut path_nodes = vec![s];
            // DFS stack of (current vertex, next neighbor index).
            let mut st: Vec<(usize, usize)> = vec![(s, 0)];
            while let Some(&mut (v, ref mut ci)) = st.last_mut() {
                if *ci >= adj[v].len() {
                    st.pop();
                    path_nodes.pop();
                    path_edges.pop();
                    continue;
                }
                let (w, eid) = adj[v][*ci];
                *ci += 1;
                // Closing the cycle requires ≥1 path edge and a
                // *different* closing edge — parallel edges (u—e1→v
                // —e2→u) form a real 2-cycle in a multigraph.
                if w == s && !path_edges.is_empty() && !path_edges.contains(&eid) {
                    // Found a simple cycle: path_edges + eid.
                    let mut cyc: Vec<u32> = path_edges.clone();
                    cyc.push(eid);
                    let mut key = cyc.clone();
                    key.sort_unstable();
                    if seen_cycles.insert(key) {
                        for &a in &cyc {
                            for &b in &cyc {
                                if a != b {
                                    co[a as usize][b as usize] = true;
                                }
                            }
                        }
                    }
                } else if w != s && !path_nodes.contains(&w) {
                    path_nodes.push(w);
                    path_edges.push(eid);
                    st.push((w, 0));
                }
            }
        }
        // Equivalence classes under co_cyclic (each edge also forms its
        // own class — a bridge is a singleton, a self-loop is dropped).
        let mut used = vec![false; m];
        let mut out: Vec<BTreeSet<u32>> = Vec::new();
        for i in 0..m {
            if used[i] {
                continue;
            }
            let (u, v) = edges[i];
            if u == v || u as usize >= n || v as usize >= n {
                used[i] = true;
                continue;
            }
            let mut class = BTreeSet::from([i as u32]);
            let mut changed = true;
            while changed {
                changed = false;
                for (j, co_j) in co.iter().enumerate() {
                    if class.contains(&(j as u32)) {
                        continue;
                    }
                    if class.iter().any(|&a| co_j[a as usize]) {
                        class.insert(j as u32);
                        changed = true;
                    }
                }
            }
            for &j in &class {
                used[j as usize] = true;
            }
            out.push(class);
        }
        out.sort_by_key(|c| *c.iter().next().unwrap());
        out
    }

    fn norm(comps: &[Vec<u32>]) -> Vec<BTreeSet<u32>> {
        let mut v: Vec<BTreeSet<u32>> = comps.iter().map(|c| c.iter().copied().collect()).collect();
        v.sort_by_key(|c| *c.iter().next().unwrap());
        v
    }

    #[test]
    fn matches_cycle_enumeration_oracle() {
        let mut rng = SplitMix64::new(0xB1CC);
        for _ in 0..150 {
            let n = (rng.below(7) + 1) as usize;
            let m = rng.below(10) as usize;
            let edges: Vec<(u32, u32)> = (0..m)
                .map(|_| (rng.below(n as u32), rng.below(n as u32)))
                .collect();
            let got = biconnected_components(n, &edges);
            let want = oracle_classes(n, &edges);
            assert_eq!(norm(&got), want, "edges={edges:?} n={n}");
        }
    }

    #[test]
    fn known_shapes() {
        // Single cycle = one component.
        let c = biconnected_components(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].len(), 4);
        // Two triangles sharing an articulation vertex.
        let c = biconnected_components(5, &[(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 2)]);
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|x| x.len() == 3));
        // Bridge chain: every edge a singleton.
        let c = biconnected_components(4, &[(0, 1), (1, 2), (2, 3)]);
        assert_eq!(c.len(), 3);
        assert!(c.iter().all(|x| x.len() == 1));
        // Empty / isolated.
        assert_eq!(biconnected_components(3, &[]), Vec::<Vec<u32>>::new());
        assert_eq!(
            biconnected_components(2, &[(0, 0)]),
            Vec::<Vec<u32>>::new(),
            "self-loops carry no articulation info"
        );
    }
}

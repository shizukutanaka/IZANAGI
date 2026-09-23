//! Minimum path cover of a DAG — the classic reduction: split every
//! vertex `v` into a left copy `L_v` and a right copy `R_v`, put
//! `L_u — R_v` for each arc `u → v`, and compute a maximum matching.
//! Every matched edge is a "successor" link in a path, so the number
//! of paths needed is `n − |matching|` (Dilworth dual: a path cover
//! is a chain partition).
//!
//! The returned paths are canonical: each vertex's successor is the
//! unique matched arc, paths are listed in the order of their
//! smallest vertex's index, so the whole result is a pure function
//! of the edge set — not of matching internals.
//!
//! `None` when the input is not acyclic (a cyclic graph has no
//! topological order, and the reduction is meaningless there).
//!
//! ```
//! use izanagi_kit::pathcover;
//! // Two disjoint chains 0→1→2 and 3→4 → cover = 2 paths.
//! let adj = vec![vec![1u32], vec![2], vec![], vec![4], vec![]];
//! let cover = pathcover::path_cover(&adj).unwrap();
//! assert_eq!(cover.len(), 2);
//! assert_eq!(cover[0], vec![0, 1, 2]);
//! ```

use crate::bipartite::hopcroft_karp;

/// Topological order of the DAG `adj`, or `None` on a cycle.
pub fn topo_order(adj: &[Vec<u32>]) -> Option<Vec<u32>> {
    let n = adj.len();
    let mut indeg = vec![0u32; n];
    for outs in adj {
        for &v in outs {
            indeg[v as usize] += 1;
        }
    }
    // Smallest-index-first Kahn keeps the order canonical.
    let mut heap: std::collections::BTreeSet<u32> =
        (0..n as u32).filter(|&v| indeg[v as usize] == 0).collect();
    let mut order = Vec::with_capacity(n);
    while let Some(&v) = heap.iter().next() {
        heap.remove(&v);
        order.push(v);
        for &w in &adj[v as usize] {
            let d = &mut indeg[w as usize];
            *d -= 1;
            if *d == 0 {
                heap.insert(w);
            }
        }
    }
    if order.len() == n {
        Some(order)
    } else {
        None
    }
}

/// Minimum path cover as a list of vertex paths (sorted by each
/// path's head). `None` if `adj` has a cycle.
pub fn path_cover(adj: &[Vec<u32>]) -> Option<Vec<Vec<u32>>> {
    topo_order(adj)?;
    let n = adj.len();
    let mut bi: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (u, outs) in adj.iter().enumerate() {
        for &v in outs {
            bi[u].push(v as usize);
        }
    }
    let (_size, mate) = hopcroft_karp(&bi, n);
    // mate[u] = v means u→v is in the cover. Build paths by walking
    // successor chains; a vertex starts a path iff no arc enters it
    // in the matching.
    let mut has_pred = vec![false; n];
    for m in mate.iter().flatten() {
        has_pred[*m] = true;
    }
    let mut out: Vec<Vec<u32>> = Vec::new();
    for s in 0..n {
        if has_pred[s] {
            continue;
        }
        let mut path = vec![s as u32];
        let mut cur = mate[s];
        while let Some(v) = cur {
            path.push(v as u32);
            cur = mate[v];
        }
        out.push(path);
    }
    // `out` is already in head order (s ascends) — canonical.
    Some(out)
}

/// Just the cover size (`n − max_matching`). `None` on cycles.
pub fn min_path_cover_size(adj: &[Vec<u32>]) -> Option<usize> {
    path_cover(adj).map(|c| c.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Brute oracle: smallest k such that the vertex set splits
    /// into k chains. Enumerates assignments of "next" pointers —
    /// feasible only for n ≤ 6.
    fn brute_cover_size(adj: &[Vec<u32>]) -> usize {
        let n = adj.len();
        // A valid cover is a set of vertex-disjoint paths where each
        // step u→v is an arc (not just reachability).
        let arc: Vec<Vec<bool>> = (0..n)
            .map(|u| {
                let mut row = vec![false; n];
                for &v in &adj[u] {
                    row[v as usize] = true;
                }
                row
            })
            .collect();
        // Enumerate successor functions succ: V → V ∪ {−1} requiring
        // (a) succ[u] = v ⇒ arc u→v, (b) indegree ≤ 1, (c) acyclic.
        // Count resulting path count; take min over all valid.
        let mut best = n;
        // successor as u8 array, 255 = none
        let mut succ = vec![255u8; n];
        fn go(adj_arc: &[Vec<bool>], succ: &mut Vec<u8>, i: usize, best: &mut usize) {
            let n = adj_arc.len();
            if i == n {
                // Check: indegree ≤ 1 and no cycles.
                let mut indeg = vec![0u8; n];
                for &s in succ.iter() {
                    if s != 255 {
                        indeg[s as usize] += 1;
                    }
                }
                if indeg.iter().any(|&d| d > 1) {
                    return;
                }
                // Cycle check via walk.
                for s in 0..n {
                    let mut seen = vec![false; n];
                    let mut cur = s as u8;
                    let mut steps = 0;
                    loop {
                        if cur == 255 || steps > n {
                            break;
                        }
                        if seen[cur as usize] {
                            return; // cycle
                        }
                        seen[cur as usize] = true;
                        cur = succ[cur as usize];
                        steps += 1;
                    }
                }
                // Path count = n − (# succ links).
                let links = succ.iter().filter(|&&s| s != 255).count();
                if n - links < *best {
                    *best = n - links;
                }
                return;
            }
            // succ[i] = none.
            succ[i] = 255;
            go(adj_arc, succ, i + 1, best);
            for v in 0..n {
                if adj_arc[i][v] {
                    succ[i] = v as u8;
                    go(adj_arc, succ, i + 1, best);
                }
            }
            succ[i] = 255;
        }
        go(&arc, &mut succ, 0, &mut best);
        best
    }

    #[test]
    fn basics() {
        let adj = vec![vec![1u32], vec![2], vec![], vec![4], vec![]];
        let cover = path_cover(&adj).unwrap();
        assert_eq!(cover.len(), 2);
        assert_eq!(cover[0], vec![0, 1, 2]);
        assert_eq!(cover[1], vec![3, 4]);
        assert_eq!(min_path_cover_size(&adj), Some(2));
        // Cycle → None.
        let cyc = vec![vec![1u32], vec![0]];
        assert_eq!(path_cover(&cyc), None);
        assert_eq!(topo_order(&cyc), None);
        // No edges → n singletons.
        let empty: Vec<Vec<u32>> = vec![vec![], vec![], vec![]];
        assert_eq!(path_cover(&empty).unwrap().len(), 3);
    }

    #[test]
    fn oracle_small() {
        let mut rng = SplitMix64::new(0xd119_ab12_3412_1234);
        for _case in 0..60 {
            let n = 1 + rng.below(6) as usize;
            // Random DAG: edge i→j only for i < j.
            let mut adj = vec![Vec::<u32>::new(); n];
            for (i, out) in adj.iter_mut().enumerate() {
                for j in (i + 1)..n {
                    if rng.below(3) == 0 {
                        out.push(j as u32);
                    }
                }
            }
            let cover = path_cover(&adj).unwrap();
            // Validity: every vertex covered exactly once, every
            // consecutive pair an arc.
            let mut seen = BTreeSet::new();
            for p in &cover {
                for w in p.windows(2) {
                    assert!(adj[w[0] as usize].contains(&w[1]));
                }
                for &v in p {
                    assert!(seen.insert(v));
                }
            }
            assert_eq!(seen.len(), n);
            // Optimality against the brute oracle (n ≤ 6 → cheap).
            assert_eq!(cover.len(), brute_cover_size(&adj));
        }
    }

    #[test]
    fn topo() {
        let adj = vec![vec![1u32, 2], vec![3], vec![3], vec![]];
        let order = topo_order(&adj).unwrap();
        // Every edge goes earlier → later in the order.
        let pos = |v: u32| order.iter().position(|&x| x == v).unwrap();
        for (u, outs) in adj.iter().enumerate() {
            for &v in outs {
                assert!(pos(u as u32) < pos(v));
            }
        }
    }
}

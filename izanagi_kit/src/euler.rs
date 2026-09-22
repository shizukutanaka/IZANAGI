//! Eulerian walks — Hierholzer's algorithm on an undirected
//! multigraph: a path/circuit that traverses every edge exactly once.
//!
//! Complements [`crate::tsp`]: `tsp` answers "visit every *vertex*
//! once" (patrol routes), this answers "traverse every *edge* once" —
//! snowplow / inspection / draw-without-lifting-the-pen quests, and
//! floor-plan coverage checks.
//!
//! Determinism: edges are consumed in input order (the first unused
//! edge out of a vertex wins), so the returned walk is a pure function
//! of the edge list. Vertex ids are `u32` and need not be dense.
//!
//! ```
//! use izanagi_kit::euler::{euler_walk, EulerKind};
//! // A 4-cycle is an Eulerian circuit: ends where it started.
//! let edges = vec![(0u32,1u32),(1,2),(2,3),(3,0)];
//! let (kind, walk) = euler_walk(&edges).unwrap();
//! assert_eq!(kind, EulerKind::Circuit);
//! assert_eq!(walk.len(), 5);
//! assert_eq!(walk[0], walk[4]);
//! ```

/// Whether a returned walk is a circuit (all degrees even,
/// `walk[0] == walk[n-1]`) or an open path (exactly two odd degrees).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EulerKind {
    /// All degrees even — walk starts and ends at the same vertex.
    Circuit,
    /// Exactly two odd degrees — open walk between them.
    Path,
}

/// Hierholzer walk covering every edge of `edges` exactly once.
///
/// Returns `Some((kind, walk))` with `walk.len() == edges.len() + 1`
/// when an Eulerian walk exists, `None` when the edge-connected part is
/// disconnected or has more than two odd-degree vertices. `walk` is a
/// vertex sequence; consecutive pairs are edges of the input.
///
/// Empty input is a degenerate circuit — `Some((Circuit, vec![0]))`.
/// Isolated vertices are ignored (they never appear in `edges`).
pub fn euler_walk(edges: &[(u32, u32)]) -> Option<(EulerKind, Vec<u32>)> {
    if edges.is_empty() {
        return Some((EulerKind::Circuit, vec![0]));
    }

    // Dense-index the vertex set (sorted, so the start pick is canonical).
    let mut verts: Vec<u32> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
    verts.sort_unstable();
    verts.dedup();
    let index_of = |v: u32| -> usize {
        verts.binary_search(&v).unwrap_or(0) // v always present
    };
    let n = verts.len();
    let m = edges.len();

    // Adjacency: for each vertex, its incident edge ids in input order.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut deg = vec![0usize; n];
    for (id, &(a, b)) in edges.iter().enumerate() {
        let (ia, ib) = (index_of(a), index_of(b));
        adj[ia].push(id);
        if ia != ib {
            adj[ib].push(id);
        }
        deg[ia] += 1;
        deg[ib] += 1;
    }

    // Euler's condition: 0 or 2 odd-degree vertices.
    let odd: Vec<usize> = (0..n).filter(|&v| deg[v] % 2 == 1).collect();
    let kind = match odd.len() {
        0 => EulerKind::Circuit,
        2 => EulerKind::Path,
        _ => return None,
    };

    // Connectivity of the non-isolated part: BFS from vertex of edge 0.
    let start_component = index_of(edges[0].0);
    let mut seen = vec![false; n];
    let mut q = std::collections::VecDeque::from([start_component]);
    seen[start_component] = true;
    while let Some(v) = q.pop_front() {
        for &eid in &adj[v] {
            let (a, b) = edges[eid];
            for w in [a, b] {
                let iw = index_of(w);
                if !seen[iw] {
                    seen[iw] = true;
                    q.push_back(iw);
                }
            }
        }
    }
    if (0..n).any(|v| deg[v] > 0 && !seen[v]) {
        return None; // edges split across components
    }

    // Start vertex: the odd endpoint with the smaller index for a path,
    // else the endpoint of edge 0 — deterministic either way.
    let start = match kind {
        EulerKind::Path => odd[0].min(odd[1]),
        EulerKind::Circuit => start_component,
    };

    // Hierholzer: explicit stack + per-vertex edge cursors. First unused
    // edge in input order wins — fully deterministic.
    let mut used = vec![false; m];
    let mut it = vec![0usize; n];
    let mut stack = vec![start];
    let mut tour: Vec<usize> = Vec::with_capacity(m + 1);
    while let Some(&v) = stack.last() {
        let mut advanced = false;
        while it[v] < adj[v].len() {
            let eid = adj[v][it[v]];
            it[v] += 1;
            if !used[eid] {
                used[eid] = true;
                let (a, b) = edges[eid];
                stack.push(if index_of(a) == v {
                    index_of(b)
                } else {
                    index_of(a)
                });
                advanced = true;
                break;
            }
        }
        if !advanced {
            tour.push(v);
            stack.pop();
        }
    }
    tour.reverse();

    let walk: Vec<u32> = tour.into_iter().map(|i| verts[i]).collect();
    if walk.len() == m + 1 {
        Some((kind, walk))
    } else {
        None // unreachable given the checks, kept as the honest tail
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    /// Oracle: verify `walk` is a valid Eulerian traversal of `edges` —
    /// consecutive pairs match input edges, each edge used exactly once.
    fn valid_walk(edges: &[(u32, u32)], walk: &[u32]) -> bool {
        if walk.len() != edges.len() + 1 {
            return false;
        }
        // Multiset of canonical edge pairs.
        let mut want: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        for &(a, b) in edges {
            let key = if a <= b { (a, b) } else { (b, a) };
            *want.entry(key).or_insert(0) += 1;
        }
        for w in walk.windows(2) {
            let (a, b) = (w[0], w[1]);
            let key = if a <= b { (a, b) } else { (b, a) };
            match want.get_mut(&key) {
                Some(c) if *c > 0 => *c -= 1,
                _ => return false,
            }
        }
        want.values().all(|&c| c == 0)
    }

    #[test]
    fn circuits_paths_and_rejections() {
        // Circuit: triangle.
        let e = vec![(0u32, 1u32), (1, 2), (2, 0)];
        let (k, w) = euler_walk(&e).unwrap();
        assert_eq!(k, EulerKind::Circuit);
        assert!(w[0] == w[w.len() - 1]);
        assert!(valid_walk(&e, &w));
        // Path: single edge chain 0-1-2-3 has odd endpoints 0,3.
        let e = vec![(0u32, 1u32), (1, 2), (2, 3)];
        let (k, w) = euler_walk(&e).unwrap();
        assert_eq!(k, EulerKind::Path);
        assert!(valid_walk(&e, &w));
        assert!(w[0] == w[0].min(w[w.len() - 1])); // starts at odd endpoint
                                                   // Reject: 4 odd (star K1,3).
        let star = vec![(0u32, 1u32), (0, 2), (0, 3)];
        assert_eq!(euler_walk(&star), None);
        // Reject: two disconnected edges.
        let disc = vec![(0u32, 1u32), (5, 7)];
        assert_eq!(euler_walk(&disc), None);
        // Empty is a degenerate circuit.
        assert_eq!(euler_walk(&[]), Some((EulerKind::Circuit, vec![0])));
    }

    #[test]
    fn parallel_edges_and_self_loops() {
        // Two parallel edges between 0 and 1: degree(0)=degree(1)=2.
        let e = vec![(0u32, 1u32), (0, 1)];
        let (k, w) = euler_walk(&e).unwrap();
        assert_eq!(k, EulerKind::Circuit);
        assert!(valid_walk(&e, &w));
        // Self-loop counts as degree 2: loop at 0 + two 0-1 parallels
        // → deg(0)=4, deg(1)=2 → circuit.
        let e = vec![(0u32, 0u32), (0, 1), (0, 1)];
        let (k, w) = euler_walk(&e).unwrap();
        assert_eq!(k, EulerKind::Circuit);
        assert!(valid_walk(&e, &w));
        // Loop + single edge: deg(0)=3, deg(1)=1 → open path 0→1 or 1→0.
        let e = vec![(0u32, 0u32), (0, 1)];
        let (k, w) = euler_walk(&e).unwrap();
        assert_eq!(k, EulerKind::Path);
        assert!(valid_walk(&e, &w));
    }

    #[test]
    fn random_eulerian_multigraphs_round_trip() {
        let mut rng = SplitMix64::new(0xE0E1);
        for _ in 0..200 {
            // Generate an Eulerian graph by construction: a random
            // closed walk's edge multiset is always a circuit.
            let nv = 2 + rng.below(8) as usize;
            let mut edges = Vec::new();
            let mut cur = rng.below(nv as u32);
            let start = cur;
            let steps = 1 + rng.below(20) as usize;
            for i in 0..steps {
                let next = if i + 1 == steps {
                    start
                } else {
                    rng.below(nv as u32)
                };
                edges.push((cur, next));
                cur = next;
            }
            let (k, w) = euler_walk(&edges).unwrap();
            assert_eq!(k, EulerKind::Circuit);
            assert!(valid_walk(&edges, &w));
        }
    }

    #[test]
    fn walk_is_deterministic_across_runs() {
        let e = vec![(0u32, 1u32), (1, 2), (2, 0), (2, 3), (3, 4), (4, 2)];
        assert_eq!(euler_walk(&e), euler_walk(&e));
    }
}

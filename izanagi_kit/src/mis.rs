//! Deterministic greedy maximal independent set. Vertices are
//! examined in index order: `v` joins the set iff no neighbor of `v`
//! already has. The result is the unique *canonical* MIS of the edge
//! set — lexicographically-first under the greedy rule — so it is a
//! pure function of `(n, edges)`, never of adjacency iteration order.
//!
//! Greedy MIS is not maximum, but it is maximal (no vertex can be
//! added) and, on bounded-degree graphs, within a `Δ+1` factor of
//! optimal. It is the standard tool for picking non-adjacent
//! representatives: lockstep peer sets, non-conflicting spawns on a
//! graph, coloring round-0.
//!
//! ```
//! use izanagi_kit::mis::maximal_independent_set;
//! let s = maximal_independent_set(4, &[(0, 1), (1, 2), (2, 3)]).unwrap();
//! assert_eq!(s, vec![0, 2]);
//! ```

/// The canonical greedy MIS: ascending vertex order, `v` is included
/// iff no included vertex is adjacent to `v`.
///
/// `edges` may be given in either direction and with duplicates;
/// self-loops disqualify their vertex (a vertex adjacent to itself
/// can never be independent). Returns `None` on an out-of-range
/// endpoint.
pub fn maximal_independent_set(n: usize, edges: &[(u32, u32)]) -> Option<Vec<u32>> {
    let mut adj = vec![Vec::new(); n];
    for &(a, b) in edges {
        if a as usize >= n || b as usize >= n {
            return None;
        }
        adj[a as usize].push(b);
        if a != b {
            adj[b as usize].push(a);
        }
    }
    for l in adj.iter_mut() {
        l.sort_unstable();
        l.dedup();
    }
    let mut chosen = vec![false; n];
    for (v, nbrs) in adj.iter().enumerate() {
        if nbrs.iter().any(|&u| chosen[u as usize] || u as usize == v) {
            continue;
        }
        chosen[v] = true;
    }
    Some(
        chosen
            .iter()
            .enumerate()
            .filter(|&(_, &c)| c)
            .map(|(i, _)| i as u32)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn verify_mis(n: usize, edges: &[(u32, u32)], s: &[u32]) {
        let set: BTreeSet<u32> = s.iter().copied().collect();
        assert_eq!(set.len(), s.len(), "duplicates in MIS");
        for &(a, b) in edges {
            assert!(!(a == b && set.contains(&a)), "self-loop vertex {a} in MIS");
            assert!(
                !(set.contains(&a) && set.contains(&b)),
                "edge ({a},{b}) has both endpoints in MIS"
            );
        }
        for v in 0..n as u32 {
            if set.contains(&v) {
                continue;
            }
            // A self-looped vertex can never join any IS — exempt it
            // from maximality coverage.
            if edges.iter().any(|&(a, b)| a == v && b == v) {
                continue;
            }
            let covered = edges
                .iter()
                .any(|&(a, b)| (a == v && set.contains(&b)) || (b == v && set.contains(&a)));
            assert!(covered, "MIS not maximal: {v} could be added");
        }
    }

    #[test]
    fn greedy_result_is_a_valid_maximal_is() {
        let mut rng = SplitMix64::new(23);
        for _ in 0..60 {
            let n = rng.below(24) as usize + 1;
            let m = rng.below(n as u32 * 3) as usize;
            let edges: Vec<(u32, u32)> = (0..m)
                .map(|_| (rng.below(n as u32), rng.below(n as u32)))
                .collect();
            let s = maximal_independent_set(n, &edges).unwrap();
            verify_mis(n, &edges, &s);
        }
    }

    #[test]
    fn canonical_regardless_of_edge_order() {
        let mut e1 = vec![(0, 1), (2, 3), (1, 2), (3, 0)];
        let s1 = maximal_independent_set(4, &e1).unwrap();
        e1.reverse();
        let s2 = maximal_independent_set(4, &e1).unwrap();
        assert_eq!(s1, s2);
        assert_eq!(s1, vec![0, 2]);
    }

    #[test]
    fn known_shapes() {
        // Path 0-1-2-3-4: lexicographic greedy takes 0,2,4.
        assert_eq!(
            maximal_independent_set(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]),
            Some(vec![0, 2, 4])
        );
        // Star centered at 0: greedy takes 0, spokes excluded.
        assert_eq!(
            maximal_independent_set(5, &[(0, 1), (0, 2), (0, 3), (0, 4)]),
            Some(vec![0])
        );
        // Empty edge set: everything.
        assert_eq!(maximal_independent_set(3, &[]), Some(vec![0, 1, 2]));
        // Out of range rejected.
        assert_eq!(maximal_independent_set(2, &[(0, 5)]), None);
    }
}

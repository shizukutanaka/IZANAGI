//! k-core decomposition — Batagelj & Zaveršnik's degree-peeling:
//! the *coreness* of a vertex is the largest `k` such that it
//! belongs to a subgraph where every vertex has degree ≥ `k`
//! (the `k`-core). `degeneracy` is the maximum coreness — the
//! graph's density spine, and the ordering backbone of
//! fixed-parameter colorings and clique bounds.
//!
//! Runs `O(n + m)`: bucket vertices by current degree, peel the
//! minimum, decrement unpeeled neighbors. Vertex ids are
//! `0..n`, parallel edges and loops count as full degree (an
//! input-contract choice — callers dedupe).
//!
//! ```
//! use izanagi_kit::kcore::{coreness, degeneracy};
//!
//! // Triangle + pendant: the triangle vertices have coreness 2,
//! // the pendant 1.
//! let edges = vec![(0u32, 1u32), (1, 2), (2, 0), (2, 3)];
//! assert_eq!(coreness(4, &edges), vec![2, 2, 2, 1]);
//! assert_eq!(degeneracy(4, &edges), 2);
//! ```

/// Coreness per vertex — `out[v] = k` when `v` sits in the k-core
/// but not the (k+1)-core.
pub fn coreness(n: usize, edges: &[(u32, u32)]) -> Vec<u32> {
    let mut deg = vec![0u32; n];
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        let (a, b) = (a as usize, b as usize);
        if a < n && b < n {
            deg[a] += 1;
            deg[b] += 1;
            adj[a].push(b as u32);
            adj[b].push(a as u32);
        }
    }
    // Bucket queue: pos[d] chains of vertices at current degree d.
    let max_d = deg.iter().copied().max().unwrap_or(0) as usize;
    let mut bin = vec![0usize; max_d + 1]; // bin[d] = count at degree d
    for &d in &deg {
        bin[d as usize] += 1;
    }
    let mut start = vec![0usize; max_d + 1];
    let mut acc = 0;
    for d in 0..=max_d {
        start[d] = acc;
        acc += bin[d];
    }
    // vert[] = vertices sorted by current degree; pos[v] = slot.
    let mut vert = vec![0u32; n];
    let mut pos = vec![0usize; n];
    let mut fill = start.clone();
    for (v, &d) in deg.iter().enumerate() {
        pos[v] = fill[d as usize];
        vert[pos[v]] = v as u32;
        fill[d as usize] += 1;
    }
    let mut core = vec![0u32; n];
    for i in 0..n {
        let v = vert[i] as usize;
        core[v] = deg[v];
        // Neighbors of v that out-degree it (i.e. unpeeled):
        // each loses one edge and bubbles one bin left.
        for &u32_u in &adj[v] {
            let u = u32_u as usize;
            if deg[u] > deg[v] {
                // Swap u with the first vertex of its degree bin,
                // then shift that bin's start right.
                let du = deg[u] as usize;
                let pu = pos[u];
                let pw = start[du];
                let w = vert[pw] as usize;
                vert[pu] = w as u32;
                vert[pw] = u as u32;
                pos[u] = pw;
                pos[w] = pu;
                start[du] += 1;
                deg[u] -= 1;
            }
        }
    }
    core
}

/// The graph's degeneracy — `max` over all coreness values.
pub fn degeneracy(n: usize, edges: &[(u32, u32)]) -> u32 {
    coreness(n, edges).into_iter().max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute_coreness(n: usize, edges: &[(u32, u32)]) -> Vec<u32> {
        // Oracle: for each k, iteratively delete vertices with
        // degree < k; a vertex's coreness is the largest k whose
        // deletion process never removes it.
        let mut out = vec![0u32; n];
        for v in 0..n {
            for k in 1..=(n as u32) {
                let mut alive = vec![true; n];
                let mut deg = vec![0u32; n];
                for &(a, b) in edges {
                    deg[a as usize] += 1;
                    deg[b as usize] += 1;
                }
                let mut changed = true;
                while changed {
                    changed = false;
                    for u in 0..n {
                        if alive[u] && deg[u] < k {
                            alive[u] = false;
                            changed = true;
                            for &(a, b) in edges {
                                let (a, b) = (a as usize, b as usize);
                                if a == u && alive[b] {
                                    deg[b] -= 1;
                                }
                                if b == u && alive[a] {
                                    deg[a] -= 1;
                                }
                            }
                        }
                    }
                }
                if alive[v] {
                    out[v] = k;
                } else {
                    break;
                }
            }
        }
        out
    }

    #[test]
    fn known_graphs() {
        // K4: all coreness 3.
        let k4: Vec<_> = (0u32..4)
            .flat_map(|a| (a + 1..4).map(move |b| (a, b)))
            .collect();
        assert_eq!(coreness(4, &k4), vec![3; 4]);
        // Path: all 1. Empty: all 0.
        assert_eq!(coreness(4, &[(0, 1), (1, 2), (2, 3)]), vec![1; 4]);
        assert_eq!(coreness(4, &[]), vec![0; 4]);
        // Double triangle joined by one edge: all 2 except none 3.
        let two = vec![(0u32, 1u32), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3), (2, 3)];
        assert_eq!(coreness(6, &two), vec![2, 2, 2, 2, 2, 2]);
        // K4 plus a triangle sharing vertex 1: the K4 stays 3-core;
        // the triangle-only pair peaks at 2 (degree 2 each).
        let joined = vec![
            (0u32, 1u32),
            (0, 2),
            (0, 3),
            (1, 2),
            (1, 3),
            (2, 3),
            (1, 4),
            (1, 5),
            (4, 5),
        ];
        assert_eq!(coreness(6, &joined), vec![3, 3, 3, 3, 2, 2]);
    }

    #[test]
    fn matches_brute_oracle() {
        // Deterministic graph family: compare against k-deletion
        // oracle on all edge subsets of small graphs? Too big —
        // use seeded pseudo-random graphs via SplitMix64-free LCG.
        let mut s = 0x9e3779b9u64;
        let mut next = || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (s >> 33) as u32
        };
        for case in 0..40 {
            let n = 2 + (next() % 9) as usize;
            let mut edges = Vec::new();
            for a in 0..n {
                for b in (a + 1)..n {
                    if next() % 3 == 0 {
                        edges.push((a as u32, b as u32));
                    }
                }
            }
            assert_eq!(
                coreness(n, &edges),
                brute_coreness(n, &edges),
                "case {case} edges {edges:?}"
            );
        }
    }

    #[test]
    fn degeneracy_is_max_coreness() {
        let e = vec![(0u32, 1u32), (1, 2), (2, 0), (2, 3), (3, 4)];
        assert_eq!(degeneracy(5, &e), 2);
        assert_eq!(degeneracy(0, &[]), 0);
    }

    #[test]
    fn out_of_range_edges_ignored() {
        assert_eq!(coreness(2, &[(0u32, 7u32), (0, 1)]), vec![1, 1]);
    }

    #[test]
    fn deterministic_twice() {
        let e = vec![(0u32, 1u32), (2, 3), (1, 3)];
        assert_eq!(coreness(4, &e), coreness(4, &e));
    }
}

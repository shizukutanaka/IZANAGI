//! 0–1 BFS and Dial's bucket shortest paths — linear-ish SSSP for
//! graphs whose edge weights are tiny non-negative integers.
//!
//! Two classical specializations of Dijkstra that trade the priority
//! queue for plain `Vec` machinery:
//!
//! - [`zero_one_bfs`]: every edge weighs `0` or `1`; a `VecDeque`
//!   replaces the heap (0-edges push front, 1-edges push back) —
//!   `O(V + E)` exactly.
//! - [`dial`]: edge weights in `0..=cap`; distances are bounded by
//!   `cap·(V−1)`, so an array of `cap·(V−1)+1` buckets indexed by
//!   distance replaces the heap — `O(cap·V + E)` worst case,
//!   effectively `O(V + E)` when `cap` is small.
//!
//! Both return [`crate::bellman::Shortest`]-style results so the
//! outputs slot into existing path tooling. Weights are `u32`;
//! `zero_one_bfs` rejects any edge weighing more than `1` (`None`),
//! and `dial` rejects edges above the declared `cap` or weights the
//! caller under-declared — fail closed, never silently wrong.
//!
//! ```
//! use izanagi_kit::zerobfs::zero_one_bfs;
//! // 0→1 (cost 1), 1→2 (cost 0), 0→2 (cost 1): optimal is 1.
//! let d = zero_one_bfs(3, &[(0, 1, 1), (1, 2, 0), (0, 2, 1)], 0).unwrap();
//! assert_eq!(d[2], Some(1));
//! ```

use std::collections::VecDeque;

/// Shortest distances from `src` in a graph with edge weights in
/// `{0, 1}` — `dist[v]` is `None` for unreachable `v`.
///
/// `edges` is `(from, to, weight)` directed; each edge weighing `0`
/// goes on the front of the deque, each `1` on the back, keeping the
/// queue monotone without a heap. `None` when `src >= n` or any edge
/// weighs more than `1`.
pub fn zero_one_bfs(n: usize, edges: &[(u32, u32, u32)], src: usize) -> Option<Vec<Option<u32>>> {
    if src >= n {
        return None;
    }
    let mut adj: Vec<Vec<(usize, u32)>> = vec![Vec::new(); n];
    for &(u, v, w) in edges {
        if u as usize >= n || v as usize >= n {
            continue;
        }
        if w > 1 {
            return None; // caller lied about the weight bound
        }
        adj[u as usize].push((v as usize, w));
    }
    let mut dist: Vec<Option<u32>> = vec![None; n];
    dist[src] = Some(0);
    let mut dq: VecDeque<usize> = VecDeque::from([src]);
    while let Some(u) = dq.pop_front() {
        let du = dist[u];
        for &(v, w) in &adj[u] {
            let cand = du.map(|d| d + w);
            let better = match (dist[v], cand) {
                (None, Some(_)) => true,
                (Some(dv), Some(c)) => c < dv,
                _ => false,
            };
            if better {
                dist[v] = cand;
                if w == 0 {
                    dq.push_front(v);
                } else {
                    dq.push_back(v);
                }
            }
        }
    }
    Some(dist)
}

/// Shortest distances via Dial's bucket queue — edge weights in
/// `0..=cap`, so at most `cap · (n−1)` distinct distances are
/// reachable and a slot-per-distance array replaces the heap.
///
/// `None` when `src >= n` or any edge weighs more than `cap` (the
/// declared bound must be honest — under-declaring would silently
/// truncate the distance space). Vertices are stored in the bucket of
/// their tentative distance; once a bucket is popped its distance is
/// final (standard Dial argument: the walk only moves forward).
pub fn dial(n: usize, edges: &[(u32, u32, u32)], src: usize, cap: u32) -> Option<Vec<Option<u32>>> {
    if src >= n {
        return None;
    }
    let mut adj: Vec<Vec<(usize, u32)>> = vec![Vec::new(); n];
    for &(u, v, w) in edges {
        if u as usize >= n || v as usize >= n {
            continue;
        }
        if w > cap {
            return None;
        }
        adj[u as usize].push((v as usize, w));
    }
    let bound = cap as usize * n.saturating_sub(1);
    // buckets[d] holds vertices whose tentative distance equals d;
    // the walk scans d = 0..=bound, never revisiting.
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); bound + 1];
    let mut dist: Vec<Option<u32>> = vec![None; n];
    dist[src] = Some(0);
    buckets[0].push(src);
    for d in 0..=bound {
        let mut i = 0;
        while i < buckets[d].len() {
            let u = buckets[d][i];
            i += 1;
            if dist[u] != Some(d as u32) {
                continue; // stale entry — a shorter write replaced it
            }
            for &(v, w) in &adj[u] {
                let nd = (d as u32) + w;
                let better = match dist[v] {
                    None => true,
                    Some(dv) => nd < dv,
                };
                if better {
                    dist[v] = Some(nd);
                    buckets[nd as usize].push(v);
                }
            }
        }
    }
    Some(dist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bellman;
    use crate::rng::SplitMix64;

    /// Oracle: Bellman–Ford over the same edges (non-negative weights).
    fn oracle(n: usize, edges: &[(u32, u32, u32)], src: usize) -> Vec<Option<u32>> {
        let e: Vec<(usize, usize, i64)> = edges
            .iter()
            .map(|&(u, v, w)| (u as usize, v as usize, w as i64))
            .collect();
        bellman::shortest(n, &e, src)
            .map(|s| s.dist.iter().map(|d| d.map(|x| x as u32)).collect())
            .unwrap_or_else(|| vec![None; n])
    }

    #[test]
    fn zero_one_matches_bellman() {
        let mut rng = SplitMix64::new(0x0B5F);
        for _ in 0..300 {
            let n = (rng.below(10) + 1) as usize;
            let m = rng.below(20) as usize;
            let edges: Vec<(u32, u32, u32)> = (0..m)
                .map(|_| (rng.below(n as u32), rng.below(n as u32), rng.below(2)))
                .collect();
            let src = rng.below(n as u32) as usize;
            assert_eq!(zero_one_bfs(n, &edges, src), Some(oracle(n, &edges, src)));
        }
    }

    #[test]
    fn dial_matches_bellman() {
        let mut rng = SplitMix64::new(0xDA11);
        for _ in 0..300 {
            let n = (rng.below(10) + 1) as usize;
            let cap = rng.below(5) + 1;
            let m = rng.below(20) as usize;
            let edges: Vec<(u32, u32, u32)> = (0..m)
                .map(|_| (rng.below(n as u32), rng.below(n as u32), rng.below(cap + 1)))
                .collect();
            let src = rng.below(n as u32) as usize;
            assert_eq!(dial(n, &edges, src, cap), Some(oracle(n, &edges, src)));
        }
    }

    #[test]
    fn known_topologies() {
        // Pure 0-weight chain: everything reachable at cost 0.
        let d = zero_one_bfs(4, &[(0, 1, 0), (1, 2, 0), (2, 3, 0)], 0).unwrap();
        assert_eq!(d, vec![Some(0); 4]);
        // Dial with cap 0 == BFS on 0-edges only.
        let d = dial(3, &[(0, 1, 0), (1, 2, 0)], 0, 0).unwrap();
        assert_eq!(d, vec![Some(0), Some(0), Some(0)]);
        // Unreached stays None.
        let d = zero_one_bfs(3, &[(0, 1, 1)], 0).unwrap();
        assert_eq!(d[2], None);
        // Parallel edges: the lighter one wins.
        let d = dial(3, &[(0, 1, 5), (0, 1, 2), (1, 2, 1), (0, 2, 9)], 0, 9).unwrap();
        assert_eq!(d[2], Some(3));
    }

    #[test]
    fn rejects_bad_bounds() {
        // Weight 2 declared for 0-1 BFS.
        assert_eq!(zero_one_bfs(2, &[(0, 1, 2)], 0), None);
        // Edge over declared cap.
        assert_eq!(dial(2, &[(0, 1, 5)], 0, 3), None);
        // Source out of range / empty graph.
        assert_eq!(zero_one_bfs(2, &[], 2), None);
        assert_eq!(dial(0, &[], 0, 1), None);
    }
}

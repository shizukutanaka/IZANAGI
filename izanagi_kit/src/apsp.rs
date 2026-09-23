//! All-pairs shortest paths: Floyd–Warshall and Johnson.
//!
//! - [`floyd_warshall`] — `O(n³)` dense-matrix DP over `i64`
//!   weights with `i128` internal accumulation (a path can sum
//!   past `i64`); negative diagonal ⇒ negative cycle.
//! - [`johnson`] — `O(n·m + n²·log n)` for sparse graphs:
//!   Bellman–Ford potentials `h[v]` reweight every edge to
//!   non-negative (`w' = w + h[u] − h[v]`), then one Dijkstra
//!   per source. Returns `None` on any negative cycle.
//!
//! Distances are `Option<i64>` — `None` = unreachable.
//!
//! ```
//! use izanagi_kit::apsp::{floyd_warshall, johnson};
//! let e = [(0usize, 1usize, 3i64), (1, 2, -2), (0, 2, 4)];
//! let fw = floyd_warshall(3, &e);
//! let j = johnson(3, &e).unwrap();
//! assert_eq!(fw, j);
//! assert_eq!(fw[0][2], Some(1)); // 0→1→2 beats direct 4
//! ```

use std::cmp::Reverse;
use std::collections::BinaryHeap;

const INF: i128 = i128::MAX / 4;

fn to_i64(x: i128) -> Option<i64> {
    if x >= INF {
        None
    } else {
        i64::try_from(x).ok()
    }
}

/// Floyd–Warshall on `n` vertices. `edges` are
/// `(from, to, weight)`; parallel edges keep the min weight.
/// After the run, `d[v][v] < 0` iff `v` sits on a reachable
/// negative cycle (check [`has_negative_cycle`]).
pub fn floyd_warshall(n: usize, edges: &[(usize, usize, i64)]) -> Vec<Vec<Option<i64>>> {
    let mut d = vec![vec![INF; n]; n];
    for (v, row) in d.iter_mut().enumerate() {
        row[v] = 0;
    }
    for &(u, v, w) in edges {
        if u < n && v < n {
            d[u][v] = d[u][v].min(i128::from(w));
        }
    }
    for k in 0..n {
        for i in 0..n {
            if d[i][k] >= INF {
                continue;
            }
            for j in 0..n {
                let cand = d[i][k] + d[k][j];
                if cand < d[i][j] {
                    d[i][j] = cand;
                }
            }
        }
    }
    d.into_iter()
        .map(|row| row.into_iter().map(to_i64).collect())
        .collect()
}

/// `true` when the graph has any negative cycle — detected
/// from a raw Floyd–Warshall pass (kept separate so callers
/// can decide before reading distances).
pub fn has_negative_cycle(n: usize, edges: &[(usize, usize, i64)]) -> bool {
    // re-run the internal matrix and inspect the diagonal
    let mut d = vec![vec![INF; n]; n];
    for (v, row) in d.iter_mut().enumerate() {
        row[v] = 0;
    }
    for &(u, v, w) in edges {
        if u < n && v < n {
            d[u][v] = d[u][v].min(i128::from(w));
        }
    }
    for k in 0..n {
        for i in 0..n {
            if d[i][k] >= INF {
                continue;
            }
            for j in 0..n {
                let cand = d[i][k] + d[k][j];
                if cand < d[i][j] {
                    d[i][j] = cand;
                }
            }
        }
    }
    (0..n).any(|v| d[v][v] < 0)
}

/// Dijkstra on non-negative `i128` weights from `src`.
fn dijkstra(adj: &[Vec<(usize, i128)>], src: usize) -> Vec<i128> {
    let n = adj.len();
    let mut dist = vec![INF; n];
    let mut heap = BinaryHeap::new();
    dist[src] = 0;
    heap.push(Reverse((0i128, src)));
    while let Some(Reverse((du, u))) = heap.pop() {
        if du > dist[u] {
            continue;
        }
        for &(v, w) in &adj[u] {
            let cand = du + w;
            if cand < dist[v] {
                dist[v] = cand;
                heap.push(Reverse((cand, v)));
            }
        }
    }
    dist
}

/// Johnson's algorithm — `None` on negative cycle.
/// Reuses [`crate::bellman::shortest`] for the potential
/// computation (super-source `0`-edges to every vertex).
pub fn johnson(n: usize, edges: &[(usize, usize, i64)]) -> Option<Vec<Vec<Option<i64>>>> {
    // potentials via super-source
    let mut be: Vec<(usize, usize, i64)> = edges
        .iter()
        .copied()
        .filter(|&(u, v, _)| u < n && v < n)
        .collect();
    for v in 0..n {
        be.push((n, v, 0));
    }
    let bf = crate::bellman::shortest(n + 1, &be, n)?;
    let h: Vec<i128> = bf
        .dist
        .iter()
        .take(n)
        .map(|d| i128::from(d.unwrap_or(0)))
        .collect();

    // reweighted adjacency — w' = w + h[u] - h[v] ≥ 0 when no
    // negative cycle (triangle inequality on potentials)
    let mut adj = vec![Vec::<(usize, i128)>::new(); n];
    for &(u, v, w) in edges {
        if u < n && v < n {
            adj[u].push((v, i128::from(w) + h[u] - h[v]));
        }
    }

    let mut out = vec![vec![None; n]; n];
    for (u, row) in out.iter_mut().enumerate() {
        let dp = dijkstra(&adj, u);
        for (v, cell) in row.iter_mut().enumerate() {
            if dp[v] < INF {
                *cell = to_i64(dp[v] - h[u] + h[v]);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute(n: usize, edges: &[(usize, usize, i64)]) -> Vec<Vec<Option<i64>>> {
        // single-source Bellman-Ford per vertex — an
        // independent third implementation
        (0..n)
            .map(|s| {
                crate::bellman::shortest(n, edges, s)
                    .map(|r| r.dist)
                    .unwrap_or_else(|| vec![None; n])
            })
            .collect()
    }

    #[test]
    fn basics() {
        let e = [(0usize, 1usize, 3i64), (1, 2, -2), (0, 2, 4), (2, 3, 5)];
        let fw = floyd_warshall(4, &e);
        assert_eq!(fw[0][3], Some(6)); // 3-2+5
        assert_eq!(fw[3][0], None);
        assert_eq!(fw[0][0], Some(0));
        assert!(!has_negative_cycle(4, &e));
        assert_eq!(johnson(4, &e).unwrap(), fw);
    }

    #[test]
    fn negative_cycle_detected() {
        let e = [(0usize, 1usize, 1i64), (1, 2, -5), (2, 0, 2)];
        assert!(has_negative_cycle(3, &e));
        assert!(johnson(3, &e).is_none());
        // unreachable negative cycle still poisons the matrix
        let e2 = [(0usize, 1usize, 1i64), (2, 3, -5), (3, 2, 1)];
        assert!(has_negative_cycle(4, &e2));
    }

    /// Disconnected & empty graphs.
    #[test]
    fn degenerate() {
        assert_eq!(floyd_warshall(0, &[]), Vec::<Vec<Option<i64>>>::new());
        let d = floyd_warshall(3, &[]);
        assert_eq!(d[0][1], None);
        assert_eq!(d[2][2], Some(0));
        assert_eq!(johnson(3, &[]).unwrap(), d);
        // out-of-range endpoints skipped, not panicked
        let e = [(0usize, 9usize, 1i64)];
        assert_eq!(floyd_warshall(2, &e)[0][1], None);
    }

    /// Random graphs — johnson == floyd_warshall == per-source
    /// Bellman-Ford, weights spanning negative-but-acyclic.
    #[test]
    fn oracle_three_way_agreement() {
        let mut rng = crate::rng::SplitMix64::new(0xA11A55);
        for _ in 0..60 {
            let n = 2 + rng.below(9) as usize;
            let m = rng.below((n * n) as u32 + 1) as usize;
            let mut edges = Vec::new();
            for _ in 0..m {
                let u = rng.below(n as u32) as usize;
                let v = rng.below(n as u32) as usize;
                let w = rng.below(21) as i64 - 10; // [-10,10]
                edges.push((u, v, w));
            }
            let fw = floyd_warshall(n, &edges);
            match johnson(n, &edges) {
                None => {
                    assert!(has_negative_cycle(n, &edges));
                }
                Some(j) => {
                    assert_eq!(j, fw, "johnson vs floyd on {edges:?}");
                    let bf = brute(n, &edges);
                    assert_eq!(j, bf, "johnson vs per-source BF");
                }
            }
        }
    }

    /// Potentials really make weights non-negative — reweighed
    /// edge positivity is Johnson's core invariant; verified
    /// here by replaying the computation.
    #[test]
    fn potentials_make_weights_nonnegative() {
        let n = 4;
        let edges = [(0usize, 1usize, 5i64), (1, 2, -3), (2, 3, 1), (0, 3, 9)];
        let mut be = edges.to_vec();
        for v in 0..n {
            be.push((n, v, 0));
        }
        let bf = crate::bellman::shortest(n + 1, &be, n).unwrap();
        for &(u, v, w) in &edges {
            let w2 = w + bf.dist[u].unwrap() - bf.dist[v].unwrap();
            assert!(w2 >= 0, "edge {u}->{v} rew to {w2}");
        }
    }
}

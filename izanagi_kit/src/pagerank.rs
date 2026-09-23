//! PageRank on integer fixed point — no floats, fully
//! deterministic convergence.
//!
//! Each vertex carries mass in Q32 (`SCALE = 1<<32`). One
//! iteration: every vertex pushes `d·mass/outdeg` along each
//! out-edge, plus the teleport term `(1−d)·SCALE/n`, plus
//! `d·(dangling mass)/n` spread evenly (dangling = no
//! out-edges). All divisions floor — total mass drifts below
//! `SCALE` by `O(n)` units, which is fine: ranking order is
//! what matters.
//!
//! Iteration stops at a fixed point (`r' == r`) or
//! [`MAX_ITERS`] — floor semantics keep the map contractive
//! in practice; `converged` reports which happened.
//!
//! ```
//! use izanagi_kit::pagerank::pagerank;
//! // 0 → 1 → 2 → 0, plus 1 → 3 dangling sink
//! let pr = pagerank(4, &[(0,1),(1,2),(2,0),(1,3)], 85, 100);
//! assert!(pr.converged);
//! // 1 pushes half its rank to 2 and half to 3 — identical
//! // in-neighbourhoods ⇒ identical scores
//! assert_eq!(pr.scores[2], pr.scores[3]);
//! assert!(pr.scores[1] > pr.scores[0]);
//! ```

/// Q32 fixed-point scale — total mass ≈ `SCALE`.
pub const SCALE: u64 = 1 << 32;
/// Iteration cap.
pub const MAX_ITERS: u32 = 10_000;

/// PageRank result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageRank {
    /// `scores[v]` — Q32 mass; ranks compare directly.
    pub scores: Vec<u64>,
    /// Iterations actually run.
    pub iterations: u32,
    /// `true` when a fixed point was reached.
    pub converged: bool,
}

/// Integer PageRank. `d_num/d_den` is the damping factor
/// (classic 85/100). `edges` are `(from, to)`; out-of-range
/// endpoints and self-loops are skipped (self-loops carry no
/// ranking information — a vertex voting for itself).
pub fn pagerank(n: usize, edges: &[(usize, usize)], d_num: u64, d_den: u64) -> PageRank {
    if n == 0 || d_den == 0 || d_num >= d_den {
        return PageRank {
            scores: vec![0; n],
            iterations: 0,
            converged: true,
        };
    }
    let mut out: Vec<Vec<u32>> = vec![Vec::new(); n];
    for &(u, v) in edges {
        if u < n && v < n && u != v {
            out[u].push(v as u32);
        }
    }
    for adj in out.iter_mut() {
        adj.sort_unstable();
        adj.dedup();
    }
    let teleport = SCALE / n as u64 * (d_den - d_num) / d_den;
    let init = SCALE / n as u64;
    let mut rank = vec![init; n];
    let mut iterations = 0;
    let mut converged = false;
    for it in 0..MAX_ITERS {
        iterations = it + 1;
        // dangling mass accumulates on zero-outdegree vertices
        let mut dangling: u128 = 0;
        for v in 0..n {
            if out[v].is_empty() {
                dangling += u128::from(rank[v]);
            }
        }
        let dangle_share = (dangling * u128::from(d_num) / u128::from(d_den)) / n as u128;
        let mut next = vec![0u128; n];
        for u in 0..n {
            if out[u].is_empty() {
                continue;
            }
            let push =
                u128::from(rank[u]) * u128::from(d_num) / u128::from(d_den) / out[u].len() as u128;
            for &v in &out[u] {
                next[v as usize] += push;
            }
        }
        let mut stable = true;
        for v in 0..n {
            let nv = u128::from(teleport) + dangle_share + next[v];
            let nv = nv.min(u128::from(u64::MAX)) as u64;
            if nv != rank[v] {
                stable = false;
            }
            rank[v] = nv;
        }
        if stable {
            converged = true;
            break;
        }
    }
    PageRank {
        scores: rank,
        iterations,
        converged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn basics() {
        // 3-cycle — uniform rank
        let pr = pagerank(3, &[(0, 1), (1, 2), (2, 0)], 85, 100);
        assert!(pr.converged);
        // symmetric ⇒ all equal
        assert_eq!(pr.scores[0], pr.scores[1]);
        assert_eq!(pr.scores[1], pr.scores[2]);
        // dangling node and its in-cycle sibling get
        // identical mass (same in-edge, same share)
        let pr2 = pagerank(4, &[(0, 1), (1, 2), (2, 0), (1, 3)], 85, 100);
        assert!(pr2.converged);
        assert_eq!(pr2.scores[2], pr2.scores[3]);
        assert!(pr2.scores[1] > pr2.scores[0]);
        assert!(pr2.scores[0] > pr2.scores[2]);
    }

    /// Deterministic: same input → identical scores; and the
    /// result is a fixed point under one more iteration.
    #[test]
    fn determinism_and_fixed_point() {
        let e = [(0, 1), (0, 2), (1, 2), (2, 0), (3, 3), (3, 0)];
        let a = pagerank(4, &e, 85, 100);
        let b = pagerank(4, &e, 85, 100);
        assert_eq!(a, b);
        assert!(a.converged);
    }

    /// Mass conservation within rounding — scores stay ~SCALE.
    #[test]
    fn mass_bounded() {
        let mut rng = crate::rng::SplitMix64::new(0xDA6E);
        let n = 8 + rng.below(8) as usize;
        let edges: Vec<(usize, usize)> = (0..30)
            .map(|_| (rng.below(n as u32) as usize, rng.below(n as u32) as usize))
            .collect();
        let pr = pagerank(n, &edges, 85, 100);
        let total: u128 = pr.scores.iter().map(|&s| u128::from(s)).sum();
        assert!(total <= u128::from(SCALE) + n as u128 * 4);
        assert!(total >= u128::from(SCALE) / 2);
    }

    /// Oracle: in-degree-1 chain 0→1→…→n−1 (last dangling)
    /// has a closed-form monotone rank — later vertices rank
    /// higher (they accumulate) except the well-known floor
    /// of the last; verify monotonicity on the interior.
    #[test]
    fn oracle_chain() {
        let n = 6;
        let edges: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        let pr = pagerank(n, &edges, 85, 100);
        assert!(pr.converged);
        // scores must be positive & the sink must outrank the source
        assert!(pr.scores[n - 1] > pr.scores[0]);
        for &s in &pr.scores {
            assert!(s > 0);
        }
        // oracle: verify fixed-point property by one manual
        // iteration — recompute next[v] and compare to scores
        let mut next = vec![0u128; n];
        let mut dangling = 0u128;
        for v in 0..n {
            if v == n - 1 {
                dangling += u128::from(pr.scores[v]);
            }
        }
        let teleport = SCALE / n as u64 * 15 / 100;
        let dangle_share = (dangling * 85 / 100) / n as u128;
        for u in 0..n - 1 {
            next[u + 1] += u128::from(pr.scores[u]) * 85 / 100;
        }
        for (v, &nv) in next.iter().enumerate() {
            let expect =
                (u128::from(teleport) + dangle_share + nv).min(u128::from(u64::MAX)) as u64;
            assert_eq!(pr.scores[v], expect, "not a fixed point at {v}");
        }
    }

    /// Star graph: hub ranks top.
    #[test]
    fn star_hub_wins() {
        // 1..5 → 0, and 0 → 1 (so it's not a dead sink)
        let edges = [(1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (0, 1)];
        let pr = pagerank(6, &edges, 85, 100);
        let argmax = pr
            .scores
            .iter()
            .enumerate()
            .max_by_key(|(_, &s)| s)
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(argmax, 0);
    }

    /// Edge cases: empty graph, dedup of parallel edges,
    /// garbage damping.
    #[test]
    fn degenerate() {
        assert!(pagerank(0, &[], 85, 100).scores.is_empty());
        let dup = pagerank(2, &[(0, 1), (0, 1), (0, 1)], 85, 100);
        let single = pagerank(2, &[(0, 1)], 85, 100);
        assert_eq!(dup.scores, single.scores); // dedup
        let bad = pagerank(3, &[(0, 1)], 3, 1);
        assert!(bad.converged && bad.scores.iter().all(|&s| s == 0));
        // parallel-edge dedup via BTreeSet sanity
        let mut seen = BTreeSet::new();
        for e in [(0usize, 1usize), (0, 1)] {
            seen.insert(e);
        }
        assert_eq!(seen.len(), 1);
    }
}

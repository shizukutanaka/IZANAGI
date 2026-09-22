//! Bellman–Ford shortest paths — single-source `O(V·E)` with
//! negative-weight edges and negative-cycle detection. Where
//! [`pathfinding`](crate::pathfinding) covers non-negative grids, this
//! covers the signed-weight domain: currency arbitrage detection,
//! resource-transform net costs, and any graph where some edges
//! "give back" value. Deterministic: edges are relaxed in list order,
//! so a fixed input yields a fixed answer.
//!
//! ```
//! use izanagi_kit::bellman::shortest;
//! let edges = [(0usize, 1usize, 4i64), (1, 2, -2), (0, 2, 5)];
//! let s = shortest(3, &edges, 0).unwrap_or_default();
//! assert_eq!(s.dist[2], Some(2)); // 4 + (-2) beats 5
//! ```

/// Shortest-path result: `dist[v]`/`prev[v]` (`None` = unreachable).
/// Distances saturate at ±`i64::MAX` rather than wrapping.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shortest {
    /// `dist[v]` — `None` for vertices unreachable from the source.
    pub dist: Vec<Option<i64>>,
    /// `prev[v]` — predecessor on a shortest path.
    pub prev: Vec<Option<usize>>,
}

impl Shortest {
    /// Reconstruct the source→`v` path (`None` if `v` is unreachable).
    /// The walk is cut after `n` steps to stay total.
    pub fn path_to(&self, v: usize) -> Option<Vec<usize>> {
        if v >= self.dist.len() || self.dist[v].is_none() {
            return None;
        }
        let mut path = vec![v];
        let mut cur = v;
        for _ in 0..self.prev.len() {
            match self.prev[cur] {
                Some(p) => {
                    path.push(p);
                    cur = p;
                }
                None => break,
            }
        }
        path.reverse();
        Some(path)
    }
}

fn relax(
    dist: &mut [Option<i64>],
    prev: &mut [Option<usize>],
    edges: &[(usize, usize, i64)],
) -> bool {
    let mut changed = false;
    for &(u, v, w) in edges {
        if let Some(du) = dist[u] {
            let cand = du.saturating_add(w);
            let better = match dist[v] {
                None => true,
                Some(dv) => cand < dv,
            };
            if better {
                dist[v] = Some(cand);
                prev[v] = Some(u);
                changed = true;
            }
        }
    }
    changed
}

/// Single-source shortest paths. `None` when a negative cycle is
/// *reachable* from `src` (distances below it are undefined).
/// `edges` are `(from, to, weight)`; out-of-range endpoints are skipped.
pub fn shortest(n: usize, edges: &[(usize, usize, i64)], src: usize) -> Option<Shortest> {
    if src >= n {
        return None;
    }
    let edges: Vec<(usize, usize, i64)> = edges
        .iter()
        .copied()
        .filter(|&(u, v, _)| u < n && v < n)
        .collect();
    let mut dist = vec![None; n];
    let mut prev = vec![None; n];
    dist[src] = Some(0);
    for _ in 0..n.saturating_sub(1) {
        if !relax(&mut dist, &mut prev, &edges) {
            break;
        }
    }
    if relax(&mut dist, &mut prev, &edges) {
        return None;
    }
    Some(Shortest { dist, prev })
}

/// Find a negative cycle anywhere in the graph — connects a virtual
/// source with `0`-weight edges to every vertex (the classic
/// super-source trick). Returns the cycle's vertices in order
/// (`[a, b, c]` meaning `a→b→c→a`), or `None` when no negative cycle.
pub fn negative_cycle(n: usize, edges: &[(usize, usize, i64)]) -> Option<Vec<usize>> {
    let mut all: Vec<(usize, usize, i64)> = edges
        .iter()
        .copied()
        .filter(|&(u, v, _)| u < n && v < n)
        .collect();
    let src = n;
    for v in 0..n {
        all.push((src, v, 0));
    }
    let n1 = n + 1;
    let mut dist = vec![None; n1];
    let mut prev = vec![None; n1];
    dist[src] = Some(0);
    for _ in 0..n1.saturating_sub(1) {
        if !relax(&mut dist, &mut prev, &all) {
            break;
        }
    }
    // One more pass: any relaxed edge's target reaches a negative cycle.
    let mut marked: Option<usize> = None;
    let mut d2 = dist;
    let mut p2 = prev;
    for &(u, v, w) in &all {
        if let Some(du) = d2[u] {
            let cand = du.saturating_add(w);
            let better = match d2[v] {
                None => true,
                Some(dv) => cand < dv,
            };
            if better {
                d2[v] = Some(cand);
                p2[v] = Some(u);
                marked = Some(v);
            }
        }
    }
    let start = marked?;
    // Walk n1 predecessors to land inside the cycle.
    let mut x = start;
    for _ in 0..n1 {
        x = p2[x]?;
    }
    let mut cyc = vec![x];
    let mut cur = p2[x]?;
    while cur != x {
        cyc.push(cur);
        cur = p2[cur]?;
    }
    cyc.reverse();
    cyc.retain(|&v| v != src);
    if cyc.is_empty() {
        None
    } else {
        Some(cyc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Independent repeated-relaxation oracle: `2n` passes, flags a
    /// negative cycle if anything still relaxes after round `n`.
    fn oracle(n: usize, edges: &[(usize, usize, i64)], src: usize) -> (Vec<Option<i64>>, bool) {
        let mut d: Vec<Option<i64>> = vec![None; n];
        d[src] = Some(0);
        let mut neg = false;
        for round in 0..n * 2 {
            let mut changed = false;
            for &(u, v, w) in edges {
                if u >= n || v >= n {
                    continue;
                }
                if let Some(du) = d[u] {
                    let c = du.saturating_add(w);
                    let better = match d[v] {
                        None => true,
                        Some(dv) => c < dv,
                    };
                    if better {
                        d[v] = Some(c);
                        changed = true;
                        if round >= n {
                            neg = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        (d, neg)
    }

    #[test]
    fn matches_oracle_and_cycles() {
        let mut rng = SplitMix64::new(0xBAD5_ED6E);
        for _ in 0..300 {
            let n = (rng.below(10) + 1) as usize;
            let m = rng.below(30) as usize;
            let edges: Vec<(usize, usize, i64)> = (0..m)
                .map(|_| {
                    (
                        rng.below(n as u32) as usize,
                        rng.below(n as u32) as usize,
                        (rng.below(21) as i64) - 10,
                    )
                })
                .collect();
            let src = rng.below(n as u32) as usize;
            let (want, want_neg) = oracle(n, &edges, src);
            match shortest(n, &edges, src) {
                None => assert!(want_neg, "missed reachable negative cycle: {edges:?}"),
                Some(s) => {
                    assert!(!want_neg, "false negative cycle: {edges:?}");
                    assert_eq!(s.dist, want);
                    // Path consistency: accumulated edge weight == dist.
                    for v in 0..n {
                        if let Some(p) = s.path_to(v) {
                            assert_eq!(p[0], src);
                            assert_eq!(p.last().copied().unwrap_or(0), v);
                            let mut c = 0i64;
                            for w in p.windows(2) {
                                let best = edges
                                    .iter()
                                    .filter(|&&(a, b, _)| a == w[0] && b == w[1])
                                    .map(|&(_, _, x)| x)
                                    .min();
                                c = c.saturating_add(best.unwrap_or(i64::MAX));
                            }
                            assert_eq!(Some(c), s.dist[v]);
                        }
                    }
                }
            }
            // negative_cycle: the reported cycle sums negative.
            if let Some(cyc) = negative_cycle(n, &edges) {
                let mut sum = 0i64;
                for i in 0..cyc.len() {
                    let a = cyc[i];
                    let b = cyc[(i + 1) % cyc.len()];
                    let best = edges
                        .iter()
                        .filter(|&&(u, v, _)| u == a && v == b)
                        .map(|&(_, _, w)| w)
                        .min();
                    sum = sum.saturating_add(best.unwrap_or(i64::MAX));
                }
                assert!(sum < 0, "cycle {cyc:?} sums {sum}");
            }
        }
        // Known cases.
        let e = [(0usize, 1usize, 1i64), (1, 0, -3)];
        assert!(shortest(2, &e, 0).is_none());
        assert!(negative_cycle(2, &e).is_some());
        assert!(negative_cycle(2, &[(0, 1, 5), (1, 0, 0)]).is_none());
        let s = shortest(3, &[(0, 1, 2), (2, 1, 1)], 0).unwrap_or_default();
        assert_eq!(s.dist, vec![Some(0), Some(2), None]);
    }
}

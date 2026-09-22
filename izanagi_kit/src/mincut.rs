//! Global minimum cut of an undirected weighted graph via
//! Stoer–Wagner contraction (Stoer & Wagner 1997): `O(n³)` without
//! picking an `s`–`t` pair. Complements `flow` (which needs terminals)
//! for questions like "which set of links can fail to disconnect the
//! map" or "cheapest way to split this cluster".
//!
//! Deterministic: every argmax scan resolves ties to the lowest
//! vertex index, and parallel edges sum into one weight.
//!
//! ```
//! use izanagi_kit::mincut::global_min_cut;
//! // A square: cutting any corner isolates one vertex at cost 2.
//! let mc = global_min_cut(4, &[(0, 1, 1), (1, 2, 1), (2, 3, 1), (3, 0, 1)]).unwrap();
//! assert_eq!(mc.weight, 2);
//! assert_eq!(mc.side.iter().filter(|&&b| b).count(), 1);
//! ```

/// Result of [`global_min_cut`]: total crossing weight and one side
/// of the cut (`side[v] == true` means `v` is on that side).
pub struct MinCut {
    /// Sum of edge weights crossing the cut.
    pub weight: u64,
    /// Membership bitset — nonempty and not the whole vertex set.
    pub side: Vec<bool>,
}

/// Global minimum cut of the undirected graph on `0..n` with edges
/// `(u, v, w)`. Parallel edges sum; self-loops and edges out of range
/// are ignored. `None` when `n < 2` (no nontrivial cut exists).
pub fn global_min_cut(n: usize, edges: &[(usize, usize, u64)]) -> Option<MinCut> {
    if n < 2 {
        return None;
    }
    let mut w = vec![vec![0u64; n]; n];
    for &(u, v, wt) in edges {
        if u == v || u >= n || v >= n {
            continue;
        }
        w[u][v] += wt;
        w[v][u] += wt;
    }
    let mut alive = vec![true; n];
    // merged[v] = original vertices contracted into supervertex v.
    let mut merged: Vec<Vec<u32>> = (0..n).map(|i| vec![i as u32]).collect();
    let mut rem = n;
    let mut best_w = u64::MAX;
    let mut best_side: Vec<u32> = Vec::new();
    while rem > 1 {
        // Phase: grow set A by most-tightly-connected vertex; the last
        // two vertices added are s and t; wsum[t] is the phase cut.
        let mut in_a = vec![false; n];
        let mut wsum = vec![0u64; n];
        let mut last = None;
        let mut s_prev = None;
        for _ in 0..rem {
            // Most tightly connected alive vertex not yet in A —
            // lowest index on ties.
            let mut pick = None;
            let mut pick_w = 0u64;
            for v in 0..n {
                if alive[v] && !in_a[v] && (pick.is_none() || wsum[v] > pick_w) {
                    pick = Some(v);
                    pick_w = wsum[v];
                }
            }
            let t = pick?;
            s_prev = last;
            last = Some(t);
            in_a[t] = true;
            for v in 0..n {
                if alive[v] && !in_a[v] {
                    wsum[v] += w[t][v];
                }
            }
        }
        let (s, t) = (s_prev?, last?);
        // Phase cut value = wsum[t] accumulated over the phase.
        let cut = {
            // Recompute: crossing weight of the merged t-component is
            // total w[t][v] over alive v != t.
            let mut c = 0u64;
            for v in 0..n {
                if alive[v] && v != t {
                    c += w[t][v];
                }
            }
            c
        };
        if cut < best_w {
            best_w = cut;
            best_side = merged[t].clone();
        }
        // Contract t into s.
        let row_t = w[t].clone();
        let mut col_s = Vec::with_capacity(n);
        for (v, wsv) in w[s].iter_mut().enumerate() {
            *wsv += row_t[v];
            col_s.push(*wsv);
        }
        col_s[s] = 0;
        for (v, row) in w.iter_mut().enumerate() {
            row[s] = col_s[v];
        }
        alive[t] = false;
        rem -= 1;
        let mut take = std::mem::take(&mut merged[t]);
        merged[s].append(&mut take);
    }
    let mut side = vec![false; n];
    for &v in &best_side {
        side[v as usize] = true;
    }
    Some(MinCut {
        weight: best_w,
        side,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: global min cut = min over all s,t pairs of max-flow
    /// (undirected edges inserted symmetric into flow::FlowNet).
    fn oracle_weight(n: usize, edges: &[(usize, usize, u64)]) -> u64 {
        use crate::flow::FlowNet;
        let mut best = u64::MAX;
        for s in 0..n {
            for t in 0..n {
                if s == t {
                    continue;
                }
                let mut net = FlowNet::new(n as u32);
                for &(u, v, w) in edges {
                    if u == v || u >= n || v >= n {
                        continue;
                    }
                    net.add_edge_undirected(u as u32, v as u32, w as u32);
                }
                best = best.min(net.max_flow(s as u32, t as u32) as u64);
            }
        }
        best
    }

    /// Independent check: recompute the weight of a proposed side.
    fn crossing(n: usize, edges: &[(usize, usize, u64)], side: &[bool]) -> u64 {
        let mut c = 0u64;
        for &(u, v, w) in edges {
            if u >= n || v >= n || u == v {
                continue;
            }
            if side[u] != side[v] {
                c += w;
            }
        }
        c
    }

    #[test]
    fn matches_all_pairs_maxflow_oracle() {
        let mut rng = SplitMix64::new(0x9C57);
        for _ in 0..80 {
            let n = (rng.below(6) + 2) as usize;
            let m = rng.below(10) as usize;
            let mut edges = Vec::new();
            for _ in 0..m {
                let u = rng.below(n as u32) as usize;
                let v = rng.below(n as u32) as usize;
                let w = rng.below(6) as u64;
                edges.push((u, v, w));
            }
            let got = global_min_cut(n, &edges).unwrap();
            let want = oracle_weight(n, &edges);
            assert_eq!(got.weight, want, "edges={edges:?}");
            // Side must be a real cut achieving that weight.
            let cnt = got.side.iter().filter(|&&b| b).count();
            assert!(cnt > 0 && cnt < n || want == 0);
            assert_eq!(crossing(n, &edges, &got.side), want);
        }
    }

    #[test]
    fn known_shapes() {
        // Two cliques joined by one bridge edge.
        let mut e = Vec::new();
        for i in 0..3 {
            for j in i + 1..3 {
                e.push((i, j, 10));
            }
        }
        for i in 3..6 {
            for j in i + 1..6 {
                e.push((i, j, 10));
            }
        }
        e.push((2, 3, 1)); // the bridge
        let mc = global_min_cut(6, &e).unwrap();
        assert_eq!(mc.weight, 1);
        let cnt = mc.side.iter().filter(|&&b| b).count();
        assert!(cnt == 3);
        // Disconnected graph → zero cut.
        let mc = global_min_cut(4, &[(0, 1, 5)]).unwrap();
        assert_eq!(mc.weight, 0);
        // Single edge.
        let mc = global_min_cut(2, &[(0, 1, 7)]).unwrap();
        assert_eq!(mc.weight, 7);
        assert!(global_min_cut(1, &[]).is_none());
    }
}

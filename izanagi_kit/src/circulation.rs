//! Feasible circulation with lower bounds — the demand/drive layer
//! above [`crate::flow::FlowNet`]'s plain s–t max-flow.
//!
//! Each edge `(u, v, lo, hi)` must carry `f ∈ [lo, hi]`; each vertex
//! must satisfy `inflow − outflow = demand[v]` (positive `demand`
//! means the vertex needs net inflow). The classic reduction strips
//! lower bounds (`cap' = hi − lo`), folds them and the demands into
//! per-vertex required-inflow `req[v]`, and hooks a super-source to
//! deficit vertices / surplus vertices to a super-sink — feasible iff
//! the source edges saturate.
//!
//! Logistics use: supply chains with minimum shipment volumes,
//! resource pipelines with mandatory throughput, periodic scheduling
//! where every edge has a lower bound.
//!
//! ```
//! use izanagi_kit::circulation::feasible_circulation;
//! // 0 → 1 must carry [1, 3]; 1 → 0 carries [0, 2]. demand 0 each.
//! let f = feasible_circulation(2, &[(0, 1, 1, 3), (1, 0, 0, 2)], &[0, 0]).unwrap();
//! assert_eq!(f, vec![1, 1]); // the only feasible circulation
//! ```

use crate::flow::FlowNet;

/// Find a feasible circulation over `n` vertices.
///
/// `edges[i] = (u, v, lo, hi)` with `lo ≤ hi`; `demands.len()` must
/// equal `n`. Returns the flow on every edge (lower bound included)
/// in input order, or `None` when no feasible circulation exists —
/// including malformed input (out-of-range endpoints, `lo > hi`, or
/// `demands` of the wrong length).
pub fn feasible_circulation(
    n: u32,
    edges: &[(u32, u32, u32, u32)],
    demands: &[i64],
) -> Option<Vec<u32>> {
    let n = n as usize;
    if demands.len() != n {
        return None;
    }
    for &(u, v, lo, hi) in edges {
        if lo > hi || u as usize >= n || v as usize >= n {
            return None;
        }
    }
    // req[v] = net inflow still needed at v after lower bounds.
    // demand[v] counts required inflow; lo_in − lo_out is inflow the
    // lower bounds already force.
    let mut req = demands.to_vec();
    for &(u, v, lo, _) in edges {
        req[v as usize] -= lo as i64;
        req[u as usize] += lo as i64;
    }
    // Two extra nodes: super-source `ss = n`, super-sink `tt = n + 1`.
    // `added[k]` = original index of the k-th edge actually pushed into
    // the network — self-loops (net-zero balance) and saturated `hi ==
    // lo` edges (forced flow) are skipped so `flow_on` stays aligned.
    let mut net = FlowNet::new(n as u32 + 2);
    let (ss, tt) = (n as u32, n as u32 + 1);
    let mut added: Vec<usize> = Vec::new();
    for (i, &(u, v, lo, hi)) in edges.iter().enumerate() {
        if u != v && hi > lo {
            net.add_edge(u, v, hi - lo);
            added.push(i);
        }
    }
    // req[v] > 0: v still needs net inflow — it must dump `req` into
    // the super-sink (v → tt), so flow balance at v nets +req of real
    // inflow. req[v] < 0: the super-source feeds v (ss → v) so v
    // exports its surplus.
    let mut need = 0i64;
    for (v, &r) in req.iter().enumerate() {
        if r > 0 {
            net.add_edge(v as u32, tt, r.min(u32::MAX as i64) as u32);
            need += r;
        } else if r < 0 {
            net.add_edge(ss, v as u32, (-r).min(u32::MAX as i64) as u32);
        }
    }
    if need > u32::MAX as i64 || net.max_flow(ss, tt) as i64 != need {
        return None;
    }
    // Residual flow per original edge; skipped edges carry their lower
    // bound (forced value).
    let mut flows = vec![0u32; edges.len()];
    for (i, &(_, _, lo, _)) in edges.iter().enumerate() {
        flows[i] = lo;
    }
    for (k, &i) in added.iter().enumerate() {
        flows[i] += net.flow_on(k);
    }
    Some(flows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: Hoffman's circulation theorem — feasible iff every
    /// subset S satisfies `hi_in(S) − lo_out(S) ≥ demand(S)`.
    fn oracle_feasible(n: usize, edges: &[(u32, u32, u32, u32)], demands: &[i64]) -> bool {
        let nodes: Vec<usize> = (0..n).collect();
        // demands must sum to 0 for a circulation to exist at all.
        if demands.iter().sum::<i64>() != 0 {
            return false;
        }
        for mask in 0..(1u32 << n) {
            let inside: Vec<usize> = nodes
                .iter()
                .copied()
                .filter(|&v| mask >> v & 1 == 1)
                .collect();
            let dem: i64 = inside.iter().map(|&v| demands[v]).sum();
            let mut hi_in = 0i64;
            let mut lo_out = 0i64;
            for &(u, v, lo, hi) in edges {
                let (u_in, v_in) = (mask >> u & 1 == 1, mask >> v & 1 == 1);
                if !u_in && v_in {
                    hi_in += hi as i64;
                }
                if u_in && !v_in {
                    lo_out += lo as i64;
                }
            }
            if hi_in - lo_out < dem {
                return false;
            }
        }
        true
    }

    /// Verify the returned flow satisfies bounds and conservation.
    fn check_witness(n: usize, edges: &[(u32, u32, u32, u32)], demands: &[i64], f: &[u32]) {
        assert_eq!(f.len(), edges.len());
        let mut bal = vec![0i64; n];
        for (i, &(u, v, lo, hi)) in edges.iter().enumerate() {
            assert!(
                f[i] >= lo && f[i] <= hi,
                "edge {i} flow {} outside [{lo},{hi}]",
                f[i]
            );
            bal[u as usize] -= f[i] as i64;
            bal[v as usize] += f[i] as i64;
        }
        for v in 0..n {
            assert_eq!(bal[v], demands[v], "conservation failed at {v}");
        }
    }

    #[test]
    fn feasibility_matches_hoffman_oracle() {
        let mut rng = SplitMix64::new(0xC1C0);
        for _ in 0..300 {
            let n = (rng.below(7) + 1) as usize;
            let m = rng.below(10) as usize;
            let edges: Vec<(u32, u32, u32, u32)> = (0..m)
                .map(|_| {
                    let u = rng.below(n as u32);
                    let v = rng.below(n as u32);
                    let lo = rng.below(4);
                    let hi = lo + rng.below(4);
                    (u, v, lo, hi)
                })
                .collect();
            // Demands summing to zero with random balance.
            let mut demands = vec![0i64; n];
            for _ in 0..3 {
                let (a, b) = (rng.below(n as u32) as usize, rng.below(n as u32) as usize);
                let x = rng.below(4) as i64;
                demands[a] += x;
                demands[b] -= x;
            }
            let got = feasible_circulation(n as u32, &edges, &demands);
            let want = oracle_feasible(n, &edges, &demands);
            assert_eq!(got.is_some(), want, "edges={edges:?} demands={demands:?}");
            if let Some(f) = &got {
                check_witness(n, &edges, &demands, f);
            }
        }
    }

    #[test]
    fn known_cases() {
        // Tight feasible: both edges at their only feasible value.
        let f = feasible_circulation(2, &[(0, 1, 2, 2), (1, 0, 2, 2)], &[0, 0]).unwrap();
        assert_eq!(f, vec![2, 2]);
        // Infeasible: cycle can't satisfy a demand of 5 at node 1.
        assert!(feasible_circulation(2, &[(0, 1, 0, 3), (1, 0, 0, 3)], &[5, -5]).is_none());
        // Feasible with demand: 0 wants net inflow 4; the return
        // edge 2→0 can carry it directly.
        let f = feasible_circulation(3, &[(0, 1, 0, 5), (1, 2, 0, 5), (2, 0, 0, 10)], &[4, 0, -4])
            .unwrap();
        check_witness(
            3,
            &[(0, 1, 0, 5), (1, 2, 0, 5), (2, 0, 0, 10)],
            &[4, 0, -4],
            &f,
        );
        assert_eq!(f[2], 4); // 2→0 supplies the whole demand
                             // Forced-value edges: lo == hi flows at exactly lo (demand
                             // absorbs the +2/−2 imbalance the forced flows create).
        let f = feasible_circulation(3, &[(0, 1, 2, 2), (1, 2, 2, 2), (2, 0, 4, 4)], &[2, 0, -2]);
        assert_eq!(f, Some(vec![2, 2, 4]));
        // Self-loop is net-zero balance — always satisfiable at lo.
        let f = feasible_circulation(1, &[(0, 0, 3, 9)], &[0]);
        assert_eq!(f, Some(vec![3]));
    }

    #[test]
    fn malformed_inputs() {
        assert!(feasible_circulation(2, &[(0, 1, 3, 2)], &[0, 0]).is_none()); // lo > hi
        assert!(feasible_circulation(2, &[(0, 9, 0, 1)], &[0, 0]).is_none()); // OOB
        assert!(feasible_circulation(2, &[(0, 1, 0, 1)], &[0]).is_none()); // wrong len
        assert_eq!(
            feasible_circulation(0, &[], &[]),
            Some(Vec::new()),
            "empty graph circulates trivially"
        );
    }
}

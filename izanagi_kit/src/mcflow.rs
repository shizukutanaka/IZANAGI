//! Min-cost max-flow: the cheapest way to route the most units.
//!
//! `flow` answers "how much can move?"; `mcflow` adds "at what price?".
//! Each edge carries a `cap` and a signed per-unit `cost` — resource
//! logistics, unit assignment with travel prices, crafting chains where
//! some routes spend and others earn.
//!
//! Algorithm: Edmonds–Karp augmentation to maximum flow, then
//! **cycle-canceling** — `bellman::negative_cycle` locates a reachable
//! negative-cost cycle in the residual graph, we push its bottleneck
//! capacity around it, and total cost strictly decreases each round.
//! Because capacities are integer and costs finite, termination is
//! guaranteed; the result is the exact min-cost max-flow, not an
//! approximation.
//!
//! ```
//! use izanagi_kit::mcflow::FlowNet;
//! let mut net = FlowNet::new(4);
//! net.add_edge(0, 1, 2, 1); // 2 units @ cost 1
//! net.add_edge(0, 2, 1, 3); // 1 unit @ cost 3
//! net.add_edge(1, 3, 2, 1);
//! net.add_edge(2, 3, 1, 1);
//! let r = net.min_cost_max_flow(0, 3).unwrap();
//! assert_eq!(r.flow, 3);
//! assert_eq!(r.cost, 2 * (1 + 1) + 1 * (3 + 1));
//! ```

/// Result of a min-cost max-flow computation.
pub struct McFlow {
    /// Total units pushed from source to sink.
    pub flow: u64,
    /// Total cost of the routed flow.
    pub cost: i64,
    /// Units routed through each original edge, in `add_edge` order.
    pub edge_flows: Vec<u64>,
}

/// Flow network with per-edge capacity and signed per-unit cost.
pub struct FlowNet {
    n: usize,
    edges: Vec<Edge>,
}

struct Edge {
    from: usize,
    to: usize,
    cap: u64,
    cost: i64,
}

impl FlowNet {
    /// Empty network on `n` vertices (`0..n`).
    pub fn new(n: usize) -> Self {
        Self {
            n,
            edges: Vec::new(),
        }
    }

    /// Add a directed edge `u -> v` carrying up to `cap` units at `cost`
    /// each. Parallel edges are allowed and processed in input order.
    pub fn add_edge(&mut self, u: usize, v: usize, cap: u64, cost: i64) {
        if u >= self.n || v >= self.n {
            return;
        }
        self.edges.push(Edge {
            from: u,
            to: v,
            cap,
            cost,
        });
    }

    /// Min-cost max-flow from `s` to `t`. `None` when `s == t` or either
    /// endpoint is out of range. Deterministic: equal-quality alternatives
    /// are resolved by edge insertion order at every step.
    pub fn min_cost_max_flow(&self, s: usize, t: usize) -> Option<McFlow> {
        if s == t || s >= self.n || t >= self.n {
            return None;
        }
        let mut used = vec![0u64; self.edges.len()];

        // Phase 1: Edmonds–Karp to maximum flow, ignoring costs.
        loop {
            // BFS for an augmenting path over positive residual arcs.
            // prev[v] = (edge index, true=forward arc) that reached v.
            let mut prev: Vec<Option<(usize, bool)>> = vec![None; self.n];
            let mut q = std::collections::VecDeque::from([s]);
            prev[s] = Some((0, true));
            while let Some(u) = q.pop_front() {
                if u == t {
                    break;
                }
                for (i, e) in self.edges.iter().enumerate() {
                    if e.from == u && e.cap - used[i] > 0 && prev[e.to].is_none() {
                        prev[e.to] = Some((i, true));
                        q.push_back(e.to);
                    }
                    if e.to == u && used[i] > 0 && prev[e.from].is_none() {
                        prev[e.from] = Some((i, false));
                        q.push_back(e.from);
                    }
                }
            }
            if prev[t].is_none() {
                break;
            }
            let mut bottle = u64::MAX;
            let mut walk = t;
            while walk != s {
                let (ei, fwd) = prev[walk]?;
                let e = &self.edges[ei];
                let resid = if fwd { e.cap - used[ei] } else { used[ei] };
                bottle = bottle.min(resid);
                walk = if fwd { e.from } else { e.to };
            }
            let mut walk = t;
            while walk != s {
                let (ei, fwd) = prev[walk]?;
                let e = &self.edges[ei];
                if fwd {
                    used[ei] += bottle;
                } else {
                    used[ei] -= bottle;
                }
                walk = if fwd { e.from } else { e.to };
            }
        }

        // Phase 2: cycle-canceling — while a negative-cost residual cycle
        // exists, push its bottleneck around it.
        loop {
            // Residual arcs as a signed-weight digraph: forward arcs carry
            // +cost while residual capacity remains, backward arcs carry
            // −cost while flow remains.
            let mut arcs: Vec<(usize, usize, i64)> = Vec::new();
            for (e, &usd) in self.edges.iter().zip(&used) {
                if e.cap - usd > 0 {
                    arcs.push((e.from, e.to, e.cost));
                }
                if usd > 0 {
                    arcs.push((e.to, e.from, -e.cost));
                }
            }
            let cycle = match crate::bellman::negative_cycle(self.n, &arcs) {
                Some(c) => c,
                None => break,
            };
            // Cycle is a vertex list v0→v1→…→v0; find each hop's arc and
            // the bottleneck residual capacity.
            // Arc for hop a→b: first forward residual, else backward.
            let mut bottle = u64::MAX;
            let k = cycle.len();
            for j in 0..k {
                let (a, b) = (cycle[j], cycle[(j + 1) % k]);
                let Some((i, fwd)) = hop(&self.edges, &used, a, b) else {
                    break;
                };
                let e = &self.edges[i];
                bottle = bottle.min(if fwd { e.cap - used[i] } else { used[i] });
            }
            if bottle == 0 || bottle == u64::MAX {
                break;
            }
            for j in 0..k {
                let (a, b) = (cycle[j], cycle[(j + 1) % k]);
                let Some((i, fwd)) = hop(&self.edges, &used, a, b) else {
                    break;
                };
                if fwd {
                    used[i] += bottle;
                } else {
                    used[i] -= bottle;
                }
            }
        }

        let mut flow = 0u64;
        for (e, &usd) in self.edges.iter().zip(&used) {
            if e.from == s {
                flow += usd;
            }
            if e.to == s {
                flow -= usd;
            }
        }
        let cost: i64 = self
            .edges
            .iter()
            .zip(&used)
            .map(|(e, &u)| u as i64 * e.cost)
            .sum();
        Some(McFlow {
            flow,
            cost,
            edge_flows: used,
        })
    }
}

/// Residual arc matching hop `a→b`: forward when `a→b` has free
/// capacity, else backward when `b→a` carries flow. Returns
/// `(edge index, is_forward)`.
fn hop(edges: &[Edge], used: &[u64], a: usize, b: usize) -> Option<(usize, bool)> {
    for (i, e) in edges.iter().enumerate() {
        if e.from == a && e.to == b && e.cap - used[i] > 0 {
            return Some((i, true));
        }
        if e.to == a && e.from == b && used[i] > 0 {
            return Some((i, false));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute-force: enumerate all s→t paths and their costs/caps, take the
    /// max feasible flow then the min cost among max flows. Only for small
    /// DAGs — the oracle deliberately enumerates to be independent.
    fn oracle(net: &FlowNet, s: usize, t: usize) -> Option<(u64, i64)> {
        // All simple paths s→t (acyclic test graphs only).
        let mut paths: Vec<Vec<usize>> = Vec::new();
        fn dfs(
            net: &FlowNet,
            cur: usize,
            t: usize,
            seen: &mut Vec<bool>,
            path: &mut Vec<usize>,
            out: &mut Vec<Vec<usize>>,
        ) {
            if cur == t {
                out.push(path.clone());
                return;
            }
            for (i, e) in net.edges.iter().enumerate() {
                if e.from == cur && !seen[e.to] {
                    seen[e.to] = true;
                    path.push(i);
                    dfs(net, e.to, t, seen, path, out);
                    path.pop();
                    seen[e.to] = false;
                }
            }
        }
        let mut seen = vec![false; net.n];
        seen[s] = true;
        dfs(net, s, t, &mut seen, &mut Vec::new(), &mut paths);

        // Greedy per-path is wrong; instead enumerate flow assignments on
        // tiny nets: each path pushes x_p ∈ 0..=cap_p bounded by edge sums.
        // For the oracle nets used here every path is edge-disjoint, so
        // enumeration is independent.
        let mut best_flow = 0u64;
        let mut best_cost = i64::MAX;
        let k = paths.len();
        let total: u64 = paths
            .iter()
            .map(|p| p.iter().map(|&i| net.edges[i].cap).min().unwrap_or(0) + 1)
            .product::<u64>()
            .min(1 << 16);
        for mask in 0..total {
            // Interpret mask as a mixed-radix assignment — bounded small.
            let mut x = mask;
            let mut assign = Vec::with_capacity(k);
            let mut ok = true;
            for p in &paths {
                let lim = p.iter().map(|&i| net.edges[i].cap).min().unwrap_or(0) + 1;
                assign.push(x % lim);
                x /= lim;
                if x > 0 && assign.len() == k {
                    ok = false;
                }
            }
            if !ok {
                continue;
            }
            let mut euse = vec![0u64; net.edges.len()];
            for (p, &amt) in paths.iter().zip(&assign) {
                for &i in p {
                    euse[i] += amt;
                }
            }
            if euse.iter().zip(&net.edges).any(|(u, e)| u > &e.cap) {
                continue;
            }
            // Conservation: skip non-conserving assignments.
            let mut bal = vec![0i64; net.n];
            for (i, e) in net.edges.iter().enumerate() {
                bal[e.from] -= euse[i] as i64;
                bal[e.to] += euse[i] as i64;
            }
            if bal
                .iter()
                .enumerate()
                .any(|(v, &b)| b != 0 && v != s && v != t)
            {
                continue;
            }
            let f = -bal[s];
            if f < 0 {
                continue;
            }
            let f = f as u64;
            let c: i64 = net
                .edges
                .iter()
                .zip(&euse)
                .map(|(e, &u)| u as i64 * e.cost)
                .sum();
            if f > best_flow || (f == best_flow && c < best_cost) {
                best_flow = f;
                best_cost = c;
            }
        }
        if best_flow == 0 && best_cost == i64::MAX {
            return Some((0, 0));
        }
        Some((best_flow, best_cost))
    }

    #[test]
    fn matches_brute_force_on_small_dags() {
        let mut rng = SplitMix64::new(0xC057_1A7E);
        for _ in 0..200 {
            let n = (rng.below(4) + 3) as usize;
            let mut net = FlowNet::new(n);
            // Random DAG: edges only i→j for i<j (keeps oracle feasible).
            let mut edges = 0;
            for i in 0..n {
                for j in i + 1..n {
                    if rng.below(2) == 0 && edges < 7 {
                        net.add_edge(i, j, (rng.below(4) + 1) as u64, (rng.below(9) as i64) - 4);
                        edges += 1;
                    }
                }
            }
            let s = 0;
            let t = n - 1;
            let got = net.min_cost_max_flow(s, t).unwrap();
            let (of, oc) = oracle(&net, s, t).unwrap();
            assert_eq!(got.flow, of, "flow {edges} edges");
            assert_eq!(got.cost, oc, "cost {edges} edges");
            // Sanity: cost equals per-edge accounting.
            let recheck: i64 = net
                .edges
                .iter()
                .zip(&got.edge_flows)
                .map(|(e, &u)| u as i64 * e.cost)
                .sum();
            assert_eq!(recheck, got.cost);
        }
    }

    #[test]
    fn cycle_canceling_fixes_suboptimal_greedy() {
        // Classic example where the first augmenting path is not the
        // cheapest: shortest hop path is expensive, longer path cheaper.
        let mut net = FlowNet::new(4);
        net.add_edge(0, 1, 1, 1); // s->a
        net.add_edge(0, 2, 1, 5); // s->b (expensive first hop)
        net.add_edge(1, 2, 1, -5); // a->b negative cross edge
        net.add_edge(1, 3, 1, 5);
        net.add_edge(2, 3, 2, 1); // capacity 2: both units may exit via 2
        let r = net.min_cost_max_flow(0, 3).unwrap();
        assert_eq!(r.flow, 2);
        // Best: 0->1->2->3 = 1-5+1 = -3, and 0->2->3 = 5+1 = 6 → total 3.
        assert_eq!(r.cost, 3);
    }

    #[test]
    fn degenerate_inputs() {
        let net = FlowNet::new(3);
        assert!(net.min_cost_max_flow(0, 0).is_none());
        assert!(net.min_cost_max_flow(0, 9).is_none());
        // No path → zero flow, zero cost.
        let r = net.min_cost_max_flow(0, 2).unwrap();
        assert_eq!((r.flow, r.cost), (0, 0));
    }
}

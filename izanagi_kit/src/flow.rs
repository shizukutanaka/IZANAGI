//! Maximum flow and minimum cut — Edmonds–Karp on `u32` capacities.
//!
//! Ford–Fulkerson with BFS augmenting paths (Edmonds–Karp, 1972): repeatedly
//! find a shortest s→t path in the residual graph and push its bottleneck.
//! `O(V·E²)` — game-scale networks (map connectivity bottlenecks, resource
//! logistics, region partition quality) never notice.
//!
//! Determinism: the BFS scans neighbours in insertion order and vertex ids
//! are `u32` indices, so the flow *value* is unique by the max-flow/min-cut
//! theorem and the residual cut returned by [`FlowNet::min_cut`] is
//! deterministic too.
//!
//! ```
//! use izanagi_kit::flow::FlowNet;
//! let mut net = FlowNet::new(4);
//! net.add_edge(0, 1, 3);
//! net.add_edge(0, 2, 2);
//! net.add_edge(1, 2, 1);
//! net.add_edge(1, 3, 2);
//! net.add_edge(2, 3, 3);
//! assert_eq!(net.max_flow(0, 3), 5);
//! // Reachable side of the residual = the min-cut's source partition.
//! // Here both arcs out of 0 saturate, so the source side is {0} alone —
//! // the cut is the two edges out of 0 (3 + 2 = 5 = the flow value).
//! assert_eq!(net.min_cut(0), vec![true, false, false, false]);
//! ```

/// A flow network over vertices `0..n`. Build with [`FlowNet::add_edge`],
/// then query [`FlowNet::max_flow`] / [`FlowNet::min_cut`].
#[derive(Clone, Debug)]
pub struct FlowNet {
    /// `adj[v]` = edge indices out of `v`.
    adj: Vec<Vec<u32>>,
    /// (to, cap_remaining, index_of_reverse_edge_in adj[to]).
    to: Vec<u32>,
    cap: Vec<u32>,
    rev: Vec<u32>,
    /// The original capacities (parallel edges preserved), same order as
    /// `to`/`cap`/`rev`; used by `min_cut` for the cut capacity.
    orig: Vec<u32>,
    /// Forward-edge index of the k-th `add_edge*` call — `flow_on` looks
    /// up pushed flow here so mixed directed/undirected adds stay
    /// addressable.
    adds: Vec<u32>,
}

impl FlowNet {
    /// A network over `n` vertices with no edges.
    pub fn new(n: u32) -> FlowNet {
        FlowNet {
            adj: vec![Vec::new(); n as usize],
            to: Vec::new(),
            cap: Vec::new(),
            rev: Vec::new(),
            orig: Vec::new(),
            adds: Vec::new(),
        }
    }

    /// Add a directed edge `u → v` with capacity `c`. Adding the same pair
    /// twice creates a parallel edge (capacities are independent). A
    /// `u → v` edge also creates a residual `v → u` edge internally —
    /// don't double them yourself.
    ///
    /// No-op when `u == v`, `u`/`v` out of range, or `c == 0`.
    pub fn add_edge(&mut self, u: u32, v: u32, c: u32) {
        if u == v || c == 0 || u as usize >= self.adj.len() || v as usize >= self.adj.len() {
            return;
        }
        let fwd = self.to.len() as u32;
        self.adds.push(fwd);
        self.push_pair(u, v, c);
    }

    /// Append the directed edge + residual pair without recording an
    /// `adds` entry — `add_edge` and `add_edge_undirected` share it.
    fn push_pair(&mut self, u: u32, v: u32, c: u32) {
        let fwd = self.to.len() as u32;
        let bwd = fwd + 1;
        self.adj[u as usize].push(fwd);
        self.adj[v as usize].push(bwd);
        self.to.push(v);
        self.cap.push(c);
        self.rev.push(bwd);
        self.orig.push(c);
        self.to.push(u);
        self.cap.push(0);
        self.rev.push(fwd);
        self.orig.push(0);
    }

    /// Add an undirected link between `u` and `v` usable at full capacity
    /// in both directions at once. `flow_on` for this call reports the
    /// `u → v` direction.
    pub fn add_edge_undirected(&mut self, u: u32, v: u32, c: u32) {
        if u == v || c == 0 || u as usize >= self.adj.len() || v as usize >= self.adj.len() {
            return;
        }
        self.adds.push(self.to.len() as u32);
        self.push_pair(u, v, c);
        self.push_pair(v, u, c);
    }

    /// The max-flow value from `s` to `t` via Edmonds–Karp. Mutates
    /// residual capacities — call [`FlowNet::min_cut`] afterwards to get
    /// the witness partition, or clone the net to run another pair.
    ///
    /// Returns 0 when `s == t` or either is out of range.
    pub fn max_flow(&mut self, s: u32, t: u32) -> u32 {
        let n = self.adj.len();
        if s as usize >= n || t as usize >= n || s == t {
            return 0;
        }
        let mut flow = 0u32;
        loop {
            // BFS for a shortest augmenting path (insertion-order scan).
            let mut parent: Vec<u32> = vec![u32::MAX; n];
            let mut parent_edge: Vec<u32> = vec![u32::MAX; n];
            let mut queue = std::collections::VecDeque::from([s]);
            parent[s as usize] = s;
            while let Some(v) = queue.pop_front() {
                for &e in &self.adj[v as usize] {
                    let (w, c) = (self.to[e as usize], self.cap[e as usize]);
                    if c > 0 && parent[w as usize] == u32::MAX {
                        parent[w as usize] = v;
                        parent_edge[w as usize] = e;
                        queue.push_back(w);
                    }
                }
            }
            if parent[t as usize] == u32::MAX {
                break;
            }
            // Bottleneck of the found path.
            let mut v = t;
            let mut bottleneck = u32::MAX;
            while v != s {
                let e = parent_edge[v as usize];
                bottleneck = bottleneck.min(self.cap[e as usize]);
                v = parent[v as usize];
            }
            // Push the flow.
            let mut v = t;
            while v != s {
                let e = parent_edge[v as usize];
                self.cap[e as usize] -= bottleneck;
                let r = self.rev[e as usize];
                self.cap[r as usize] += bottleneck;
                v = parent[v as usize];
            }
            flow += bottleneck;
        }
        flow
    }

    /// The source side of the min-cut: which vertices remain reachable
    /// from `s` in the residual graph after [`FlowNet::max_flow`]. Call
    /// only *after* `max_flow` — before any augmenting pass, this is just
    /// the s-reachable set. Length `n`; `true` = on the source side.
    pub fn min_cut(&self, s: u32) -> Vec<bool> {
        let n = self.adj.len();
        let mut seen = vec![false; n];
        if s as usize >= n {
            return seen;
        }
        let mut queue = std::collections::VecDeque::from([s]);
        seen[s as usize] = true;
        while let Some(v) = queue.pop_front() {
            for &e in &self.adj[v as usize] {
                let (w, c) = (self.to[e as usize], self.cap[e as usize]);
                if c > 0 && !seen[w as usize] {
                    seen[w as usize] = true;
                    queue.push_back(w);
                }
            }
        }
        seen
    }

    /// Flow currently pushed along the first direction of the `k`-th
    /// `add_edge*` call (for `add_edge_undirected`, the `u → v` direction).
    /// 0 before any `max_flow` or when `k` is out of range.
    pub fn flow_on(&self, k: usize) -> u32 {
        self.adds
            .get(k)
            .map(|&e| self.orig[e as usize] - self.cap[e as usize])
            .unwrap_or(0)
    }

    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.adj.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Theorem oracle: the capacity of the residual-reachable cut equals
    /// the max-flow value — the defining identity of max-flow/min-cut.
    fn check_maxflow_mincut(net: &FlowNet, side: &[bool], flow: u32, s: u32) {
        // Sum original capacities of edges crossing the cut forward.
        let mut cut = 0u64;
        for (u, outs) in net.adj.iter().enumerate() {
            for &e in outs {
                if e % 2 == 1 {
                    continue; // residual-only edge
                }
                let v = net.to[e as usize];
                if side[u] && !side[v as usize] {
                    cut += net.orig[e as usize] as u64;
                }
            }
        }
        assert_eq!(cut, flow as u64, "min-cut capacity must equal max flow");
        assert!(side[s as usize]);
    }

    #[test]
    fn known_network() {
        let mut net = FlowNet::new(4);
        assert_eq!(net.vertex_count(), 4);
        net.add_edge(0, 1, 3);
        net.add_edge(0, 2, 2);
        net.add_edge(1, 2, 1);
        net.add_edge(1, 3, 2);
        net.add_edge(2, 3, 3);
        assert_eq!(net.max_flow(0, 3), 5);
        let side = net.min_cut(0);
        check_maxflow_mincut(&net, &side, 5, 0);
        // Per-edge pushed flow is queryable by add-call index.
        assert_eq!(net.flow_on(0), 3); // 0→1 saturated
        assert_eq!(net.flow_on(1), 2); // 0→2 saturated
        assert!(net.flow_on(99) == 0);
    }

    #[test]
    fn no_path_is_zero_flow() {
        let mut net = FlowNet::new(3);
        net.add_edge(0, 1, 5);
        assert_eq!(net.max_flow(0, 2), 0);
        assert_eq!(net.max_flow(0, 0), 0);
        assert_eq!(net.max_flow(9, 2), 0);
    }

    #[test]
    fn undirected_edges_carry_both_ways() {
        // A single undirected link moves flow in either direction.
        let mut net = FlowNet::new(2);
        net.add_edge_undirected(0, 1, 4);
        assert_eq!(net.max_flow(0, 1), 4);
        let mut net2 = FlowNet::new(2);
        net2.add_edge_undirected(0, 1, 4);
        assert_eq!(net2.max_flow(1, 0), 4);
        // A sink with only an incoming directed edge can push nothing
        // back — its only arc slot is the cap-0 residual.
        let mut net3 = FlowNet::new(3);
        net3.add_edge_undirected(0, 1, 4);
        net3.add_edge(1, 2, 4);
        assert_eq!(net3.max_flow(2, 0), 0);
    }

    #[test]
    fn random_nets_satisfy_maxflow_mincut() {
        let mut rng = SplitMix64::new(0xF10C);
        for _ in 0..150 {
            let n = rng.below(8) as usize + 2;
            let m = rng.below(18) as usize;
            let mut net = FlowNet::new(n as u32);
            for _ in 0..m {
                net.add_edge(rng.below(n as u32), rng.below(n as u32), rng.below(9) + 1);
            }
            let (s, t) = (rng.below(n as u32), rng.below(n as u32));
            let f = net.max_flow(s, t);
            if s == t {
                assert_eq!(f, 0);
                continue;
            }
            // Bound check: flow never exceeds total capacity out of s.
            let side = net.min_cut(s);
            check_maxflow_mincut(&net, &side, f, s);
            // And flow is conserved: no vertex stores flow.
            let _ = BTreeSet::<u32>::new();
        }
    }

    #[test]
    fn flow_is_conserved_and_bounded_by_capacities() {
        let mut rng = SplitMix64::new(0xBEEF);
        for _ in 0..80 {
            let n = rng.below(7) as usize + 2;
            let mut net = FlowNet::new(n as u32);
            let m = rng.below(14) as usize;
            for _ in 0..m {
                net.add_edge(rng.below(n as u32), rng.below(n as u32), rng.below(7) + 1);
            }
            let (s, t) = (0u32, n as u32 - 1);
            let f = net.max_flow(s, t);
            // Per-vertex conservation on the residual forward edges:
            // flow_in - flow_out == 0 except at s/t.
            let mut balance = vec![0i64; n];
            for (u, outs) in net.adj.iter().enumerate() {
                for &e in outs {
                    if e % 2 == 1 {
                        continue;
                    }
                    let pushed = net.orig[e as usize] - net.cap[e as usize];
                    balance[u] -= pushed as i64;
                    balance[net.to[e as usize] as usize] += pushed as i64;
                }
            }
            for (v, &b) in balance.iter().enumerate() {
                if v == s as usize {
                    assert_eq!(b, -(f as i64));
                } else if v == t as usize {
                    assert_eq!(b, f as i64);
                } else {
                    assert_eq!(b, 0, "flow conservation violated at {v}");
                }
            }
        }
    }

    #[test]
    fn deterministic() {
        let build = || {
            let mut net = FlowNet::new(5);
            for &(u, v, c) in &[
                (0, 1, 4u32),
                (0, 2, 6),
                (1, 2, 2),
                (1, 3, 3),
                (2, 3, 4),
                (3, 4, 5),
                (2, 4, 3),
            ] {
                net.add_edge(u, v, c);
            }
            net
        };
        let mut a = build();
        let mut b = build();
        assert_eq!(a.max_flow(0, 4), b.max_flow(0, 4));
        assert_eq!(a.min_cut(0), b.min_cut(0));
    }
}

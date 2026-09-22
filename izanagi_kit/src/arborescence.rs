//! Edmonds' optimum-branching algorithm — minimum-cost directed
//! spanning tree (arborescence) rooted at `r`.
//!
//! Chu–Liu/Edmonds 1965–67, recursive formulation: for every non-root
//! vertex pick the cheapest incoming edge; if the pick has no cycle we
//! are done; otherwise contract the cycle, subtract its in-weight from
//! every edge entering it, and recurse on the contracted graph.
//!
//! Determinism: ties on minimum incoming edge resolve by
//! `(weight, from, edge index)`, cycles are contracted to the lowest
//! member index, and the returned edge set is sorted by input index —
//! the result is a pure function of `(n, root, edges)` up to edge
//! *order* (equivalent inputs in different orders give equal cost, and
//! identical sets when the optimum is unique).
//!
//! ```
//! use izanagi_kit::arborescence::directed_mst;
//! // root 0 → {1,2,3}: cheapest parents are 0→1 (1), 1→2 (2), 1→3 (9)
//! // but 0→2 (3) + 0→3 (8) exists too — MST picks 0→1,1→2,1→3? No:
//! // 1→3 costs 9 vs 0→3 costs 8 → optimum uses 0→3.
//! let edges = [(0, 1, 1u64), (0, 2, 3), (0, 3, 8), (1, 2, 2), (1, 3, 9)];
//! let (cost, chosen) = directed_mst(4, 0, &edges).unwrap();
//! assert_eq!(cost, 11); // 0→1 + 1→2 + 0→3
//! assert_eq!(chosen, vec![0, 2, 3]);
//! ```

/// Finds the minimum arborescence rooted at `root` over
/// `edges = (from, to, weight)`. Returns `(total weight, sorted input
/// edge indices)`. `None` when some vertex is unreachable from `root`
/// (every arborescence needs an in-edge to every non-root vertex).
/// Self-loops are ignored; `n < 2` yields `(0, [])` when `n == 1` and
/// `root == 0`, `None` when `n == 0` or `root >= n`.
pub fn directed_mst(n: usize, root: usize, edges: &[(u32, u32, u64)]) -> Option<(u64, Vec<u32>)> {
    if root >= n {
        return None;
    }
    if n == 1 {
        return Some((0, Vec::new()));
    }
    let es: Vec<E> = edges
        .iter()
        .enumerate()
        .filter(|(_, &(u, v, _))| (u as usize) < n && (v as usize) < n && u != v)
        .map(|(i, &(u, v, w))| E {
            u: u as usize,
            v: v as usize,
            w: w as i64,
            id: i as u32,
        })
        .collect();
    let (cost, ids) = solve(n, &es, root)?;
    Some((cost as u64, ids))
}

struct E {
    u: usize,
    v: usize,
    w: i64,
    id: u32,
}

/// Recursive Chu–Liu/Edmonds on the current (possibly contracted)
/// graph. Edge `id` always refers to the original input index.
fn solve(n: usize, edges: &[E], root: usize) -> Option<(i64, Vec<u32>)> {
    if n == 1 {
        return Some((0, Vec::new()));
    }
    // best_in[v] = index into `edges` of the min incoming edge of v.
    let mut best: Vec<Option<usize>> = vec![None; n];
    for (i, e) in edges.iter().enumerate() {
        if e.v == root {
            continue;
        }
        match best[e.v] {
            None => best[e.v] = Some(i),
            Some(j) => {
                let b = &edges[j];
                if (e.w, e.u, e.id) < (b.w, b.u, b.id) {
                    best[e.v] = Some(i);
                }
            }
        }
    }
    if best
        .iter()
        .enumerate()
        .any(|(v, b)| v != root && b.is_none())
    {
        return None; // some vertex has no incoming edge
    }
    // Find a cycle in the parent relation v -> best[v].u.
    let mut state = vec![0u8; n]; // 0 unseen, 1 on stack, 2 done
    let mut cycle = None;
    for start in 0..n {
        if start == root {
            continue;
        }
        let mut path = Vec::new();
        let mut v = start;
        while v != root && state[v] == 0 {
            state[v] = 1;
            path.push(v);
            v = edges[best[v]?].u;
        }
        if v != root && state[v] == 1 {
            let pos = path.iter().position(|&x| x == v)?;
            cycle = Some(path[pos..].to_vec());
            break;
        }
        for &x in &path {
            state[x] = 2;
        }
    }
    match cycle {
        None => {
            // The greedy pick is already a tree.
            let mut cost = 0i64;
            let mut ids = Vec::new();
            for v in 0..n {
                if v != root {
                    let e = &edges[best[v]?];
                    cost += e.w;
                    ids.push(e.id);
                }
            }
            ids.sort_unstable();
            Some((cost, ids))
        }
        Some(cyc) => {
            let in_cyc = {
                let mut m = vec![false; n];
                for &v in &cyc {
                    m[v] = true;
                }
                m
            };
            // Renumber: non-cycle nodes keep ascending order; the cycle
            // becomes the last node id.
            // Every slot is assigned below before `map` is read.
            let mut map = vec![0usize; n];
            let mut k = 0usize;
            for v in 0..n {
                if !in_cyc[v] {
                    map[v] = k;
                    k += 1;
                }
            }
            let cyc_node = k;
            for &v in &cyc {
                map[v] = cyc_node;
            }
            let nn = k + 1;
            let nroot = map[root];
            // Adjusted edge set: entering the cycle from outside costs
            // w - best_in[v] (the displaced in-edge).
            let mut nes: Vec<E> = Vec::new();
            for e in edges {
                let (mu, mv) = (map[e.u], map[e.v]);
                if mu == mv {
                    continue;
                }
                let w = if in_cyc[e.v] {
                    e.w - edges[best[e.v]?].w
                } else {
                    e.w
                };
                nes.push(E {
                    u: mu,
                    v: mv,
                    w,
                    id: e.id,
                });
            }
            let (sub_cost, mut ids) = solve(nn, &nes, nroot)?;
            // Expand: the contracted edge (if any) enters some member
            // v* — its best_in is displaced; every other member keeps
            // its own cheapest in-edge.
            let mut displaced = None;
            for &id in &ids {
                // Map the chosen contracted-graph edge back to the
                // ORIGINAL edge to learn which member it enters.
                let orig = edges.iter().find(|x| x.id == id);
                if let Some(o) = orig {
                    if in_cyc[o.v] {
                        displaced = Some(o.v);
                    }
                }
            }
            let mut total = sub_cost;
            for &v in &cyc {
                let e = &edges[best[v]?];
                total += e.w;
                if Some(v) != displaced {
                    ids.push(e.id);
                }
            }
            ids.sort_unstable();
            Some((total, ids))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: enumerate every size-(n-1) edge subset, keep the
    /// cheapest that gives every non-root exactly one incoming edge
    /// and a path to the root.
    fn oracle(n: usize, root: usize, edges: &[(u32, u32, u64)]) -> Option<u64> {
        let m = edges.len();
        let mut best = None;
        for mask in 0..(1u64 << m) {
            if mask.count_ones() as usize != n - 1 {
                continue;
            }
            let mut parent = vec![None; n];
            let mut cost = 0u64;
            for (i, &(u, v, w)) in edges.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    if (u as usize) >= n || (v as usize) >= n || u == v {
                        break;
                    }
                    if parent[v as usize].is_some() {
                        break; // two incoming edges — not an arborescence
                    }
                    parent[v as usize] = Some(u as usize);
                    cost += w;
                }
            }
            if parent
                .iter()
                .enumerate()
                .any(|(v, p)| (v != root && p.is_none()) || (v == root && p.is_some()))
            {
                continue;
            }
            // Reachability: every vertex walks to root.
            let ok = (0..n).all(|v| {
                let mut seen = vec![false; n];
                let mut x = v;
                while x != root {
                    if seen[x] {
                        return false;
                    }
                    seen[x] = true;
                    match parent[x] {
                        Some(p) => x = p,
                        None => return false,
                    }
                }
                true
            });
            if ok {
                best = Some(best.map_or(cost, |b: u64| b.min(cost)));
            }
        }
        best
    }

    /// The chosen edges must form a valid arborescence of the claimed
    /// cost — independent of the oracle's enumeration.
    fn check_witness(n: usize, root: usize, edges: &[(u32, u32, u64)], cost: u64, ids: &[u32]) {
        assert_eq!(ids.len(), n - 1);
        let mut parent = vec![None; n];
        let mut sum = 0u64;
        for &i in ids {
            let (u, v, w) = edges[i as usize];
            assert!((v as usize) != root);
            assert!(parent[v as usize].is_none());
            parent[v as usize] = Some(u as usize);
            sum += w;
        }
        assert_eq!(sum, cost);
        for v in 0..n {
            let mut seen = vec![false; n];
            let mut x = v;
            while x != root {
                assert!(!seen[x]);
                seen[x] = true;
                x = parent[x].expect("non-root must have a parent");
            }
        }
    }

    #[test]
    fn matches_brute_force_oracle() {
        let mut rng = SplitMix64::new(0xED21);
        for _ in 0..150 {
            let n = (rng.below(5) + 2) as usize;
            let root = rng.below(n as u32) as usize;
            let m = rng.below(10) as usize;
            let mut edges = Vec::new();
            for _ in 0..m {
                edges.push((
                    rng.below(n as u32),
                    rng.below(n as u32),
                    rng.below(30) as u64,
                ));
            }
            let got = directed_mst(n, root, &edges);
            let want = oracle(n, root, &edges);
            assert_eq!(
                got.as_ref().map(|g| g.0),
                want,
                "n={n} root={root} edges={edges:?}"
            );
            if let Some((cost, ids)) = &got {
                let (cost, ids) = (*cost, ids.clone());
                check_witness(n, root, &edges, cost, &ids);
            }
        }
    }

    #[test]
    fn contract_cycle_recovers_optimum() {
        // Classic example: the two-cycle forces contraction.
        // 0→1(5), 1→2(1), 2→1(1), 0→2(10). Optimum: 0→1, 1→2 (cost 6).
        let edges = [(0, 1, 5), (1, 2, 1), (2, 1, 1), (0, 2, 10)];
        let (cost, ids) = directed_mst(3, 0, &edges).unwrap();
        assert_eq!(cost, 6);
        check_witness(3, 0, &edges, cost, &ids);
    }

    #[test]
    fn edge_cases() {
        assert!(directed_mst(0, 0, &[]).is_none());
        assert_eq!(directed_mst(1, 0, &[]), Some((0, vec![])));
        assert!(directed_mst(2, 3, &[]).is_none());
        // Unreachable vertex.
        assert!(directed_mst(3, 0, &[(0, 1, 1)]).is_none());
        // Self-loop ignored.
        assert_eq!(
            directed_mst(2, 0, &[(0, 0, 9), (0, 1, 4)]),
            Some((4, vec![1]))
        );
    }
}
